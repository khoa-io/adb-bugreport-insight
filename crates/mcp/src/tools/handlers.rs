//! The seven archive tools, implemented against `bugreport_insight_core`.
//!
//! Five of the seven are pure functions over the immutable index and touch no
//! handle and no lock, ever. The two that read bytes open a fresh reader per
//! call — the server holds no `Mutex` and no `RwLock`
//! (`design/05-mcp-server-design.md` §3).
//!
//! Each handler's first description sentence states its cost class and bound,
//! which is the testable half of `REQ-MCP-05`.

use std::sync::Arc;

use bugreport_insight_core::{
    Archive, ArchiveError, ContentType, EntryId, EntryMeta, Excerpt, ExcerptOutcome, PatternMode,
    ScanBudget, ScanRequest, StoppedReason, caps,
};
use serde_json::{Value, json};

use crate::error::ToolError;
use crate::registry::ToolHandler;
use crate::tools::envelope::ToolEnvelope;
use crate::tools::params::{
    ArchiveInfoParams, EntryInfoParams, FindByTypeParams, FindEntriesParams, GrepArchiveParams,
    ListEntriesParams, RangeParams, ReadEntryParams, parse_mode,
};

/// The shared, immutable archive. `Archive` is `Send + Sync`, holds no lock,
/// and memoises the digest — so one of these per process is the whole state.
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

fn excerpt_value(e: &Excerpt) -> Value {
    json!({
        "text": e.text,
        "bytes": e.bytes,
        "truncated": e.truncated,
        "complete": e.complete,
        "decoding": format!("{:?}", e.decoding),
    })
}

// ------------------------------------------------------------------ archive_info

handler!(
    /// Identity of the bound archive. Pure index + memoised digest.
    ArchiveInfo,
    "archive_info",
    "Cheapest call: index only, no bytes read, and a SHA-256 memoised so it is paid once per \
     process. Identity of the bound archive — basename, size, entry count, version.txt, main \
     entry. Call this first to confirm you are looking at the archive you think you are.",
    ArchiveInfoParams,
    archive_info
);

fn archive_info(ctx: &Archive, p: ArchiveInfoParams) -> Result<ToolEnvelope, ToolError> {
    let identity = ctx.identity(p.with_sha256).map_err(map_err)?;
    Ok(ToolEnvelope::ok(json!({ "archive": identity })))
}

// ------------------------------------------------------------------ list_entries

handler!(
    /// Bounded listing, optionally under one path prefix.
    ListEntries,
    "list_entries",
    "Cheap: index only, no content read; 200 items max. Lists ZIP entries with sizes, optionally \
     under one path prefix such as `ANR/`, and reports what it omitted.",
    ListEntriesParams,
    list_entries
);

fn list_entries(ctx: &Archive, p: ListEntriesParams) -> Result<ToolEnvelope, ToolError> {
    let matched: Vec<Value> = ctx
        .entries()
        .iter()
        .filter(|e| match &p.path_prefix {
            Some(prefix) => e.path.starts_with(prefix.as_str()),
            None => true,
        })
        .map(entry_summary)
        .collect();
    let (kept, omitted) = cap_items(matched, p.limit);
    let env = ToolEnvelope::ok(json!({
        "entries": kept,
        "entry_count": ctx.entry_count(),
    }));
    Ok(if omitted > 0 {
        env.with_cap(StoppedReason::ItemCap, omitted)
    } else {
        env
    })
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
    let env = ToolEnvelope::ok(json!({ "entries": kept }));
    Ok(if omitted > 0 {
        env.with_cap(StoppedReason::ItemCap, omitted)
    } else {
        env
    })
}

// ------------------------------------------------------------------- entry_info

handler!(
    /// Everything known about one entry, addressed by `entry_id`.
    EntryInfo,
    "entry_info",
    "Cheap: index plus one 8 KiB detect sample. Authoritative compressed and uncompressed size, \
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
    "Moderate: an 8 KiB detect sample per entry; 200 items max. Entries of one detected content \
     type, with the basis that produced the classification.",
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
    let mut matched = Vec::new();
    for meta in ctx.entries() {
        // A per-entry detect failure is skipped rather than fatal: one
        // unreadable entry must not make the whole archive unlistable.
        if let Ok(detection) = ctx.detect(meta.id)
            && detection.content_type == want
        {
            matched.push(json!({
                "entry": entry_summary(meta),
                "detection": detection,
            }));
        }
    }
    let (kept, omitted) = cap_items(matched, p.limit);
    let env = ToolEnvelope::ok(json!({ "matches": kept }));
    Ok(if omitted > 0 {
        env.with_cap(StoppedReason::ItemCap, omitted)
    } else {
        env
    })
}

// ------------------------------------------------------------------- read_entry

