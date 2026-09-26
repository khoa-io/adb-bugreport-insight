use super::{MAX_EVIDENCE_PER_FINDING, find_all};
use crate::{
    model::{Evidence, Finding},
    text::bounded,
};
use std::collections::BTreeMap;

pub fn scan_activity_timeline(source: &str, report: &str) -> Vec<Finding> {
    let matches = find_all(report, "am_focused_activity");
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
        id: "activity_timeline".into(),
        severity: "informational".into(),
        title: "Focused activity timeline".into(),
        summary: format!(
            "{} focused-activity transition(s) were recorded; use this timeline to correlate user-visible context with other findings.",
            matches.len()
        ),
        evidence,
        metadata,
    }]
}
