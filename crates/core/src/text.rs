//! Text shaping: bounded excerpts, line outcomes, and line indexing.

use serde::{Deserialize, Serialize};

use crate::archive::EntryId;
use crate::caps;
use crate::error::ArchiveError;
use crate::search::StoppedReason;

/// How a line or excerpt came out. Never silently lossy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExcerptOutcome {
    Complete,
    /// Line exceeded `MAX_LINE_BYTES`. The remainder is **skipped** by
    /// counting newlines through the 64 KiB scratch buffer, never buffered.
    LineTooLong {
        capped_at: u64,
        skipped: u64,
    },
    /// Cut short by a budget rather than by the input.
    Cut {
        at: u64,
    },
}

/// One line, bounded.
///
/// `bytes` may be shorter than `byte_len_full`. When it is, `outcome` is
/// [`ExcerptOutcome::LineTooLong`] and the difference is `skipped`.
#[derive(Debug, Clone)]
pub struct LineSlice {
    pub entry_id: EntryId,
    /// 1-based.
    pub line_no: u64,
    /// Capped at `MAX_LINE_BYTES`.
    pub bytes: Vec<u8>,
    /// Uncompressed offset of the line's first byte.
    pub byte_offset: u64,
    /// True line length in bytes, excluding the terminator. Recorded even when
    /// `bytes` is short.
    pub byte_len_full: u64,
    pub outcome: ExcerptOutcome,
}

impl LineSlice {
    /// Decodes this line's bytes into an [`Excerpt`].
    ///
    /// NUL bytes become U+FFFD and invalid UTF-8 is replaced, so a raw `\u0000`
    /// can never reach a JSON string (`06-hostile-archive.md` §1).
    pub fn to_excerpt(&self) -> Excerpt {
        let mut text = String::with_capacity(self.bytes.len());
        let mut contains_nul = false;
        for &b in &self.bytes {
            if b == 0 {
                contains_nul = true;
                text.push('\u{FFFD}');
            } else {
                text.push(b as char);
            }
        }
        let complete_utf8 = std::str::from_utf8(&self.bytes).is_ok() && !contains_nul;
        let decoding = if contains_nul {
            TextDecoding::Binary
        } else if complete_utf8 {
            TextDecoding::Utf8
        } else {
            // Byte-per-char above is lossy for multi-byte UTF-8, so re-decode
            // properly when the bytes were valid UTF-8 all along.
            match std::str::from_utf8(&self.bytes) {
                Ok(s) => {
                    text.clear();
                    text.push_str(s);
                    TextDecoding::Utf8
                }
                Err(_) => TextDecoding::Utf8Lossy,
            }
        };
        Excerpt {
            bytes: self.bytes.len(),
            truncated: matches!(self.outcome, ExcerptOutcome::LineTooLong { .. }),
            complete: matches!(self.outcome, ExcerptOutcome::Complete),
            reason: match self.outcome {
                ExcerptOutcome::LineTooLong { .. } => Some(StoppedReason::LineTooLong),
                ExcerptOutcome::Cut { .. } => Some(StoppedReason::ItemCap),
                ExcerptOutcome::Complete => None,
            },
            text,
            decoding,
        }
    }
}

/// A bounded piece of text handed to an agent. Hard-capped at
/// [`caps::MAX_EXCERPT_BYTES`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Excerpt {
    pub text: String,
    pub bytes: usize,
    pub truncated: bool,
    pub complete: bool,
    pub reason: Option<StoppedReason>,
    pub decoding: TextDecoding,
}

impl Excerpt {
    /// Builds an excerpt from raw bytes, capped at `MAX_EXCERPT_BYTES`.
    pub fn from_bytes(bytes: &[u8], cap: usize) -> Self {
        let cap = cap.min(caps::MAX_EXCERPT_BYTES);
        let truncated = bytes.len() > cap;
        let slice = &bytes[..bytes.len().min(cap)];

        let contains_nul = slice.contains(&0);
        let decoding = match std::str::from_utf8(slice) {
            Ok(_) if !contains_nul => TextDecoding::Utf8,
            _ => TextDecoding::Utf8Lossy,
        };
        // NUL becomes U+FFFD; invalid sequences are replaced by from_utf8_lossy.
        let lossy = String::from_utf8_lossy(slice);
        let text = lossy.replace('\0', "\u{FFFD}");

        Self {
            text,
            bytes: slice.len(),
            truncated,
            complete: !truncated,
            reason: truncated.then_some(StoppedReason::ItemCap),
            decoding,
        }
    }

    pub fn contains_nul(&self) -> bool {
        self.text.contains('\u{FFFD}') && self.decoding != TextDecoding::Utf8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TextDecoding {
    Utf8,
    Utf8Lossy,
    Binary,
}

/// Lazily built per entry, capped. Never materialises a 192,666-char line.
#[derive(Debug, Clone, Default)]
pub struct LineIndex {
    /// `(byte_offset, line_no)` checkpoints every `stride` lines.
    checkpoints: Vec<(u64, u64)>,
    total_lines: Option<u64>,
    capped: bool,
}

impl LineIndex {
    /// Walks the entry once, recording checkpoints every `stride` lines,
    /// using the same bounded scratch buffer as the reader.
    ///
    /// Memory is O(`max_checkpoints`); the walk is sequential, so time is
    /// O(entry bytes) rather than the O(lines²) a per-line `read_line` loop
    /// would cost.
    pub fn build(
        reader: &mut crate::reader::EntryReader,
        max_checkpoints: usize,
    ) -> Result<Self, ArchiveError> {
        let stride = 1024u64;
        let mut checkpoints = vec![(0u64, 1u64)];
        let mut line_no = 0u64;
        let mut capped = false;

        reader.for_each_line(|slice| {
            line_no += 1;
            if line_no.is_multiple_of(stride) {
                if checkpoints.len() >= max_checkpoints {
                    capped = true;
                    return Ok(false);
                }
                checkpoints.push((slice.byte_offset, slice.line_no));
            }
            Ok(true)
        })?;

        Ok(Self {
            checkpoints,
            // A capped walk did not reach the end, so the total is unknown —
            // reporting a partial count as fact would be a lie.
            total_lines: if capped { None } else { Some(line_no) },
            capped,
        })
    }

    pub fn total_lines(&self) -> Option<u64> {
        self.total_lines
    }

    pub fn is_capped(&self) -> bool {
        self.capped
    }

    /// Nearest checkpoint at or below `line_no`.
    pub fn checkpoint_for(&self, line_no: u64) -> Option<(u64, u64)> {
        self.checkpoints
            .iter()
            .copied()
            .rev()
            .find(|(_, ln)| *ln <= line_no)
    }
}
