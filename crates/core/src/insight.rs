//! Evidence-based analysis and staged artifact generation for UC-03.
mod apps;
mod common;
mod incidents;
mod output;
mod render;
mod report;
mod resources;
mod system;
mod validate;
pub use output::{generate_to, generate_to_with_limits};

use crate::{Archive, ArchiveError};
use chrono::Local;
use common::*;
use report::Report;
use serde_json::{Map, Value, json};

/// A failed analysis never publishes a partial result.
#[derive(Debug, thiserror::Error)]
pub enum InsightError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error("cannot serialize insight artifact: {0}")]
    Serialize(#[from] serde_json::Error),
    /// A retained analysis budget that the explicit force profile can raise.
    #[error("insight analysis exceeds {resource} limit {limit}; no artifacts published")]
    LimitExceeded {
        resource: &'static str,
        limit: usize,
    },
    #[error("invalid insight data: {0}")]
    Invalid(String),
    #[error("insight output I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Published crash-loop thresholds; callers may supply different positive bounds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrashThresholds {
    pub repeat_count: usize,
    pub rate_per_second: f64,
    pub window_seconds: f64,
    /// None uses the report's printed min_crash_interval, then 120000 ms.
    pub inter_gap_max_ms: Option<i64>,
    pub escalation: usize,
}
impl Default for CrashThresholds {
    fn default() -> Self {
        Self {
            repeat_count: 5,
            rate_per_second: 1.0,
            window_seconds: 300.0,
            inter_gap_max_ms: None,
            escalation: 1,
        }
    }
}
/// Analysis parameters shared by reusable core consumers and the CLI defaults.
#[derive(Debug, Clone, Default)]
pub struct InsightOptions {
    pub crash_thresholds: CrashThresholds,
}

/// One of the fixed seven generated files; names never originate in ZIP paths.
#[derive(Debug, Clone)]
pub struct InsightFile {
    pub name: &'static str,
    pub contents: Vec<u8>,
}

#[derive(Debug)]
struct FindingInput {
    kind: &'static str,
    severity: &'static str,
    confidence: &'static str,
    process_key: Option<String>,
    attributes: Value,
    metrics: Value,
    evidence: Value,
    provenance: Value,
}

