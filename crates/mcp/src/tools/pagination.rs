//! Bounded, process-local continuation of archive content searches.
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bugreport_insight_core::{
    EntryId, MatchLocator, ScanRequest, ScanSession, ScanStep, StoppedReason, caps,
};
use serde_json::{Value, json};

use super::envelope::ToolEnvelope;
use super::handlers::{Ctx, excerpt_value};
use super::params::{GrepArchiveParams, parse_mode};
use crate::error::ToolError;
use crate::registry::ToolHandler;

const MAX_SESSIONS: usize = 8;
const TTL: Duration = Duration::from_secs(300);
const MIN_RESPONSE: usize = 2048;
const MAX_RESPONSE: usize = 262_144;
const DEFAULT_RESPONSE: usize = 65_536;
const CURSOR_PLACEHOLDER: &str = "00000000000000000000000000000000";

struct Continuation {
    scan: ScanSession,
    pending: Option<MatchLocator>,
    touched: Instant,
}

/// The only stateful archive tool: bounded live decoders, never stored ZIP text.
pub struct GrepArchive {
    ctx: Ctx,
    sessions: Mutex<HashMap<String, Continuation>>,
    tokens: std::collections::hash_map::RandomState,
    sequence: std::sync::atomic::AtomicU64,
}

impl GrepArchive {
    pub const NAME: &'static str = "grep_archive";
    pub const DESCRIPTION: &'static str = "Confirmation required per page: scans entry content; up to 50 matches and 32 MiB new source bytes per page. \
         Client response_budget bounds serialized tool-result JSON; scan_budget bounds scanning. \
         Literal or glob locators with 512-byte excerpts; next_cursor resumes without replay. \
         Send cursor alone with optional budgets/max_matches; cursors expire after 300 seconds.";

    pub fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            sessions: Mutex::new(HashMap::new()),
            tokens: std::collections::hash_map::RandomState::new(),
            sequence: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn token(&self) -> String {
        let n = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut a = self.tokens.build_hasher();
        a.write_u64(n);
        let mut b = self.tokens.build_hasher();
        b.write_u64(n);
        b.write_u8(1);
        format!("{:016x}{:016x}", a.finish(), b.finish())
    }
}

fn invalid(message: impl Into<String>) -> ToolError {
    ToolError::protocol("invalid_params", message)
}

