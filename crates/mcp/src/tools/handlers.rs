//! The seven archive tools, implemented against `bugreport_insight_core`.
//!
//! Listing and path filtering use the immutable index. Detection, identity,
//! reads and search also touch bytes through independent handles; the server
//! uses independent content readers; grep alone retains bounded cursor sessions
//! (`docs/swe.3-detailed-design/03-mcp-server.md`).
//!
//! Each handler's first description sentence states its cost class and bound,
//! which is the testable half of `REQ-MCP-05`.

use std::sync::Arc;

use bugreport_insight_core::{
    Archive, ArchiveError, ContentType, EntryId, EntryMeta, Excerpt, ExcerptOutcome, PatternMode,
    StoppedReason, caps,
};
use serde_json::{Value, json};

use crate::error::ToolError;
use crate::registry::ToolHandler;
use crate::tools::envelope::ToolEnvelope;
use crate::tools::params::{
    ArchiveInfoParams, EntryInfoParams, FindByTypeParams, FindEntriesParams, ListEntriesParams,
    RangeParams, ReadEntryParams, parse_mode,
};

/// The shared, immutable archive. `Archive` is `Send + Sync`, holds no lock,
/// and memoises the digest; grep separately owns bounded continuation state.
pub type Ctx = Arc<Archive>;

/// Generates the struct, `name`, and `description`; the body lives in a
/// same-named snake_case function so the macro never has to move a block.
macro_rules! handler {
    ($(#[$meta:meta])* $t:ident, $name:literal, $desc:literal, $params:ty, $run:ident) => {
        $(#[$meta])*
        pub struct $t {
            ctx: Ctx,
        }

        impl $t {
            pub const NAME: &'static str = $name;
            pub const DESCRIPTION: &'static str = $desc;

            pub fn new(ctx: Ctx) -> Self {
                Self { ctx }
            }
        }

        impl ToolHandler for $t {
            fn name(&self) -> &'static str {
                $name
            }

            fn description(&self) -> &'static str {
                $desc
            }

            fn call(&self, params: Value) -> Result<ToolEnvelope, ToolError> {
                let p: $params = parse(params)?;
                $run(&self.ctx, p)
            }
        }
    };
}

/// A decode failure is the caller's routing problem, not the archive's, so it is
/// a protocol error. `deny_unknown_fields` on every params struct is what makes
/// a misspelled field land here instead of being silently ignored.
fn parse<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, ToolError> {
    serde_json::from_value(params).map_err(|e| ToolError::protocol("invalid_params", e.to_string()))
}

fn map_err(e: ArchiveError) -> ToolError {
    ToolError::from_archive(&e)
}

fn entry_not_found(id: EntryId) -> ToolError {
    ToolError::from_archive(&ArchiveError::EntryNotFound(id))
}

/// The bounded projection of an entry, shared by every listing-shaped tool.
fn entry_summary(m: &EntryMeta) -> Value {
    json!({
        "entry_id": m.id.0,
        "path": m.path,
        "uncompressed_size": m.uncompressed_size,
        "compressed_size": m.compressed_size,
        "is_dir": m.is_dir,
        "encrypted": m.encrypted,
        "path_bytes_valid_utf8": m.path_bytes_valid_utf8,
    })
}

/// Applies `limit` and the 200-item cap, returning `(kept, omitted)`.
///
/// Capping is never silent: the caller turns `omitted > 0` into
/// `stopped_reason: item_cap`.
fn cap_items(items: Vec<Value>, limit: Option<usize>) -> (Vec<Value>, u64) {
    let limit = limit
        .unwrap_or(caps::MAX_RESULT_ITEMS)
        .min(caps::MAX_RESULT_ITEMS);
    let total = items.len() as u64;
    let kept: Vec<Value> = items.into_iter().take(limit).collect();
    let omitted = total.saturating_sub(kept.len() as u64);
    (kept, omitted)
}

/// Empty describes a completed query with zero matches, never a zero-sized
/// output caused by a cap or a query that could not inspect every target.
fn collection_result(data: Value, empty: bool, omitted: u64) -> ToolEnvelope {
    let env = if empty {
        ToolEnvelope::empty(data, "no matches")
    } else {
        ToolEnvelope::ok(data)
    };
    if omitted > 0 {
        env.with_cap(StoppedReason::ItemCap, omitted)
    } else {
        env
    }
}

