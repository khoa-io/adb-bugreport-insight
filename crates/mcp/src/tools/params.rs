//! The seven parameter structs.
//!
//! Every one is `deny_unknown_fields`, so each tool's `inputSchema` carries
//! `additionalProperties: false`. That is the testable half of `REQ-MCP-05`: a
//! misspelled parameter is a protocol error, not a silently ignored field that
//! makes the tool do something other than what was asked.
//!
//! `schemars::JsonSchema` is derived here rather than in the adapter so that
//! the wire schema and the decode rules are the same type. `schemars` is not an
//! `rmcp` symbol — it is rmcp's own dependency, re-exported.

use bugreport_insight_core::PatternMode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `archive_info` — identity, optionally including SHA-256.
///
/// `with_sha256` defaults to true because the reference identity *is* the point
/// of the tool; hashing streams the stored ZIP and caches the first identity choice.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveInfoParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Single-use token from confirmation_required; omit to preview cost without executing.
    pub confirmation: Option<String>,
    /// Hash the whole stored ZIP when the shared digest choice is not yet cached.
    #[serde(default = "default_true")]
    pub with_sha256: bool,
}

/// `list_entries` — bounded listing, optionally under one path prefix.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListEntriesParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Enumerate one subtree: `ANR` and `ANR/` exclude `ANR2`. Empty selects all.
    /// Purely a display convenience:
    /// entries are still addressed by `entry_id` in the result.
    pub path_prefix: Option<String>,
    /// Defaults to 200 and is capped at 200.
    pub limit: Option<usize>,
}

/// `find_entries` — glob on the path, no content touched.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindEntriesParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Glob against the entry path. `*` does not cross `/`.
    pub pattern: String,
    /// `glob` (default) or `literal`. `regex` is a typed refusal —
    /// `DRAFT-REQ-MCP-ARC-08`, never a silent fallback to literal.
    #[serde(default)]
    pub mode: Option<String>,
    /// Defaults to 200 and is capped at 200.
    pub limit: Option<usize>,
}

/// `entry_info` — everything known about one entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntryInfoParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Single-use token from confirmation_required; omit to preview cost without executing.
    pub confirmation: Option<String>,
    /// Index id from `list_entries` or `find_entries`. Never a path.
    pub entry_id: u32,
}

/// `find_by_type` — filter by detected content type, archive-wide.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindByTypeParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Single-use token from confirmation_required; omit to preview cost without executing.
    pub confirmation: Option<String>,
    /// A `ContentType` name, e.g. `text`, `image_png`, `binary`, `unknown`.
    pub content_type: String,
    /// Defaults to 200 and is capped at 200.
    pub limit: Option<usize>,
}

/// `read_entry` — one bounded slice of one entry.
///
/// The two range pairs are mutually exclusive and **neither is optional**: a
/// call must name exactly one (`REQ-MCP-ARC-04`).
///
/// **These are four flat fields, not an enum, and that is load-bearing.** MCP
/// requires `inputSchema` to have root type `object`, and rmcp asserts it — an
/// externally-tagged enum schema has no root `type` and panics `tools/list`
/// rather than degrading. Flat fields keep the schema a legal object; the
/// mutual exclusion is then a decode-adjacent check that returns
/// `conflicting_range_params`, the variant `core` already provides.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadEntryParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Omit for a bound ZIP; if supplied there, it must match that ZIP's basename.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Single-use token from confirmation_required; omit to preview cost without executing.
    pub confirmation: Option<String>,
    /// Index id from `list_entries` or `find_entries`. Never a path.
    pub entry_id: u32,
    /// 1-based, inclusive; must not exceed line_end. Past EOF returns available lines.
    pub line_start: Option<u64>,
    /// 1-based, inclusive. With `line_start`, exclusive of `byte_*`.
    pub line_end: Option<u64>,
    /// Uncompressed offset of the first byte. With `byte_end`, exclusive of `line_*`.
    pub byte_start: Option<u64>,
    /// Exclusive end, at least byte_start. Past EOF returns available bytes.
    pub byte_end: Option<u64>,
}

impl ReadEntryParams {
    /// Resolves the four fields to exactly one range.
    ///
    /// Neither pair, or both, is a caller mistake rather than an archive
    /// failure, so it surfaces as a protocol error.
    pub fn range(&self) -> Result<RangeParams, String> {
        match (
            self.line_start,
            self.line_end,
            self.byte_start,
            self.byte_end,
        ) {
            (Some(line_start), Some(line_end), None, None) => Ok(RangeParams::LineRange {
                line_start,
                line_end,
            }),
            (None, None, Some(byte_start), Some(byte_end)) => Ok(RangeParams::ByteRange {
                byte_start,
                byte_end,
            }),
            (None, None, None, None) => Err(
                "supply either line_start/line_end or byte_start/byte_end, but not neither"
                    .to_string(),
            ),
            _ => Err(
                "supply either line_start/line_end or byte_start/byte_end, not both".to_string(),
            ),
        }
    }
}

/// The resolved range. Never partially specified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeParams {
    /// 1-based, inclusive.
    LineRange { line_start: u64, line_end: u64 },
    /// Uncompressed offsets; `byte_end` is exclusive.
    ByteRange { byte_start: u64, byte_end: u64 },
}

/// `grep_archive` — content search returning locators.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrepArchiveParams {
    /// Optional exact bugreport basename in directory mode; use a listed identifier when present.
    /// Include it on every continuation when multiple reports are available.
    /// Omit for a bound ZIP or when directory mode has exactly one report.
    #[serde(default)]
    pub bugreport: Option<String>,
    /// Single-use token from confirmation_required; omit to preview cost without executing.
    pub confirmation: Option<String>,
    /// Literal text or glob, per `mode`.
    pub pattern: Option<String>,
    /// `glob` (default) or `literal`; omitted on continuation.
    #[serde(default)]
    pub mode: Option<String>,
    /// Restrict the scan. Omitted or empty means the whole archive.
    pub entry_ids: Option<Vec<u32>>,
    /// Defaults to 50 and is capped at 50.
    pub max_matches: Option<usize>,
    /// Maximum serialized MCP tool-result bytes, including escaped text and cursor.
    /// Defaults to 65536; accepted range 2048..=262144.
    pub response_budget: Option<usize>,
    /// New decompressed source bytes per page; defaults/max 33554432, minimum 1.
    pub scan_budget: Option<u64>,
    /// Opaque, single-use continuation; omit pattern/mode/entry_ids when supplied.
    pub cursor: Option<String>,
}

/// `glob` (default) or `literal`. `regex` is refused, not downgraded.
pub fn parse_mode(raw: Option<&str>) -> Result<PatternMode, String> {
    match raw.unwrap_or("glob") {
        "glob" => Ok(PatternMode::Glob),
        "literal" => Ok(PatternMode::Literal),
        other => Err(format!(
            "mode `{other}` is unavailable: regex matching is \
             DRAFT-REQ-MCP-ARC-08; use `glob` or `literal`"
        )),
    }
}

fn default_true() -> bool {
    true
}