impl ToolHandler for GrepArchive {
    fn name(&self) -> &'static str {
        Self::NAME
    }
    fn description(&self) -> &'static str {
        Self::DESCRIPTION
    }

    fn call(&self, params: Value) -> Result<ToolEnvelope, ToolError> {
        let p: GrepArchiveParams =
            serde_json::from_value(params).map_err(|e| invalid(e.to_string()))?;
        let response_budget = p.response_budget.unwrap_or(DEFAULT_RESPONSE);
        let scan_budget = p.scan_budget.unwrap_or(caps::MAX_GREP_SCAN_BYTES);
        let max_matches = p.max_matches.unwrap_or(caps::MAX_GREP_MATCHES);
        if !(MIN_RESPONSE..=MAX_RESPONSE).contains(&response_budget)
            || scan_budget == 0
            || scan_budget > caps::MAX_GREP_SCAN_BYTES
            || max_matches == 0
            || max_matches > caps::MAX_GREP_MATCHES
        {
            return Err(invalid(
                "response_budget: 2048..=262144; scan_budget: 1..=33554432; max_matches: 1..=50",
            ));
        }
        if p.cursor.is_some() && (p.pattern.is_some() || p.mode.is_some() || p.entry_ids.is_some())
        {
            return Err(invalid(
                "continuation accepts cursor and budgets/max_matches only",
            ));
        }
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| invalid("search session lock unavailable"))?;
        sessions.retain(|_, s| s.touched.elapsed() < TTL);
        let mut state = if let Some(cursor) = &p.cursor {
            sessions
                .remove(cursor)
                .ok_or_else(|| invalid("unknown, expired, or already consumed cursor"))?
        } else {
            if sessions.len() >= MAX_SESSIONS {
                return Err(invalid(
                    "eight searches are active; finish a search or wait for expiry",
                ));
            }
            let pattern = p
                .pattern
                .ok_or_else(|| invalid("pattern is required for a new search"))?;
            let mode = parse_mode(p.mode.as_deref())
                .map_err(|m| ToolError::protocol("pattern_mode_unavailable", m))?;
            let req = ScanRequest::new(pattern, mode).with_scope(
                p.entry_ids
                    .unwrap_or_default()
                    .into_iter()
                    .map(EntryId)
                    .collect(),
            );
            Continuation {
                scan: ScanSession::new(&self.ctx, req).map_err(|e| ToolError::from_archive(&e))?,
                pending: None,
                touched: Instant::now(),
            }
        };
        let start_bytes = state.scan.total_scanned_bytes;
        let mut remaining = scan_budget;
        let mut matches = Vec::new();
        let reason = loop {
            let step = if let Some(found) = state.pending.take() {
                ScanStep::Match(found)
            } else {
                state
                    .scan
                    .next_match(&mut remaining)
                    .map_err(|e| ToolError::from_archive(&e))?
            };
            match step {
                ScanStep::Match(found) => {
                    if matches.len() == max_matches {
                        state.pending = Some(found);
                        break Some(StoppedReason::MatchCap);
                    }
                    matches.push(locator(&found));
                    // Reserve the complete continuation metadata before accepting a match.
                    // The final reason/notes may differ; 128 bytes cover that small variation.
                    let candidate = envelope(
                        &state,
                        &matches,
                        start_bytes,
                        Some(CURSOR_PLACEHOLDER),
                        Some(StoppedReason::ByteBudget),
                    );
                    if result_size(&candidate)? + 128 > response_budget {
                        matches.pop();
                        state.pending = Some(found);
                        if matches.is_empty() {
                            if let Some(cursor) = p.cursor {
                                state.touched = Instant::now();
                                sessions.insert(cursor, state);
                            }
                            return Err(invalid(
                                "response_budget cannot hold one locator; increase it and retry the same cursor/query",
                            ));
                        }
                        break Some(StoppedReason::ByteBudget);
                    }
                }
                ScanStep::Paused(reason) => break Some(reason),
                ScanStep::Finished => break None,
            }
        };
        let has_more = state.pending.is_some() || !state.scan.exhausted();
        let next = has_more.then(|| self.token());
        let env = envelope(&state, &matches, start_bytes, next.as_deref(), reason);
        if result_size(&env)? > response_budget {
            // Fixed metadata is bounded; this is reachable only if that contract changes.
            if let Some(cursor) = p.cursor {
                sessions.insert(cursor, state);
            }
            return Err(invalid(
                "response_budget cannot hold page metadata; increase it",
            ));
        }
        if let Some(next) = next {
            state.touched = Instant::now();
            sessions.insert(next, state);
        }
        Ok(env)
    }
}

fn locator(m: &MatchLocator) -> Value {
    json!({
        "entry_id": m.entry_id.0, "path": m.path, "line": m.line, "line_end": m.line_end,
        "byte_offset": m.byte_offset, "byte_len_match": m.byte_len_match,
        "excerpt": excerpt_value(&m.excerpt),
    })
}

