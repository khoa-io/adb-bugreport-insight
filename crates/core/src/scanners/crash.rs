use super::{MAX_EVIDENCE_PER_FINDING, find_all};
use crate::{
    model::{Evidence, Finding},
    text::bounded,
};
use std::collections::BTreeMap;

// https://source.android.com/docs/core/tests/debug/native-crash: `WATCHDOG KILLING SYSTEM
// PROCESS` is a runtime restart of system_server (back to the boot animation), distinct from a
// kernel panic/reboot (back to the boot logo).
pub fn scan_watchdog(source: &str, report: &str) -> Vec<Finding> {
    let mut matches = find_all(report, "WATCHDOG KILLING SYSTEM PROCESS");
    matches.extend(find_all(report, "system_server crash"));
    matches.sort_by_key(|(index, _)| *index);
    if matches.is_empty() {
        return Vec::new();
    }
    let evidence = matches
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
    metadata.insert("occurrences".into(), matches.len().to_string());
    vec![Finding {
        schema_version: 1,
        id: "watchdog".into(),
        severity: "high".into(),
        title: "System watchdog runtime restart".into(),
        summary: "The system_server watchdog fired, causing a runtime restart back to the boot animation — this is not a kernel crash/reboot back to the boot logo.".into(),
        evidence,
        metadata,
    }]
}
