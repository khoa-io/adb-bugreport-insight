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
use crate::text::Excerpt;

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
    SkippedContent,
    EndOfEntry,
}

impl StoppedReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ItemCap => "item_cap",
            Self::MatchCap => "match_cap",
            Self::ScanBudget => "scan_budget",
            Self::ByteBudget => "byte_budget",
            Self::LineTooLong => "line_too_long",
            Self::SkippedContent => "skipped_content",
            Self::EndOfEntry => "end_of_entry",
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
    pub completed_entries: usize,
    pub skipped_entries: usize,
    pub skipped_lines: u64,
    pub coverage_complete: bool,
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

/// Persistent search position; decoder and partial lines survive page boundaries.
#[derive(Debug)]
pub struct ScanSession {
    archive: Archive,
    request: ScanRequest,
    matcher: RegexMatcher,
    scope: Vec<EntryId>,
    index: usize,
    stream: Option<crate::reader::SearchStream>,
    carry: Vec<u8>,
    carry_pos: usize,
    line: Vec<u8>,
    line_no: u64,
    offset: u64,
    line_start: u64,
    overlong: bool,
    pub scanned_entries: usize,
    pub completed_entries: usize,
    pub skipped_entries: usize,
    pub skipped_lines: u64,
    pub total_scanned_bytes: u64,
}

/// One confirmed match, a page boundary, or exhaustion of the selected scope.
#[derive(Debug)]
pub enum ScanStep {
    Match(MatchLocator),
    Paused(StoppedReason),
    Finished,
}

impl ScanSession {
    pub fn new(archive: &Archive, request: ScanRequest) -> Result<Self, ArchiveError> {
        let matcher = matcher_for(&request)?;
        let mut scope = if request.scope.is_empty() {
            archive.entries().iter().map(|e| e.id).collect::<Vec<_>>()
        } else {
            request.scope.clone()
        };
        let mut seen = std::collections::HashSet::new();
        scope.retain(|id| seen.insert(*id));
        for id in &scope {
            if archive.entry(*id).is_none() {
                return Err(ArchiveError::EntryNotFound(*id));
            }
        }
        Ok(Self {
            archive: archive.clone(),
            request,
            matcher,
            scope,
            index: 0,
            stream: None,
            carry: Vec::new(),
            carry_pos: 0,
            line: Vec::new(),
            line_no: 1,
            offset: 0,
            line_start: 0,
            overlong: false,
            scanned_entries: 0,
            completed_entries: 0,
            skipped_entries: 0,
            skipped_lines: 0,
            total_scanned_bytes: 0,
        })
    }

    pub fn entries_in_scope(&self) -> usize {
        self.scope.len()
    }
    pub fn entries_not_started(&self) -> usize {
        self.scope
            .len()
            .saturating_sub(self.index + usize::from(self.stream.is_some()))
    }
    pub fn partial_entry(&self) -> Option<EntryId> {
        self.stream.as_ref().map(|_| self.scope[self.index])
    }
    pub fn exhausted(&self) -> bool {
        self.index == self.scope.len()
    }
    pub fn coverage_complete(&self) -> bool {
        self.exhausted() && self.skipped_entries == 0 && self.skipped_lines == 0
    }

