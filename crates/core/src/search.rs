//! Bounded content search.
//!
//! Every match carries a `byte_offset` (`REQ-MCP-ARC-05`) — the uncompressed
//! offset of the **start of the line** the match was found on, which is what
//! makes a locator actionable: an agent can ask for `read_entry` at that offset
//! and get the line back without re-searching.

use grep_matcher::Matcher as _;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use serde::{Deserialize, Serialize};

use crate::archive::{Archive, EntryId};
use crate::caps;
use crate::error::ArchiveError;
use crate::text::{Excerpt, TextDecoding};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PatternMode {
    Literal,
    Glob,
    /// `DRAFT-REQ-MCP-ARC-08`. Returns [`ArchiveError::PatternModeUnavailable`]
    /// — a typed refusal, never a silent fallback.
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MatchKind {
    Literal,
    Glob,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StoppedReason {
    ItemCap,
    MatchCap,
    ScanBudget,
    ByteBudget,
    LineTooLong,
}

impl StoppedReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ItemCap => "item_cap",
            Self::MatchCap => "match_cap",
            Self::ScanBudget => "scan_budget",
            Self::ByteBudget => "byte_budget",
            Self::LineTooLong => "line_too_long",
        }
    }
}

/// Per-call budget. Defaults come from the crate constants (`REQ-MCP-06`).
#[derive(Debug, Clone, Copy)]
pub struct ScanBudget {
    pub max_matches: usize,
    pub max_scan_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_excerpt_bytes: usize,
}

impl Default for ScanBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ScanBudget {
    pub const DEFAULT: Self = Self {
        max_matches: caps::MAX_GREP_MATCHES,
        max_scan_bytes: caps::MAX_GREP_SCAN_BYTES,
        max_entry_bytes: caps::MAX_ENTRY_BYTES,
        max_excerpt_bytes: caps::MAX_EXCERPT_BYTES,
    };
}

#[derive(Debug, Clone)]
pub struct ScanRequest {
    /// Empty means every entry.
    pub scope: Vec<EntryId>,
    pub pattern: String,
    pub mode: PatternMode,
    pub budget: ScanBudget,
}

impl ScanRequest {
    pub fn new(pattern: impl Into<String>, mode: PatternMode) -> Self {
        Self {
            scope: Vec::new(),
            pattern: pattern.into(),
            mode,
            budget: ScanBudget::DEFAULT,
        }
    }