pub(super) fn excerpt_value(e: &Excerpt) -> Value {
    json!({
        "text": e.text,
        "bytes": e.bytes,
        "source_bytes": e.source_bytes,
        "truncated": e.truncated,
        "complete": e.complete,
        "decoding": format!("{:?}", e.decoding),
    })
}

// ------------------------------------------------------------------ archive_info

handler!(
    /// Identity with validated marker metadata and an optional memoised digest.
    ArchiveInfo,
    "archive_info",
    "Potentially expensive: full stored-ZIP hash when requested, memoised digest choice, and two 256-byte markers. \
     Confirmation required before execution; markers are reread and output size does not bound IO.",
    ArchiveInfoParams,
    archive_info
);

fn archive_info(ctx: &Archive, p: ArchiveInfoParams) -> Result<ToolEnvelope, ToolError> {
    let identity = ctx.identity(p.with_sha256).map_err(map_err)?;
    let markers = [
        ("version.txt", &identity.markers.version),
        ("main_entry.txt", &identity.markers.main_entry),
    ];
    let oversized = markers
        .iter()
        .any(|(_, m)| m.status == bugreport_insight_core::MarkerStatus::Oversized);
    let notes: Vec<String> = markers
        .iter()
        .filter(|(_, m)| {
            !matches!(
                m.status,
                bugreport_insight_core::MarkerStatus::Valid
                    | bugreport_insight_core::MarkerStatus::Missing
            )
        })
        .map(|(name, m)| {
            format!(
                "{name}: marker {}; value unavailable, inspect entry_id {} with read_entry",
                m.status.code(),
                m.entry_id
                    .map(|id| id.0.to_string())
                    .unwrap_or_else(|| "unknown".into())
            )
        })
        .collect();
    let mut env = ToolEnvelope::ok(json!({ "archive": identity }));
    env.notes.extend(notes);
    Ok(if oversized {
        env.with_cap(StoppedReason::ByteBudget, 0)
    } else {
        env
    })
}

// ------------------------------------------------------------------ list_entries

handler!(
    /// Bounded listing, optionally under one path prefix.
    ListEntries,
    "list_entries",
    "Cheap: index only, no content read; 200 items max. Lists ZIP entries with sizes, optionally \
     under one subtree (`ANR` or `ANR/`, excluding `ANR2`), and reports what it omitted.",
    ListEntriesParams,
    list_entries
);

fn list_entries(ctx: &Archive, p: ListEntriesParams) -> Result<ToolEnvelope, ToolError> {
    let matched: Vec<Value> = ctx
        .entries()
        .iter()
        .filter(|e| match &p.path_prefix {
            Some(prefix) => {
                let root = prefix.trim_end_matches('/');
                root.is_empty()
                    || e.path == root
                    || e.path
                        .strip_prefix(root)
                        .is_some_and(|rest| rest.starts_with('/'))
            }
            None => true,
        })
        .map(entry_summary)
        .collect();
    let (kept, omitted) = cap_items(matched, p.limit);
    let empty = kept.is_empty() && omitted == 0;
    Ok(collection_result(
        json!({
            "entries": kept,
            "entry_count": ctx.entry_count(),
        }),
        empty,
        omitted,
    ))
}

// ------------------------------------------------------------------ find_entries

handler!(
    /// Path-glob filter. No content read.
    FindEntries,
    "find_entries",
    "Cheap: index only, no content read; 200 items max. Entries whose path matches a glob (or a \
     literal). Cheaper than `find_by_type`, which samples bytes.",
    FindEntriesParams,
    find_entries
);

