use crate::{
    model::{Evidence, Finding},
    text::bounded,
};
use std::collections::BTreeMap;

const BROADCAST_LATENCY_THRESHOLD_MS: u64 = 3000;

// https://source.android.com/docs/core/tests/debug/understanding-logging:
// `BOOT_COMPLETED_BROADCAST_COMPLETION_LATENCY_REPORTED action:X ... completeLatency:Nms` reports
// how long a broadcast took every receiver to finish handling.
pub fn scan_broadcast_latency(source: &str, report: &str) -> Vec<Finding> {
    struct Broadcast<'a> {
        index: usize,
        line: &'a str,
        action: String,
        complete_latency_ms: u64,
    }
    let mut broadcasts: Vec<Broadcast> = report
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if !line.contains("BROADCAST_COMPLETION_LATENCY_REPORTED") {
                return None;
            }
            let action = line
                .split_once("action:")
                .and_then(|(_, rest)| rest.split(' ').next())
                .unwrap_or("unknown")
                .to_string();
            let complete_latency_ms = line
                .split_once("completeLatency:")
                .and_then(|(_, rest)| rest.split(' ').next())
                .and_then(|digits| digits.parse::<u64>().ok())
                .unwrap_or(0);
            Some(Broadcast {
                index,
                line,
                action,
                complete_latency_ms,
            })
        })
        .collect();
    if broadcasts.is_empty() {
        return Vec::new();
    }
    broadcasts.sort_by_key(|b| std::cmp::Reverse(b.complete_latency_ms));
    let max_latency = broadcasts
        .first()
        .map(|b| b.complete_latency_ms)
        .unwrap_or(0);
    if max_latency < BROADCAST_LATENCY_THRESHOLD_MS {
        return Vec::new();
    }
    let evidence = broadcasts
        .iter()
        .take(super::MAX_EVIDENCE_PER_FINDING)
        .map(|b| Evidence {
            source: source.into(),
            line_start: b.index + 1,
            line_end: b.index + 1,
            excerpt: format!(
                "{} ({}ms complete latency) — {}",
                b.action,
                b.complete_latency_ms,
                bounded(b.line)
            ),
        })
        .collect();
    let mut metadata = BTreeMap::new();
    metadata.insert("occurrences".into(), broadcasts.len().to_string());
    metadata.insert("max_complete_latency_ms".into(), max_latency.to_string());
    vec![Finding {
        schema_version: 1,
        id: "broadcast_latency".into(),
        severity: "medium".into(),
        title: "Broadcast dispatch backlog".into(),
        summary: format!(
            "{} broadcast completion(s) exceeded the expected latency; the slowest took {max_latency}ms for all receivers to finish.",
            broadcasts.len()
        ),
        evidence,
        metadata,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_broadcast_latency_above_threshold() {
        let report = "09-24 00:27:18.359 1000 683 862 I BroadcastQueue: BOOT_COMPLETED_BROADCAST_COMPLETION_LATENCY_REPORTED action:android.intent.action.BOOT_COMPLETED dispatchLatency:1427 completeLatency:13152 dispatchRealLatency:1427 completeRealLatency:13152 receiversSize:58 userId:10\n";
        let findings = scan_broadcast_latency("report.txt", report);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].metadata["max_complete_latency_ms"], "13152");
    }
}