    pub fn with_scope(mut self, scope: Vec<EntryId>) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_budget(mut self, budget: ScanBudget) -> Self {
        self.budget = budget;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchLocator {
    pub entry_id: EntryId,
    pub path: String,
    /// 1-based.
    pub line: u64,
    /// Equals `line` unless the match spans lines, which v1 never does.
    pub line_end: u64,
    /// Uncompressed offset of the line start. Required on every match.
    pub byte_offset: u64,
    /// Length of the matched text within the line.
    pub byte_len_match: u64,
    pub excerpt: Excerpt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOutcome {
    pub matches: Vec<MatchLocator>,
    pub scanned_entries: usize,
    pub scanned_bytes: u64,
    pub stopped: Option<StoppedReason>,
    pub omitted_count: u64,
}

/// Compiles the request into a matcher.
///
/// `Regex` is a **typed refusal**, not a fallback to literal: silently treating
/// `a.*b` as a literal would return matches the caller did not ask for.
fn matcher_for(req: &ScanRequest) -> Result<RegexMatcher, ArchiveError> {
    match req.mode {
        PatternMode::Literal => {
            let mut b = RegexMatcherBuilder::new();
            b.fixed_strings(true);
            b.line_terminator(None);
            b.build(&req.pattern).map_err(invalid_pattern)
        }
        PatternMode::Glob => {
            // A content glob is anchored to the whole line, matching the
            // segment-wise rule used for paths.
            let mut b = RegexMatcherBuilder::new();
            b.line_terminator(None);
            let pattern = glob_to_regex(&req.pattern);
            b.build(&pattern).map_err(invalid_pattern)
        }
        PatternMode::Regex => Err(ArchiveError::PatternModeUnavailable {
            mode: PatternMode::Regex,
            reason: "DRAFT-REQ-MCP-ARC-08 is not implemented; use Literal or Glob",
        }),
    }
}

fn invalid_pattern(e: grep_regex::Error) -> ArchiveError {
    ArchiveError::InvalidGlob {
        pattern: String::new(),
        message: e.to_string(),
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 2);
    out.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            c => {
                if "\\^$.|+()[]{}".contains(c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
    }
    out.push('$');
    out
}

pub(crate) fn run_scan(archive: &Archive, req: &ScanRequest) -> Result<ScanOutcome, ArchiveError> {
    let matcher = matcher_for(req)?;
    let budget = req.budget;

    let scope: Vec<EntryId> = if req.scope.is_empty() {
        archive.entries().iter().map(|e| e.id).collect()
    } else {
        req.scope.clone()
    };

    let mut matches: Vec<MatchLocator> = Vec::new();
    let mut scanned_entries = 0usize;
    let mut scanned_bytes = 0u64;
    let mut omitted_count = 0u64;
    let mut stopped: Option<StoppedReason> = None;

    for id in scope {
        if matches.len() >= budget.max_matches {
            stopped = Some(StoppedReason::MatchCap);
            break;
        }
        let Some(meta) = archive.entry(id) else {
            // Scoping to an id that does not exist is a caller error, not an
            // empty result.
            return Err(ArchiveError::EntryNotFound(id));
        };
        if meta.is_dir || meta.encrypted {
            // A directory has no bytes; an encrypted entry is reported by
            // `entry_info` as unreadable. Neither is a silent empty match.
            continue;
        }

        let path = meta.path.clone();
        let mut reader = archive.reader(id)?;
        reader.take_bounded(budget.max_entry_bytes);

        let mut consumed_for_entry = 0u64;
        // Walk sequentially: `read_line(n)` is random access and would restart
        // the decompressor per line, making this O(lines²).
        let mut verdict: Option<StoppedReason> = None;
        let mut omitted_here = 0u64;
        reader.for_each_line(|slice| {
            consumed_for_entry += slice.byte_len_full + 1;
            if scanned_bytes + consumed_for_entry >= budget.max_scan_bytes {
                verdict = Some(StoppedReason::ByteBudget);
                return Ok(false);
            }

            let found = match matcher.find_at(&slice.bytes, 0) {
                Ok(Some(m)) => m,
                Ok(None) => return Ok(true),
                Err(e) => {
                    return Err(ArchiveError::UnreadableEntry {
                        id,
                        source: zip::result::ZipError::Io(std::io::Error::other(e.to_string())),
                    });
                }
            };

            if matches.len() >= budget.max_matches {
                omitted_here += 1;
                verdict = Some(StoppedReason::MatchCap);
                return Ok(false);
            }

            let mut excerpt = Excerpt::from_bytes(&slice.bytes, budget.max_excerpt_bytes);
            if excerpt.decoding == TextDecoding::Binary {
                excerpt.reason = excerpt.reason.or(Some(StoppedReason::LineTooLong));
            }
            matches.push(MatchLocator {
                entry_id: id,
                path: path.clone(),
                line: slice.line_no,
                line_end: slice.line_no,
                byte_offset: slice.byte_offset,
                byte_len_match: (found.end() - found.start()) as u64,
                excerpt,
            });
            Ok(true)
        })?;
        omitted_count += omitted_here;
        if let Some(v) = verdict {
            stopped = Some(v);
        }

        scanned_entries += 1;
        scanned_bytes += consumed_for_entry;
        if stopped.is_some() {
            break;
        }
    }

    Ok(ScanOutcome {
        matches,
        scanned_entries,
        scanned_bytes,
        stopped,
        omitted_count,
    })
}
