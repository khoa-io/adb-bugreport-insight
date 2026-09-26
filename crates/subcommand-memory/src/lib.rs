//! Memory-pressure scanning, owned by the `bugreport-insight memory` subcommand. Exposed as a
//! library so both the `bugreport-insight-memory` binary and other consumers can call
//! [`scan_memory_pressure`] directly.

use bugreport_insight_core::{
    model::{Evidence, Finding},
    scanners::find_all,
    text::bounded,
};
use std::collections::BTreeMap;

const MEMORY_CHURN_THRESHOLD: usize = 20;

// https://source.android.com/docs/core/tests/debug/understanding-logging: `am_low_memory` marks
// low-memory-killer activity, and bursts of `am_proc_start`/`am_proc_died` indicate process
// churn/thrashing rather than a single low-memory event.
pub fn scan_memory_pressure(source: &str, report: &str) -> Vec<Finding> {
    let low_memory = find_all(report, "am_low_memory");
    let proc_died = find_all(report, "am_proc_died");
    let proc_start = find_all(report, "am_proc_start");
    let churn = proc_died.len() + proc_start.len();
    if low_memory.is_empty() && churn < MEMORY_CHURN_THRESHOLD {
        return Vec::new();
    }
    let mut evidence: Vec<Evidence> = low_memory
        .iter()
        .take(5)
        .map(|(index, line)| Evidence {
            source: source.into(),
            line_start: index + 1,
            line_end: index + 1,
            excerpt: bounded(line),
        })
        .collect();
    evidence.extend(proc_died.iter().take(3).map(|(index, line)| Evidence {
        source: source.into(),
        line_start: index + 1,
        line_end: index + 1,
        excerpt: bounded(line),
    }));
    let mut metadata = BTreeMap::new();
    metadata.insert("am_low_memory_count".into(), low_memory.len().to_string());
    metadata.insert("am_proc_died_count".into(), proc_died.len().to_string());
    metadata.insert("am_proc_start_count".into(), proc_start.len().to_string());
    vec![Finding {
        schema_version: 1,
        id: "memory_pressure".into(),
        severity: "medium".into(),
        title: "Low-memory process churn".into(),
        summary: format!(
            "Process lifecycle churn ({churn} start/death events, {} low-memory signal(s)) suggests the system may have been thrashing to free RAM.",
            low_memory.len()
        ),
        evidence,
        metadata,
    }]
}