handler!(
    /// One bounded slice: a line range or a byte range, never both.
    ReadEntry,
    "read_entry",
    "Moderate: reads bytes of one entry; 512-byte excerpt cap. Takes either \
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
    // Fresh handle per call — no shared mutable state, no lock.
    let mut reader = ctx.reader(id).map_err(map_err)?;
    match range {
        RangeParams::LineRange {
            line_start,
            line_end,
        } => {
            let slices = reader
                .read_line_span(line_start, line_end)
                .map_err(map_err)?;
            let any_too_long = slices
                .iter()
                .any(|s| matches!(s.outcome, ExcerptOutcome::LineTooLong { .. }));
            let lines: Vec<Value> = slices
                .iter()
                .map(|s| {
                    json!({
                        "line": s.line_no,
                        "byte_offset": s.byte_offset,
                        "byte_len_full": s.byte_len_full,
                        "excerpt": excerpt_value(&s.to_excerpt()),
                    })
                })
                .collect();
            let env = ToolEnvelope::ok(json!({
                "entry_id": id.0,
                "range": { "line_start": line_start, "line_end": line_end },
                "lines": lines,
            }));
            Ok(if any_too_long {
                // `omitted_count` is 0 here: the line was reported, only its tail
                // was withheld, and `byte_len_full` says by how much.
                env.with_cap(StoppedReason::LineTooLong, 0)
            } else {
                env
            })
        }
        RangeParams::ByteRange {
            byte_start,
            byte_end,
        } => {
            let len = byte_end.saturating_sub(byte_start);
            let bytes = reader.read_byte_range(byte_start, len).map_err(map_err)?;
            let excerpt = Excerpt::from_bytes(&bytes, caps::MAX_EXCERPT_BYTES);
            let env = ToolEnvelope::ok(json!({
                "entry_id": id.0,
                "range": { "byte_start": byte_start, "byte_end": byte_end },
                "excerpt": excerpt_value(&excerpt),
                "bytes_returned": bytes.len(),
            }));
            Ok(if excerpt.truncated {
                env.with_cap(StoppedReason::ByteBudget, 0)
            } else {
                env
            })
        }
    }
}

// ------------------------------------------------------------------ grep_archive

handler!(
    /// Content search returning locators, never raw section text.
    GrepArchive,
    "grep_archive",
    "Most expensive: scans entry content; 50 matches and 32 MiB per call. Literal or glob search \
     returning locators — `entry_id`, `line`, `byte_offset`, and a 512-byte excerpt. Both caps \
     are reported, never silent.",
    GrepArchiveParams,
    grep_archive
);

fn grep_archive(ctx: &Archive, p: GrepArchiveParams) -> Result<ToolEnvelope, ToolError> {
    let mode = parse_mode(p.mode.as_deref())
        .map_err(|m| ToolError::protocol("pattern_mode_unavailable", m))?;
    let scope: Vec<EntryId> = p
        .entry_ids
        .unwrap_or_default()
        .into_iter()
        .map(EntryId)
        .collect();
    // What the scan was *supposed* to cover. An empty scope means every entry.
    let in_scope = if scope.is_empty() {
        ctx.entry_count()
    } else {
        scope.len()
    };
    let req = ScanRequest::new(&p.pattern, mode)
        .with_scope(scope)
        .with_budget(ScanBudget {
            max_matches: p
                .max_matches
                .unwrap_or(caps::MAX_GREP_MATCHES)
                .min(caps::MAX_GREP_MATCHES),
            ..ScanBudget::DEFAULT
        });
    let outcome = ctx.scan(&req).map_err(map_err)?;
    let matches: Vec<Value> = outcome
        .matches
        .iter()
        .map(|m| {
            json!({
                "entry_id": m.entry_id.0,
                "path": m.path,
                "line": m.line,
                "line_end": m.line_end,
                "byte_offset": m.byte_offset,
                "byte_len_match": m.byte_len_match,
                "excerpt": excerpt_value(&m.excerpt),
            })
        })
        .collect();
    // `omitted_count` counts *matches* withheld, which is a different unit from
    // entries never searched. Reporting only the former would let
    // `truncated: true, omitted_count: 0` read as "capped but nothing lost"
    // while a third of the archive went unsearched — the silent truncation
    // `REQ-MCP-06` forbids. So coverage is stated explicitly and separately.
    let not_searched = in_scope.saturating_sub(outcome.scanned_entries);
    let scan = json!({
        "scanned_entries": outcome.scanned_entries,
        "entries_in_scope": in_scope,
        "entries_not_searched": not_searched,
        "coverage_complete": not_searched == 0,
        "scanned_bytes": outcome.scanned_bytes,
    });
    // `Empty` only here: the scan completed and found nothing. A scan that
    // stopped early is NOT empty — it is an incomplete answer, and saying
    // `empty` would claim there was nothing to find.
    //
    // The empty case keeps the *same* `data` shape as the success case. An
    // earlier version returned the bare scan object, which meant `data.matches`
    // existed on a successful call and not on an empty one — the same key
    // meaning two different shapes, which is a trap for any caller branching
    // on `status`.
    if matches.is_empty() && outcome.stopped.is_none() {
        return Ok(ToolEnvelope::empty(
            json!({ "matches": matches, "scan": scan }),
            "no matches",
        ));
    }
    let mut env = ToolEnvelope::ok(json!({ "matches": matches, "scan": scan }));
    Ok(if let Some(reason) = outcome.stopped {
        if not_searched > 0 {
            env = env.with_note(format!(
                "search stopped after {} of {in_scope} entries in scope; \
                 {not_searched} entries were not searched, so a match may exist in them — \
                 narrow with `entry_ids` or split the query",
                outcome.scanned_entries
            ));
        }
        env.with_cap(reason, outcome.omitted_count)
    } else if outcome.omitted_count > 0 {
        env.with_cap(StoppedReason::MatchCap, outcome.omitted_count)
    } else {
        env
    })
}