fn envelope(
    state: &Continuation,
    matches: &[Value],
    start: u64,
    next: Option<&str>,
    reason: Option<StoppedReason>,
) -> ToolEnvelope {
    let s = &state.scan;
    let complete = s.coverage_complete() && state.pending.is_none() && next.is_none();
    let data = json!({
        "matches": matches, "next_cursor": next,
        "scan": {
            "scanned_entries": s.scanned_entries,
            "completed_entries": s.completed_entries,
            "entries_in_scope": s.entries_in_scope(),
            "entries_not_searched": s.entries_not_started(),
            "partial_entry_id": s.partial_entry(),
            "skipped_entries": s.skipped_entries,
            "skipped_lines": s.skipped_lines,
            "coverage_complete": complete,
            "scanned_bytes": s.total_scanned_bytes - start,
            "total_scanned_bytes": s.total_scanned_bytes,
        }
    });
    let mut env = if matches.is_empty() && complete {
        ToolEnvelope::empty(data, "no matches in this completed search page")
    } else {
        ToolEnvelope::ok(data)
    };
    if let Some(reason) = reason {
        env = env.with_cap(reason, 0).with_note("search has more work/results; continue with next_cursor; unknown matches are not counted in omitted_count");
    } else if s.skipped_lines > 0 || s.skipped_entries > 0 {
        let gap_reason = if s.skipped_lines > 0 {
            StoppedReason::LineTooLong
        } else {
            StoppedReason::SkippedContent
        };
        env = env.with_cap(gap_reason, 0).with_note(
            "coverage is incomplete: encrypted entries or lines over 1 MiB were skipped",
        );
    } else if matches.iter().any(|m| m["excerpt"]["truncated"] == true) {
        env = env
            .with_cap(StoppedReason::ByteBudget, 0)
            .with_note("match excerpts were shortened; use read_entry at source byte offsets");
    }
    env
}

/// Upper bound for the SDK's text-only CallToolResult, including JSON escaping.
/// The extra 256 bytes cover its fixed content/isError/resultType wrapper.
fn result_size(env: &ToolEnvelope) -> Result<usize, ToolError> {
    let body = serde_json::to_string(env).map_err(|e| invalid(e.to_string()))?;
    Ok(serde_json::to_vec(&body)
        .map_err(|e| invalid(e.to_string()))?
        .len()
        + 256)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bugreport_insight_core::Archive;
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
    use std::sync::Arc;

    #[test]
    fn capacity_expiry_and_replay_bound_live_sessions() {
        let path = scratch_dir().join("page-session-lifecycle.zip");
        std::fs::write(&path, ZipBuilder::new().add("text", b"needle\n").finish()).unwrap();
        let tool = GrepArchive::new(Arc::new(Archive::open(path).unwrap()));
        let mut cursors = Vec::new();
        for _ in 0..MAX_SESSIONS {
            let env = tool
                .call(json!({"pattern":"needle","mode":"literal","scan_budget":1}))
                .unwrap();
            cursors.push(env.data["next_cursor"].as_str().unwrap().to_string());
        }
        assert!(tool.call(json!({"pattern":"needle"})).is_err());
        tool.sessions
            .lock()
            .unwrap()
            .get_mut(&cursors[0])
            .unwrap()
            .touched = Instant::now() - TTL;
        assert!(tool.call(json!({"cursor":cursors[0]})).is_err());
        assert_eq!(tool.sessions.lock().unwrap().len(), MAX_SESSIONS - 1);
        let env = tool.call(json!({"cursor":cursors[1]})).unwrap();
        assert!(env.data["next_cursor"].is_null());
        assert!(tool.call(json!({"cursor":cursors[1]})).is_err());
    }

    #[test]
    fn too_small_locator_budget_preserves_continuation_for_larger_retry() {
        let path = scratch_dir().join("page-long-path.zip");
        std::fs::write(
            &path,
            ZipBuilder::new()
                .add(&"p".repeat(3000), b"needle\n")
                .finish(),
        )
        .unwrap();
        let tool = GrepArchive::new(Arc::new(Archive::open(path).unwrap()));
        let first = tool
            .call(json!({"pattern":"needle","mode":"literal","scan_budget":1}))
            .unwrap();
        let cursor = &first.data["next_cursor"];
        assert!(
            tool.call(json!({"cursor":cursor,"response_budget":2048}))
                .is_err()
        );
        let last = tool
            .call(json!({"cursor":cursor,"response_budget":16384}))
            .unwrap();
        assert_eq!(last.data["matches"][0]["line"], 1);
        assert_eq!(last.data["scan"]["coverage_complete"], true);
        assert!(last.data["next_cursor"].is_null());
    }
}