    /// Reads no more than `remaining` new decompressed source bytes.
    /// Buffered bytes were charged to the page that requested them.
    pub fn next_match(&mut self, remaining: &mut u64) -> Result<ScanStep, ArchiveError> {
        loop {
            if self.exhausted() {
                return Ok(ScanStep::Finished);
            }
            let id = self.scope[self.index];
            if self.stream.is_none() {
                let meta = self
                    .archive
                    .entry(id)
                    .ok_or(ArchiveError::EntryNotFound(id))?;
                if meta.is_dir {
                    self.completed_entries += 1;
                    self.index += 1;
                    continue;
                }
                if meta.encrypted {
                    self.skipped_entries += 1;
                    self.index += 1;
                    continue;
                }
                let mut reader = self.archive.reader(id)?;
                reader.take_bounded(self.request.budget.max_entry_bytes);
                self.stream = Some(crate::reader::SearchStream::new(reader)?);
                self.scanned_entries += 1;
                self.line_no = 1;
                self.offset = 0;
                self.line_start = 0;
            }
            if self.carry_pos == self.carry.len() {
                if *remaining == 0 {
                    return Ok(ScanStep::Paused(StoppedReason::ScanBudget));
                }
                self.carry = self
                    .stream
                    .as_ref()
                    .ok_or(ArchiveError::EntryNotFound(id))?
                    .read((*remaining).min(caps::LINE_SCRATCH_BYTES as u64) as usize)?;
                self.carry_pos = 0;
                let n = self.carry.len() as u64;
                *remaining -= n;
                self.total_scanned_bytes += n;
                if n == 0 {
                    let found = if self.line.is_empty() && !self.overlong {
                        None
                    } else {
                        self.finish_line(id)?
                    };
                    self.stream = None;
                    self.completed_entries += 1;
                    self.index += 1;
                    if let Some(found) = found {
                        return Ok(ScanStep::Match(found));
                    }
                    continue;
                }
            }
            while self.carry_pos < self.carry.len() {
                let byte = self.carry[self.carry_pos];
                self.carry_pos += 1;
                self.offset += 1;
                if byte == b'\n' {
                    let found = self.finish_line(id)?;
                    self.line_no += 1;
                    self.line_start = self.offset;
                    if let Some(found) = found {
                        return Ok(ScanStep::Match(found));
                    }
                } else if !self.overlong {
                    if self.line.len() == caps::MAX_LINE_BYTES as usize {
                        self.line.clear();
                        self.overlong = true;
                        self.skipped_lines += 1;
                    } else {
                        self.line.push(byte);
                    }
                }
            }
        }
    }

    fn finish_line(&mut self, id: EntryId) -> Result<Option<MatchLocator>, ArchiveError> {
        let matched = if self.overlong {
            None
        } else {
            self.matcher
                .find_at(&self.line, 0)
                .map_err(|e| ArchiveError::UnreadableEntry {
                    id,
                    source: zip::result::ZipError::Io(std::io::Error::other(e.to_string())),
                })?
                .map(|m| (m.end() - m.start()) as u64)
        };
        let found = matched.map(|byte_len_match| MatchLocator {
            entry_id: id,
            path: self
                .archive
                .entry(id)
                .map(|m| m.path.clone())
                .unwrap_or_default(),
            line: self.line_no,
            line_end: self.line_no,
            byte_offset: self.line_start,
            byte_len_match,
            excerpt: Excerpt::from_bytes(&self.line, self.request.budget.max_excerpt_bytes),
        });
        self.line.clear();
        self.overlong = false;
        Ok(found)
    }
}

pub(crate) fn run_scan(archive: &Archive, req: &ScanRequest) -> Result<ScanOutcome, ArchiveError> {
    let mut session = ScanSession::new(archive, req.clone())?;
    let mut remaining = req.budget.max_scan_bytes;
    let mut matches = Vec::new();
    let mut omitted_count = 0;
    let stopped = loop {
        match session.next_match(&mut remaining)? {
            ScanStep::Match(found) if matches.len() < req.budget.max_matches => matches.push(found),
            ScanStep::Match(_) => {
                omitted_count = 1;
                break Some(StoppedReason::MatchCap);
            }
            ScanStep::Paused(reason) => break Some(reason),
            ScanStep::Finished => {
                break if session.skipped_lines > 0 {
                    Some(StoppedReason::LineTooLong)
                } else if session.skipped_entries > 0 {
                    Some(StoppedReason::SkippedContent)
                } else {
                    None
                };
            }
        }
    };
    Ok(ScanOutcome {
        matches,
        scanned_entries: session.scanned_entries,
        scanned_bytes: session.total_scanned_bytes,
        completed_entries: session.completed_entries,
        skipped_entries: session.skipped_entries,
        skipped_lines: session.skipped_lines,
        coverage_complete: session.coverage_complete(),
        stopped,
        omitted_count,
    })
}