#[derive(Debug)]
struct Artifact {
    name: &'static str,
    data: Map<String, Value>,
    findings: Vec<Value>,
    counts: Map<String, Value>,
    gaps: Vec<Value>,
    negative: Vec<Value>,
    notes: Vec<String>,
    scope: Value,
}
impl Artifact {
    fn new(name: &'static str, r: &Report, sections: &[&str], all: bool) -> Self {
        let scope = r.scope(sections, all);
        let mut gaps = Vec::new();
        if let Some(missing) = scope["sections_not_found"].as_array() {
            for title in missing {
                gaps.push(gap(
                    "scope.sections_not_found",
                    "SECTION_ABSENT",
                    title.as_str().unwrap_or(""),
                ));
            }
        }
        let mut notes = Vec::new();
        if scope["sections_not_found"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            notes.push("SECTIONS_NOT_FOUND: Expected source sections are absent; absence does not establish a healthy device.".into());
        }
        if r.identity.format_version.is_none() {
            notes.push("ARCHIVE_FORMAT_0: version.txt is absent.".into());
        }
        if r.main.is_none() {
            gaps.push(gap(
                "source.main_entry",
                "SECTION_ABSENT",
                "No main bugreport text entry was identified.",
            ));
            notes.push(
                "NO_MAIN_ENTRY: No main report text was available for domain searches.".into(),
            );
        }
        if r.capture_time.is_none() {
            gaps.push(gap(
                "capture.captured_at",
                "PARSE_INDETERMINATE",
                "No capture timestamp with a supported explicit UTC offset can be reconstructed.",
            ));
            notes.push("LOGCAT_YEAR_UNRESOLVED: A year and UTC offset cannot be assigned to year-less log lines.".into());
        }
        if r.lines.iter().any(|l| l.text.contains('\0')) {
            notes.push("NUL_BYTES_IN_EVIDENCE: Source text contains NUL; affected excerpts are incomplete.".into());
        }
        if r.logs
            .iter()
            .any(|l| l.time.zip(r.capture_time).is_some_and(|(t, c)| t > c))
        {
            notes.push("COLLECTED_AFTER_START: Some evidence was collected after the dumpstate banner; its age is clamped to zero.".into());
        }
        Self {
            name,
            data: Map::new(),
            findings: Vec::new(),
            counts: Map::new(),
            gaps,
            negative: Vec::new(),
            notes,
            scope,
        }
    }
    fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.into(), value);
    }
    fn finding(&mut self, input: FindingInput) {
        let FindingInput {
            kind,
            severity,
            confidence,
            process_key,
            attributes,
            metrics,
            evidence,
            provenance,
        } = input;
        let capped = matches!(provenance["freshness"].as_str(), Some("stale" | "undated"))
            && matches!(severity, "high" | "critical");
        self.findings.push(json!({"id":"","kind":kind,"severity":if capped{"medium"}else{severity},"unconstrained_severity":severity,"confidence":confidence,"process_key":process_key,"event_time":provenance["captured_at"],"attributes":attributes,"metrics":metrics,"evidence":evidence,"corroborating_evidence":[],"provenance":provenance}));
    }
    fn count(&mut self, r: &Report, key: &str, pattern: &str, validated: usize) {
        self.counts.insert(key.into(), r.count(pattern, validated));
    }
    fn absence(&mut self, r: &Report, key: &str, pattern: &str) {
        self.count(r, key, pattern, 0);
        if r.main.is_some() {
            self.negative.push(json!({"scope":format!("{}: main-entry search",self.name),"statement":format!("No structurally validated {pattern} record in the scanned main entry"),"source":null,"authority":"archive_absence"}));
        }
    }
    fn finish(
        mut self,
        r: &Report,
        source: &Value,
        generator: &Value,
        capture: &Value,
        collection: &Value,
    ) -> Value {
        for (key, count) in &self.counts {
            if count["matched_evidence"] == 0 && r.main.is_some() {
                let provenance = r.provenance(
                    usize::MAX,
                    "archive_search",
                    "command_output",
                    None,
                    "undetermined",
                    "none",
                );
                self.findings.push(json!({"id":"","kind":"signal_absence","severity":"info","unconstrained_severity":"info","confidence":"reported","process_key":null,"event_time":null,"attributes":{"search_key":key,"pattern":count["pattern"]},"metrics":{"raw_hits":count["raw_count"],"validated_records":0},"evidence":{"archive_path":r.path,"kind":"record","locator":{"line_start":null,"line_end":null,"byte_offset":null,"byte_length":null},"excerpt":"","excerpt_bytes":0,"excerpt_truncated":false,"excerpt_complete":false},"corroborating_evidence":[],"provenance":provenance}));
            }
        }
        let mut missing = std::collections::BTreeSet::new();
        null_fields(&source["device"], "source.device", &mut missing);
        if self.name == "system" {
            null_fields(capture, "capture", &mut missing);
        }
        for (key, value) in &self.data {
            if key != "fact_evidence" {
                null_fields(value, key, &mut missing);
            }
        }
        for field in missing {
            if !self
                .gaps
                .iter()
                .any(|g| g["field"].as_str() == Some(field.as_str()))
            {
                self.gaps.push(gap(&field,"NOT_PRINTED","No value is available in the consumed source records; null is not zero or false."));
            }
        }
        self.findings.sort_by_key(|v| {
            (
                v["evidence"]["entry_id"]
                    .as_u64()
                    .unwrap_or(r.main.map(|id| id.0 as u64).unwrap_or(0)),
                v["provenance"]["line_start"].as_u64().unwrap_or(0),
            )
        });
        let prefix = match self.name {
            "system" => "sys",
            "apps" => "app",
            "anr" => "anr",
            "crashes" => "crash",
            "memory" => "mem",
            _ => "cpu",
        };
        let mut severities = integer_map("critical high medium low info");
        let mut freshness = integer_map("now in_window stale undated");
        for (i, f) in self.findings.iter_mut().enumerate() {
            f["id"] = json!(format!("{prefix}-{:04}", i + 1));
            let s = f["severity"].as_str().unwrap_or("info").to_string();
            let t = f["provenance"]["freshness"]
                .as_str()
                .unwrap_or("undated")
                .to_string();
            severities[&s] = json!(severities[&s].as_u64().unwrap_or(0) + 1);
            freshness[&t] = json!(freshness[&t].as_u64().unwrap_or(0) + 1);
        }
        if self.findings.is_empty() {
            self.notes.push("NO_FINDINGS: No validated findings were produced from the available sources; consult scope and evidence_gaps.".into());
        }
        let mut v = Value::Object(self.data);
        v["schema_version"] = json!(1);
        v["artifact"] = json!(self.name);
        v["generator"] = generator.clone();
        v["source"] = source.clone();
        v["scope"] = self.scope;
        let extra_collection = v.get("collection").cloned();
        v["collection"] = collection.clone();
        if let Some(extra) = extra_collection {
            merge(&mut v["collection"], extra);
        }
        if self.name == "system" {
            v["capture"] = capture.clone();
        } else {
            v["capture_ref"] = json!({"source":"system.json#/capture","boot_epoch":capture["boot"]["at"],"uptime_seconds_at_start":capture["uptime_at_capture_seconds"],"log_window_ref":"system.json#/capture/log_window"});
        }
        v["counts"] = Value::Object(self.counts);
        v["severity_summary"] = severities;
        v["freshness_summary"] = freshness;
        v["negative_evidence"] = json!(self.negative);
        v["evidence_gaps"] = json!(self.gaps);
        v["findings"] = json!(self.findings);
        v["notes"] = json!(self.notes);
        v
    }
}