fn find_entries(ctx: &Archive, p: FindEntriesParams) -> Result<ToolEnvelope, ToolError> {
    let mode = parse_mode(p.mode.as_deref())
        .map_err(|m| ToolError::protocol("pattern_mode_unavailable", m))?;
    let ids: Vec<EntryId> = match mode {
        PatternMode::Literal => ctx
            .entries()
            .iter()
            .filter(|e| e.path == p.pattern)
            .map(|e| e.id)
            .collect(),
        _ => ctx.find_by_glob(&p.pattern).map_err(map_err)?,
    };
    let matched: Vec<Value> = ids
        .iter()
        .filter_map(|id| ctx.entry(*id))
        .map(entry_summary)
        .collect();
    let (kept, omitted) = cap_items(matched, p.limit);
    let empty = kept.is_empty() && omitted == 0;
    Ok(collection_result(
        json!({ "entries": kept }),
        empty,
        omitted,
    ))
}

// ------------------------------------------------------------------- entry_info

handler!(
    /// Everything known about one entry, addressed by `entry_id`.
    EntryInfo,
    "entry_info",
    "Confirmation required: index plus one 8 KiB detect sample. Authoritative compressed and uncompressed size, \
     compression method, mtime, and content type reported with its basis. Addressed by \
     `entry_id`, never by path.",
    EntryInfoParams,
    entry_info
);

fn entry_info(ctx: &Archive, p: EntryInfoParams) -> Result<ToolEnvelope, ToolError> {
    let id = EntryId(p.entry_id);
    let meta = ctx.entry(id).ok_or_else(|| entry_not_found(id))?;
    let detection = ctx.detect(id).map_err(map_err)?;
    Ok(ToolEnvelope::ok(json!({
        "entry": meta,
        "detection": detection,
    })))
}

// ------------------------------------------------------------------ find_by_type

handler!(
    /// Archive-wide content-type filter. Samples 8 KiB per entry.
    FindByType,
    "find_by_type",
    "Confirmation required: an 8 KiB detect sample per file; 200 matches and 200 failures max. Entries of one \
     detected content type, with classification basis, failures and sample coverage.",
    FindByTypeParams,
    find_by_type
);

fn find_by_type(ctx: &Archive, p: FindByTypeParams) -> Result<ToolEnvelope, ToolError> {
    let want: ContentType = serde_json::from_value(json!(p.content_type)).map_err(|_| {
        ToolError::protocol(
            "invalid_params",
            format!("unknown content_type `{}`", p.content_type),
        )
    })?;
    let limit = p
        .limit
        .unwrap_or(caps::MAX_RESULT_ITEMS)
        .min(caps::MAX_RESULT_ITEMS);
    let mut matched = Vec::new();
    let mut failures = Vec::new();
    let mut matched_count = 0u64;
    let mut failed_entries = 0u64;
    let mut classified_entries = 0u64;
    let mut skipped_directories = 0u64;
    for meta in ctx.entries() {
        // Directories have no file content to classify. Read failures belong
        // to the response, rather than silently becoming a non-match/Unknown.
        if meta.is_dir {
            skipped_directories += 1;
            continue;
        }
        match ctx.detect(meta.id) {
            Ok(detection) => {
                classified_entries += 1;
                if detection.content_type == want {
                    matched_count += 1;
                    if matched.len() < limit {
                        matched.push(json!({
                            "entry": entry_summary(meta), "detection": detection,
                        }));
                    }
                }
            }
            Err(error) => {
                failed_entries += 1;
                if failures.len() < caps::MAX_RESULT_ITEMS {
                    failures.push(json!({
                        "entry_id": meta.id.0,
                        "path": meta.path,
                        "error": map_err(error).to_json()["error"],
                    }));
                }
            }
        }
    }
    let omitted_matches = matched_count - matched.len() as u64;
    let omitted_failures = failed_entries - failures.len() as u64;
    let coverage_complete = failed_entries == 0;
    let data = json!({
        "matches": matched,
        "failures": failures,
        "omitted_matches": omitted_matches,
        "omitted_failures": omitted_failures,
        "coverage": {
            "entries_in_scope": classified_entries + failed_entries,
            "classified_entries": classified_entries,
            "failed_entries": failed_entries,
            "skipped_directories": skipped_directories,
            "coverage_complete": coverage_complete,
        },
    });
    let mut env = collection_result(
        data,
        matched_count == 0 && coverage_complete,
        omitted_matches + omitted_failures,
    );
    if !coverage_complete {
        if env.stopped_reason.is_none() {
            env = env.with_cap(StoppedReason::SkippedContent, 0);
        }
        env = env.with_note(format!(
            "{failed_entries} file classifications failed; coverage concerns detection samples only, not full-file validation; inspect failures by entry_id"
        ));
    }
    Ok(env)
}

