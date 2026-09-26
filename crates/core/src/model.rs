use serde::Serialize;
use std::collections::BTreeMap;

/// A bounded, source-attributed excerpt supporting a `Finding` or session `Boundary`.
#[derive(Clone, Serialize)]
pub struct Evidence {
    pub source: String,
    pub line_start: usize,
    pub line_end: usize,
    pub excerpt: String,
}

/// A single scanner-emitted, schema-versioned observation. Findings are evidence, not proof of
/// root cause; `severity` is one of `high`, `medium`, or `informational`.
#[derive(Serialize)]
pub struct Finding {
    pub schema_version: u8,
    pub id: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<Evidence>,
    pub metadata: BTreeMap<String, String>,
}

/// A contiguous logcat capture window between two reboot boundaries (or archive bounds).
#[derive(Serialize)]
pub struct Session {
    pub schema_version: u8,
    pub session_index: usize,
    pub time_range: TimeRange,
    pub boundary: Boundary,
    pub apps: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Serialize)]
pub struct Boundary {
    pub start_confidence: String,
    pub end_reason: Option<String>,
    pub evidence: Vec<Evidence>,
}

/// A live or installed application, correlated from `dumpsys package`'s `Package [name]:`
/// blocks (installed path/data path) and `dumpsys activity`'s `PID mappings:` section (live
/// PID/UID), emitted by the `apps` scanner/subcommand.
#[derive(Serialize)]
pub struct AppInfo {
    pub schema_version: u8,
    pub package: String,
    pub pid: Option<u32>,
    pub uid: Option<String>,
    pub data_dir: Option<String>,
    pub code_path: Option<String>,
    pub evidence: Vec<Evidence>,
}

/// The top-level `insight/manifest.json` artifact describing the run and generated outputs.
#[derive(Serialize)]
pub struct Manifest {
    pub schema_version: u8,
    pub tool_version: String,
    pub input_archive: String,
    pub primary_source: String,
    pub artifacts: Vec<String>,
    pub logcat_sessions: usize,
    pub observed_reboot_boundaries: usize,
    pub severity_counts: BTreeMap<String, usize>,
}