fn null_fields(value: &Value, path: &str, out: &mut std::collections::BTreeSet<String>) {
    match value {
        Value::Null => {
            out.insert(path.into());
        }
        Value::Object(map) => {
            for (key, value) in map {
                if !["provenance", "evidence", "corroborating_evidence"].contains(&key.as_str()) {
                    null_fields(value, &format!("{path}.{key}"), out);
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                null_fields(value, &format!("{path}[]"), out);
            }
        }
        _ => {}
    }
}

/// Analyze one ZIP through bounded core readers and produce the complete UC-03 set.
///
/// Missing channels and unresolvable clocks remain explicit. Read/limit/schema
/// failures return an error; this function performs no filesystem writes.
pub fn generate_insight(archive: &Archive) -> Result<Vec<InsightFile>, InsightError> {
    generate_insight_with_options(archive, &InsightOptions::default())
}

/// Generate with validated, published crash thresholds; no filesystem writes.
pub fn generate_insight_with_options(
    archive: &Archive,
    options: &InsightOptions,
) -> Result<Vec<InsightFile>, InsightError> {
    let t = &options.crash_thresholds;
    if t.repeat_count < 2
        || !t.rate_per_second.is_finite()
        || t.rate_per_second <= 0.0
        || !t.window_seconds.is_finite()
        || t.window_seconds <= 0.0
        || t.inter_gap_max_ms.is_some_and(|v| v <= 0)
        || t.escalation == 0
    {
        return Err(InsightError::Invalid(
            "crash thresholds require positive finite bounds and repeat_count >= 2".into(),
        ));
    }
    let started = Local::now().fixed_offset();
    let report = Report::load(archive)?;
    let (mut system, capture, device) = system::analyze(archive, &report)?;
    let apps = apps::analyze(&report, &capture);
    let anr = incidents::analyze_anr(archive, &report, &capture)?;
    let crashes =
        incidents::analyze_crashes(archive, &report, &capture, &apps, &options.crash_thresholds)?;
    let native_records = crashes
        .data
        .get("native_crash_incidents")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    system.count(&report, "signal_hits", "signal ", native_records);
    let (memory, cpu, collection) = resources::analyze(&report, &capture, &apps);
    let finished = Local::now().fixed_offset();
    let generator = json!({"name":"bugreport-insight","version":env!("CARGO_PKG_VERSION"),"run_started_at":stamp(&started),"run_finished_at":stamp(&finished),"host_timezone_offset":started.format("%:z").to_string()});
    let source = report.source(device);
    let artifacts: Vec<Value> = [system, apps, anr, crashes, memory, cpu]
        .into_iter()
        .map(|a| a.finish(&report, &source, &generator, &capture, &collection))
        .collect();
    validate::artifacts(&artifacts)?;
    let readme = render::readme(&artifacts);
    if readme.lines().count() > 400 {
        return Err(InsightError::Invalid("README exceeded 400 lines".into()));
    }
    let mut files = vec![InsightFile {
        name: "README.md",
        contents: readme.into_bytes(),
    }];
    for (name, value) in [
        "system.json",
        "apps.json",
        "anr.json",
        "crashes.json",
        "memory.json",
        "cpu.json",
    ]
    .into_iter()
    .zip(artifacts)
    {
        let mut contents = serde_json::to_vec_pretty(&value)?;
        contents.push(b'\n');
        files.push(InsightFile { name, contents });
    }
    Ok(files)
}

#[cfg(test)]
mod tests;
