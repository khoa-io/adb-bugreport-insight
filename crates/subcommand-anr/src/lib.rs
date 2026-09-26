//! ANR (Application Not Responding) scanning, owned by the `bugreport-insight anr` subcommand.
//! Exposed as a library so both the `bugreport-insight-anr` binary and other consumers (the
//! `bugreport-insight` CLI's default full report, or a third-party subcommand) can call
//! [`scan_anrs`] directly instead of re-implementing it.

use bugreport_insight_core::{
    model::{Evidence, Finding},
    scanners::{MAX_EVIDENCE_PER_FINDING, lookahead, unique_sample},
    text::bounded,
};
use std::collections::BTreeMap;

// https://source.android.com/docs/core/tests/debug/read-bug-reports: an ANR is reported as
// `ANR in <package>` followed shortly by a `Reason:` line and a CPU-usage snapshot.
pub fn scan_anrs(source: &str, report: &str) -> Vec<Finding> {
    let lines: Vec<&str> = report.lines().collect();
    let mut evidence = Vec::new();
    let mut packages = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((_, rest)) = line.split_once("ANR in ") else {
            continue;
        };
        let package = rest.split_whitespace().next().unwrap_or(rest).to_string();
        let reason = lookahead(&lines, index, 4)
            .iter()
            .find_map(|l| l.split_once("Reason: ").map(|(_, r)| r));
        if evidence.len() < MAX_EVIDENCE_PER_FINDING {
            let excerpt = match reason {
                Some(reason) => format!("{} — Reason: {}", bounded(line), bounded(reason)),
                None => bounded(line),
            };
            evidence.push(Evidence {
                source: source.into(),
                line_start: index + 1,
                line_end: index + 1,
                excerpt,
            });
        }
        packages.push(package);
    }
    if evidence.is_empty() {
        return Vec::new();
    }
    let mut metadata = BTreeMap::new();
    metadata.insert("occurrences".into(), packages.len().to_string());
    metadata.insert("packages".into(), unique_sample(&packages, 8));
    vec![Finding {
        schema_version: 1,
        id: "anr".into(),
        severity: "high".into(),
        title: "Application Not Responding".into(),
        summary: format!(
            "{} ANR event(s) were recorded; correlate the PID and timestamp with the matching VM traces and CPU-usage snapshot.",
            packages.len()
        ),
        evidence,
        metadata,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_anrs_with_reason() {
        let report = "09-24 00:27:01.751 1000 683 683 E ActivityManager: ANR in com.example.app\n\
                       09-24 00:27:01.751 1000 683 683 E ActivityManager: Reason: Input dispatching timed out\n";
        let findings = scan_anrs("report.txt", report);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].metadata["packages"], "com.example.app");
        assert!(
            findings[0].evidence[0]
                .excerpt
                .contains("Input dispatching timed out")
        );
    }
}
