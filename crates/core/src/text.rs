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
    /// can never reach a JSON string (`docs/swe.3-detailed-design/04-archive-safety.md`).
    pub fn to_excerpt(&self) -> Excerpt {
        let mut excerpt = Excerpt::from_bytes(&self.bytes, caps::MAX_EXCERPT_BYTES);
        match self.outcome {
            ExcerptOutcome::Complete => {}
            ExcerptOutcome::LineTooLong { .. } => {
                excerpt.truncated = true;
                excerpt.complete = false;
                excerpt.reason = Some(StoppedReason::LineTooLong);
            }
            ExcerptOutcome::Cut { .. } => {
                excerpt.truncated = true;
                excerpt.complete = false;
                excerpt.reason = Some(StoppedReason::ByteBudget);
            }
        }
        excerpt
    }
}

/// Sanitized UTF-8 text bounded by emitted bytes, with separate source progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Excerpt {
    pub text: String,
    /// UTF-8 bytes in text, before JSON escaping.
    pub bytes: usize,
    /// Source-prefix bytes represented by text; use this for byte-range follow-up.
    pub source_bytes: usize,
    pub truncated: bool,
    pub complete: bool,
    pub reason: Option<StoppedReason>,
    pub decoding: TextDecoding,
}

impl Excerpt {
    /// Sanitizes and retains whole characters up to the emitted-byte cap.
    pub fn from_bytes(bytes: &[u8], cap: usize) -> Self {
        let cap = cap.min(caps::MAX_EXCERPT_BYTES);
        let mut text = String::with_capacity(cap.min(bytes.len()));
        let mut source_bytes = 0;
        let mut lossy = false;
        let mut contains_nul = false;
        let mut append = |ch: char, source_len: usize, replaced: bool, nul: bool| {
            if text.len() + ch.len_utf8() > cap {
                return false;
            }
            text.push(ch);
            source_bytes += source_len;
            lossy |= replaced;
            contains_nul |= nul;
            true
        };
        // Replacement never shrinks source bytes. One code point of look-ahead
        // avoids validating or allocating the entire retained line to emit 512 bytes.
        let mut rest = &bytes[..bytes.len().min(cap + 4)];
        'decode: while !rest.is_empty() {
            let (valid_len, invalid_len) = match std::str::from_utf8(rest) {
                Ok(_) => (rest.len(), None),
                Err(error) => (
                    error.valid_up_to(),
                    Some(
                        error
                            .error_len()
                            .unwrap_or(rest.len() - error.valid_up_to()),
                    ),
                ),
            };
            for ch in String::from_utf8_lossy(&rest[..valid_len]).chars() {
                let nul = ch == '\0';
                let emitted = if nul { '\u{fffd}' } else { ch };
                if !append(emitted, ch.len_utf8(), nul, nul) {
                    break 'decode;
                }
            }
            rest = &rest[valid_len..];
            if let Some(length) = invalid_len {
                if !append('\u{fffd}', length, true, false) {
                    break;
                }
                rest = &rest[length..];
            }
        }
        let truncated = source_bytes < bytes.len();
        Self {
            bytes: text.len(),
            source_bytes,
            text,
            truncated,
            complete: !truncated && !lossy,
            reason: truncated.then_some(StoppedReason::ByteBudget),
            decoding: if contains_nul {
                TextDecoding::Binary
            } else if lossy {
                TextDecoding::Utf8Lossy
            } else {
                TextDecoding::Utf8
            },
        }
    }

    pub fn contains_nul(&self) -> bool {
        self.decoding == TextDecoding::Binary
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

#[cfg(test)]
mod tests {
    use super::*;

    fn line(bytes: &[u8]) -> LineSlice {
        LineSlice {
            entry_id: EntryId(3),
            line_no: 1,
            bytes: bytes.to_vec(),
            byte_offset: 0,
            byte_len_full: bytes.len() as u64,
            outcome: ExcerptOutcome::Complete,
        }
    }

    #[test]
    fn line_excerpt_preserves_multibyte_utf8() {
        for text in [
            "",
            "ASCII",
            "café",
            "Tiếng Việt",
            "日本語",
            "🦀",
            "e\u{301}",
        ] {
            let excerpt = line(text.as_bytes()).to_excerpt();
            assert_eq!(excerpt.text, text);
            assert!(matches!(excerpt.decoding, TextDecoding::Utf8));
            assert_eq!(excerpt.bytes, text.len());
            assert!(!excerpt.truncated);
            assert!(excerpt.complete);
        }
    }

    #[test]
    fn line_excerpt_replaces_invalid_utf8_and_nul_without_corrupting_valid_text() {
        for (bytes, expected, nul) in [
            (b"caf\xc3\xa9\0fin".as_slice(), "café\u{fffd}fin", true),
            (b"caf\xc3\xa9\xfffin".as_slice(), "café\u{fffd}fin", false),
            (b"tail\xe2\x82".as_slice(), "tail\u{fffd}", false),
        ] {
            let excerpt = line(bytes).to_excerpt();
            assert_eq!(excerpt.text, expected);
            assert!(!excerpt.text.contains('\0'));
            assert_eq!(excerpt.bytes, expected.len());
            assert_eq!(excerpt.source_bytes, bytes.len());
            assert!(!excerpt.complete);
            assert!(matches!(
                (excerpt.decoding, nul),
                (TextDecoding::Binary, true) | (TextDecoding::Utf8Lossy, false)
            ));
        }
    }
    #[test]
    fn emitted_excerpt_cap_preserves_character_boundaries_and_source_progress() {
        for (input, expected, source_bytes, truncated) in [
            ("a".repeat(512), "a".repeat(512), 512, false),
            ("a".repeat(513), "a".repeat(512), 512, true),
            (format!("{}é", "a".repeat(511)), "a".repeat(511), 511, true),
            (
                "café".repeat(120),
                format!("{}ca", "café".repeat(102)),
                512,
                true,
            ),
        ] {
            for excerpt in [
                Excerpt::from_bytes(input.as_bytes(), 512),
                line(input.as_bytes()).to_excerpt(),
            ] {
                assert_eq!(excerpt.text, expected);
                assert_eq!(excerpt.bytes, expected.len());
                assert_eq!(excerpt.source_bytes, source_bytes);
                assert_eq!(excerpt.truncated, truncated);
                assert_eq!(excerpt.complete, !truncated);
                assert!(excerpt.bytes <= 512);
                assert!(matches!(excerpt.decoding, TextDecoding::Utf8));
            }
        }
    }

    #[test]
    fn emitted_excerpt_cap_accounts_for_replacement_expansion() {
        for input in [vec![0; 512], vec![0xff; 512]] {
            let excerpt = Excerpt::from_bytes(&input, 512);
            assert_eq!(excerpt.text, "\u{fffd}".repeat(170));
            assert_eq!(excerpt.bytes, 510);
            assert_eq!(excerpt.source_bytes, 170);
            assert!(excerpt.truncated);
            assert!(!excerpt.complete);
        }
        let short = Excerpt::from_bytes(b"a\0b", 512);
        assert_eq!(short.text, "a\u{fffd}b");
        assert_eq!(short.bytes, 5);
        assert_eq!(short.source_bytes, 3);
        assert!(!short.truncated);
        assert!(!short.complete);
    }

    #[test]
    fn excerpt_cap_handles_zero_small_caps_and_json_escaping() {
        for cap in [0, 1, 2, 3, 4, 511, 512, 513] {
            for input in ["é🦀".as_bytes(), b"\xffa", b"\0a"] {
                let excerpt = Excerpt::from_bytes(input, cap);
                assert!(excerpt.text.len() <= cap.min(512));
                assert_eq!(excerpt.bytes, excerpt.text.len());
                assert_eq!(excerpt.truncated, excerpt.source_bytes < input.len());
            }
        }
        let text = "\\".repeat(512);
        let excerpt = Excerpt::from_bytes(text.as_bytes(), 512);
        assert_eq!(excerpt.bytes, 512);
        assert!(!excerpt.truncated);
        assert!(serde_json::to_string(&excerpt.text).unwrap().len() > 512);
    }
}
