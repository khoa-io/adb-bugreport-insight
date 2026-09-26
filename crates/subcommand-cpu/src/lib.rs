//! CPU/I-O pressure and lock/monitor-contention scanning, owned by the `bugreport-insight cpu`
//! subcommand. Exposed as a library so both the `bugreport-insight-cpu` binary and other
//! consumers can call [`scan_cpu_pressure`] and [`scan_monitor_contention`] directly.

use bugreport_insight_core::{
    model::{Evidence, Finding},
    scanners::{MAX_EVIDENCE_PER_FINDING, find_all},
    text::bounded,
};
use std::collections::BTreeMap;

const CONTENTION_DURATION_THRESHOLD_MS: u64 = 300;

// https://source.android.com/docs/core/tests/debug/understanding-logging: heavy `kswapd`,
// `kworker`, or `mmcqd` activity in the CPU-info snapshot indicates memory/I-O thrashing; a
// `waiting to lock <addr> ... held by thread N` pair in VM traces indicates lock contention that
// may be an emerging deadlock.
pub fn scan_cpu_pressure(source: &str, report: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut thrashing: Vec<(usize, &str)> = Vec::new();
    for needle in ["kswapd", "kworker", "mmcqd"] {
        thrashing.extend(find_all(report, needle));
    }
    thrashing.sort_by_key(|(index, _)| *index);
    thrashing.dedup_by_key(|(index, _)| *index);
    if !thrashing.is_empty() {
        let evidence = thrashing
            .iter()
            .take(MAX_EVIDENCE_PER_FINDING)
            .map(|(index, line)| Evidence {
                source: source.into(),
                line_start: index + 1,
                line_end: index + 1,
                excerpt: bounded(line),
            })
            .collect();
        let mut metadata = BTreeMap::new();
        metadata.insert("occurrences".into(), thrashing.len().to_string());
        findings.push(Finding {
            schema_version: 1,
            id: "cpu_io_pressure".into(),
            severity: "medium".into(),
            title: "Memory/I-O thrashing indicators".into(),
            summary: format!(
                "{} kswapd/kworker/mmcqd CPU-snapshot row(s) were recorded; sustained activity here indicates the kernel was reclaiming memory or servicing storage I/O under pressure.",
                thrashing.len()
            ),
            evidence,
            metadata,
        });
    }

    let lock_waits = find_all(report, "held by thread");
    if !lock_waits.is_empty() {
        let evidence = lock_waits
            .iter()
            .take(MAX_EVIDENCE_PER_FINDING)
            .map(|(index, line)| Evidence {
                source: source.into(),
                line_start: index + 1,
                line_end: index + 1,
                excerpt: bounded(line),
            })
            .collect();
        let mut metadata = BTreeMap::new();
        metadata.insert("occurrences".into(), lock_waits.len().to_string());
        findings.push(Finding {
            schema_version: 1,
            id: "lock_contention".into(),
            severity: "medium".into(),
            title: "Thread waiting on a held lock".into(),
            summary: "A thread dump shows a thread waiting on a monitor held by another thread; if two threads hold locks the other is waiting on, this is a deadlock — cross-check the referenced thread IDs in the VM traces.".into(),
            evidence,
            metadata,
        });
    }
    findings
}

// https://source.android.com/docs/core/tests/debug/understanding-logging: `Long monitor
// contention with owner X ... for Nms` names the lock owner and wait duration.
pub fn scan_monitor_contention(source: &str, report: &str) -> Vec<Finding> {
    struct Contention<'a> {
        index: usize,
        line: &'a str,
        owner: String,
        duration_ms: u64,
    }
    let mut contentions: Vec<Contention> = report
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if !line.contains("Long monitor contention") {
                return None;
            }
            let owner = line
                .split_once("with owner ")
                .and_then(|(_, rest)| rest.split_once(" ("))
                .map(|(owner, _)| owner)
                .unwrap_or("unknown")
                .to_string();
            let duration_ms = line
                .rsplit_once("for ")
                .and_then(|(_, tail)| tail.strip_suffix("ms"))
                .and_then(|digits| digits.trim().parse::<u64>().ok())
                .unwrap_or(0);
            Some(Contention {
                index,
                line,
                owner,
                duration_ms,
            })
        })
        .collect();
    if contentions.is_empty() {
        return Vec::new();
    }
    let total = contentions.len();
    contentions.sort_by_key(|c| std::cmp::Reverse(c.duration_ms));
    let max_duration = contentions.first().map(|c| c.duration_ms).unwrap_or(0);
    if max_duration < CONTENTION_DURATION_THRESHOLD_MS {
        return Vec::new();
    }
    let evidence = contentions
        .iter()
        .take(MAX_EVIDENCE_PER_FINDING)
        .map(|c| Evidence {
            source: source.into(),
            line_start: c.index + 1,
            line_end: c.index + 1,
            excerpt: format!("{} ({}ms) — {}", c.owner, c.duration_ms, bounded(c.line)),
        })
        .collect();
    let mut metadata = BTreeMap::new();
    metadata.insert("occurrences".into(), total.to_string());
    metadata.insert("max_duration_ms".into(), max_duration.to_string());
    vec![Finding {
        schema_version: 1,
        id: "monitor_contention".into(),
        severity: "medium".into(),
        title: "Long monitor contention".into(),
        summary: format!(
            "{total} monitor-contention event(s) were recorded; the longest held for {max_duration}ms. Repeated long waits on the same lock can indicate an emerging deadlock."
        ),
        evidence,
        metadata,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_monitor_contention_above_threshold() {
        let report = "09-24 00:27:01.751 1000 683 683 W system_server: Long monitor contention with owner Foo (10) at bar()(x:1) waiters=0 in bar() for 640ms\n";
        let findings = scan_monitor_contention("report.txt", report);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].metadata["max_duration_ms"], "640");
    }

    #[test]
    fn skips_monitor_contention_below_threshold() {
        let report = "09-24 00:27:01.751 1000 683 683 W system_server: Long monitor contention with owner Foo (10) at bar()(x:1) waiters=0 in bar() for 50ms\n";
        assert!(scan_monitor_contention("report.txt", report).is_empty());
    }
}
