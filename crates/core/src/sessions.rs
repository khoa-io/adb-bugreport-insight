use crate::{
    model::{Boundary, Evidence, Session, TimeRange},
    text::bounded,
};

pub fn sessions(source: &str, report: &str) -> Vec<Session> {
    let mut boundaries = Vec::new();
    for (index, line) in report.lines().enumerate() {
        if line.contains("log buffer reset")
            || line.contains("reboot: Restarting Android")
            || line.contains("ShutdownThread: Rebooting")
        {
            boundaries.push(Evidence {
                source: source.into(),
                line_start: index + 1,
                line_end: index + 1,
                excerpt: bounded(line),
            });
        }
    }
    let count = boundaries.len() + 1;
    (0..count)
        .map(|index| Session {
            schema_version: 1,
            session_index: index,
            time_range: TimeRange {
                start: None,
                end: None,
                duration_ms: None,
            },
            boundary: Boundary {
                start_confidence: if index == 0 { "unknown" } else { "medium" }.into(),
                end_reason: boundaries
                    .get(index)
                    .map(|_| "timestamp_regression_or_boot_marker".into()),
                evidence: boundaries
                    .get(index.saturating_sub(1))
                    .cloned()
                    .into_iter()
                    .collect(),
            },
            apps: Vec::new(),
        })
        .collect()
}