// ------------------------------------------------------------------- read_entry

handler!(
    /// One bounded slice: a line range or a byte range, never both.
    ReadEntry,
    "read_entry",
    "Confirmation required: reads bytes including skipped prefix; 512-byte output excerpt cap. Takes either \
     `line_start`/`line_end` or `byte_start`/`byte_end`, never both. Never returns a whole entry, \
     and truncation is always flagged.",
    ReadEntryParams,
    read_entry
);

fn read_entry(ctx: &Archive, p: ReadEntryParams) -> Result<ToolEnvelope, ToolError> {
    let id = EntryId(p.entry_id);
    // Neither pair, or both, is a caller mistake — a protocol error, not a
    // domain failure. Resolved before any bytes are touched.
    let range = p
        .range()
        .map_err(|m| ToolError::protocol("conflicting_range_params", m))?;
    match range {
        RangeParams::LineRange {
            line_start,
            line_end,
        } if line_start == 0 || line_end < line_start => {
            return Err(ToolError::protocol(
                "invalid_range",
                "lines require 1 <= start <= end",
            ));
        }
        RangeParams::ByteRange {
            byte_start,
            byte_end,
        } if byte_end < byte_start => {
            return Err(ToolError::protocol(
                "invalid_range",
                "bytes require start <= end",
            ));
        }
        _ => {}
    }
    // Fresh handle per call — no shared mutable state, no lock.
    let mut reader = ctx.reader(id).map_err(map_err)?;
    match range {
        RangeParams::LineRange {
            line_start,
            line_end,
        } => {
            let (slices, past_end) = reader
                .read_line_span_partial(line_start, line_end)
                .map_err(map_err)?;
            let any_too_long = slices
                .iter()
                .any(|s| matches!(s.outcome, ExcerptOutcome::LineTooLong { .. }));
            let mut any_excerpt_cut = false;
            let lines: Vec<Value> = slices
                .iter()
                .map(|s| {
                    let excerpt = s.to_excerpt();
                    any_excerpt_cut |= excerpt.truncated;
                    json!({
                        "line": s.line_no,
                        "byte_offset": s.byte_offset,
                        "byte_len_full": s.byte_len_full,
                        "excerpt": excerpt_value(&excerpt),
                    })
                })
                .collect();
            let env = ToolEnvelope::ok(json!({
                "entry_id": id.0,
                "range": { "line_start": line_start, "line_end": line_end },
                "lines": lines,
                "past_end": past_end,
            }));
            Ok(if past_end {
                env.with_cap(StoppedReason::EndOfEntry, 0)
            } else if any_too_long {
                // `omitted_count` is 0 here: the line was reported, only its tail
                // was withheld, and `byte_len_full` says by how much.
                env.with_cap(StoppedReason::LineTooLong, 0)
            } else if any_excerpt_cut {
                env.with_cap(StoppedReason::ByteBudget, 0)
            } else {
                env
            })
        }
        RangeParams::ByteRange {
            byte_start,
            byte_end,
        } => {
            let len = byte_end - byte_start;
            let (bytes, past_end) = reader
                .read_byte_range_partial(byte_start, len)
                .map_err(map_err)?;
            let excerpt = Excerpt::from_bytes(&bytes, caps::MAX_EXCERPT_BYTES);
            let env = ToolEnvelope::ok(json!({
                "entry_id": id.0,
                "range": { "byte_start": byte_start, "byte_end": byte_end },
                "excerpt": excerpt_value(&excerpt),
                "bytes_returned": bytes.len(),
                "past_end": past_end,
            }));
            Ok(if past_end {
                env.with_cap(StoppedReason::EndOfEntry, 0)
            } else if excerpt.truncated {
                env.with_cap(StoppedReason::ByteBudget, 0)
            } else {
                env
            })
        }
    }
}

// ------------------------------------------------------------------ grep_archive

pub use super::pagination::GrepArchive;
