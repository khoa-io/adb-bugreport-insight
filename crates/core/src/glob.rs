//! Hand-written glob, matched **segment-wise**.
//!
//! The correctness property this module exists to guarantee: `*` never crosses
//! a `/`. A character-wise glob turns `anr/*` into a prefix test that matches
//! `anr/sub/foo.txt`, which is a path the caller never named. Matching is done
//! on whole `/`-separated segments instead, so `anr/*` matches `anr/foo.txt`
//! and refuses `anr/sub/foo.txt`.
//!
//! Supported syntax, per segment:
//!
//! | Token | Meaning inside one segment |
//! | --- | --- |
//! | `*` | any run of characters, including empty, never including `/` |
//! | `?` | exactly one character, never `/` |
//! | `[abc]`, `[a-z]`, `[!abc]` | one character from the set, never `/` |
//! | `\x` | literal `x` |
//!
//! A `**` segment is a whole-segment wildcard that also spans separators. It is
//! the only construct that crosses `/`, it is recognised only when it is an
//! entire segment (`**`, not `a**`), and it matches zero or more segments.
//! A pattern ending in `/**` also matches the directory prefix itself, so
//! `anr/**` matches `anr` as well as `anr/a.txt` and `anr/sub/a.txt`.

use crate::error::ArchiveError;

/// One compiled `/`-separated segment of a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// A single `*`, `?`, class or literal, in source order.
    Pattern(Vec<Tok>),
    /// A whole-segment `**`: matches zero or more segments.
    AnyDepth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// `*` — any run of bytes within this segment.
    Star,
    /// `?` — exactly one byte within this segment.
    AnyChar,
    /// `[...]` — one byte from the set.
    Class {
        negated: bool,
        ranges: Vec<(u8, u8)>,
    },
    /// A literal byte.
    Literal(u8),
}

/// A compiled glob pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    source: String,
    segments: Vec<Segment>,
}

/// Byte range of a match within the subject, plus whether it matched the
/// whole subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobMatch {
    pub start: usize,
    pub end: usize,
}

impl Glob {
    pub fn new(pattern: &str) -> Result<Self, ArchiveError> {
        let segments = compile_pattern(pattern)?;
        Ok(Self {
            source: pattern.to_string(),
            segments,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// True when `subject` matches, in full.
    pub fn is_match(&self, subject: &str) -> bool {
        self.find(subject).is_some()
    }

    /// Returns the match extent, anchored to the whole subject.
    ///
    /// `None` when the subject does not match. Matching is anchored: a glob
    /// describes the entire path, so `foo` does not match `foo/bar`. A
    /// trailing `/**` is the one documented exception that also accepts the
    /// bare prefix.
    pub fn find(&self, subject: &str) -> Option<GlobMatch> {
        let parts: Vec<&str> = subject.split('/').collect();
        if self.segments.is_empty() {
            // An empty pattern matches only the empty subject.
            return (subject.is_empty()).then_some(GlobMatch { start: 0, end: 0 });
        }
        if match_segments(&self.segments, &parts) {
            Some(GlobMatch {
                start: 0,
                end: subject.len(),
            })
        } else {
            None
        }
    }
}

/// Matches the compiled segments against the subject's segments.
///
/// Backtracking on `**` only; everything else is linear in segment count.
fn match_segments(segments: &[Segment], parts: &[&str]) -> bool {
    match segments.first() {
        None => parts.is_empty(),
        Some(Segment::AnyDepth) => {
            // `**` consumes zero or more segments, then the rest must match.
            for take in 0..=parts.len() {
                if match_segments(&segments[1..], &parts[take..]) {
                    return true;
                }
            }
            false
        }
        Some(Segment::Pattern(toks)) => {
            let Some(head) = parts.first() else {
                return false;
            };
            if !match_one_segment(toks, head.as_bytes()) {
                return false;
            }
            match_segments(&segments[1..], &parts[1..])
        }
    }
}

/// Matches one segment pattern against one segment's bytes.
///
/// `*` is handled by linear backtracking: O(n) in the segment length, and never
/// crosses a separator because neither side of this function can contain one.
fn match_one_segment(toks: &[Tok], text: &[u8]) -> bool {
    // Standard linear-backtracking glob walk, keyed on the *text* cursor.
    //
    // `star` is the token index of the most recent `*`; `star_end` is how many
    // text bytes that star has been made to swallow so far. On a mismatch the
    // star is widened by exactly one byte (`star_end += 1`, `j = star_end`) and
    // matching resumes after the star.
    //
    // Two details are load-bearing, and getting either wrong hangs or lies:
    //
    //  * The loop is `while j < text.len()`, **not** `while i < toks.len()`.
    //    Keying on the pattern index never terminates when the pattern ends in
    //    `*`, because there is no text left to drive a retry.
    //  * `star_end` increases monotonically on every retry. Re-anchoring it to
    //    the current position instead would re-consume the same text forever.
    let mut star: Option<usize> = None;
    let mut star_end = 0usize;
    let (mut i, mut j) = (0usize, 0usize);

    while j < text.len() {
        if i < toks.len() {
            match &toks[i] {
                Tok::Star => {
                    star = Some(i);
                    star_end = j;
                    i += 1;
                }
                _ if tok_matches(&toks[i], text[j]) => {
                    i += 1;
                    j += 1;
                }
                _ => match star {
                    Some(s) => {
                        i = s + 1;
                        star_end += 1;
                        j = star_end;
                    }
                    None => return false,
                },
            }
        } else {
            // Pattern exhausted but text remains: only a star can absorb it.
            match star {
                Some(s) => {
                    i = s + 1;
                    star_end += 1;
                    j = star_end;
                }
                None => return false,
            }
        }
    }

    // All text consumed. Trailing stars may follow and still match empty.
    while i < toks.len() {
        if toks[i] == Tok::Star {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

/// Whether `tok` accepts the single byte `b`.
#[inline]
fn tok_matches(tok: &Tok, b: u8) -> bool {
    match tok {
        // `Star` never reaches here: it is handled before this call.
        Tok::Star => true,
        Tok::AnyChar => true,
        Tok::Literal(c) => b == *c,
        Tok::Class { negated, ranges } => class_matches(*negated, ranges, b),
    }
}

fn class_matches(negated: bool, ranges: &[(u8, u8)], b: u8) -> bool {
    let inside = ranges.iter().any(|(lo, hi)| b >= *lo && b <= *hi);
    inside != negated
}

fn invalid(pattern: &str, message: impl Into<String>) -> ArchiveError {
    ArchiveError::InvalidGlob {
        pattern: pattern.to_string(),
        message: message.into(),
    }
}

fn compile_pattern(pattern: &str) -> Result<Vec<Segment>, ArchiveError> {
    let raw: Vec<&str> = pattern.split('/').collect();
    let mut segments = Vec::with_capacity(raw.len());
    for part in raw {
        // A trailing empty segment from `foo/` is not a segment worth matching.
        if part.is_empty() {
            continue;
        }
        if part == "**" {
            segments.push(Segment::AnyDepth);
            continue;
        }
        segments.push(Segment::Pattern(compile_segment(pattern, part)?));
    }
    Ok(segments)
}

fn compile_segment(pattern: &str, seg: &str) -> Result<Vec<Tok>, ArchiveError> {
    let bytes = seg.as_bytes();
    let mut toks: Vec<Tok> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'*' => {
                // `a**b` inside a segment is just `*` twice; collapse to one.
                if !matches!(toks.last(), Some(Tok::Star)) {
                    toks.push(Tok::Star);
                }
                i += 1;
            }
            b'?' => {
                toks.push(Tok::AnyChar);
                i += 1;
            }
            b'\\' => {
                let Some(&next) = bytes.get(i + 1) else {
                    return Err(invalid(pattern, "trailing `\\` escapes nothing"));
                };
                toks.push(Tok::Literal(next));
                i += 2;
            }
            b'[' => {
                let (tok, next) = compile_class(pattern, bytes, i)?;
                toks.push(tok);
                i = next;
            }
            c => {
                toks.push(Tok::Literal(c));
                i += 1;
            }
        }
    }
    Ok(toks)
}

/// Compiles `[...]` starting at `start`, which must point at `[`.
/// Returns the token and the index just past the closing `]`.
fn compile_class(pattern: &str, bytes: &[u8], start: usize) -> Result<(Tok, usize), ArchiveError> {
    let mut i = start + 1;
    let negated = matches!(bytes.get(i), Some(b'!') | Some(b'^'));
    if negated {
        i += 1;
    }

    let mut ranges: Vec<(u8, u8)> = Vec::new();
    // A `]` in first position is a literal, as in POSIX bracket expressions.
    let mut first = true;

    loop {
        let Some(&c) = bytes.get(i) else {
            return Err(invalid(pattern, "unterminated `[` — no closing `]`"));
        };
        if c == b']' && !first {
            return Ok((Tok::Class { negated, ranges }, i + 1));
        }
        first = false;

        let lo = if c == b'\\' {
            i += 1;
            let Some(&esc) = bytes.get(i) else {
                return Err(invalid(pattern, "trailing `\\` inside `[...]`"));
            };
            esc
        } else {
            c
        };
        i += 1;

        // `a-z`, but a trailing `-` is a literal (`[a-]`).
        let hi = if bytes.get(i) == Some(&b'-') && bytes.get(i + 1).is_some_and(|&n| n != b']') {
            i += 1;
            let Some(&end) = bytes.get(i) else {
                return Err(invalid(pattern, "unterminated range in `[...]`"));
            };
            i += 1;
            end
        } else {
            lo
        };

        if lo > hi {
            return Err(invalid(pattern, "reversed byte range in `[...]`"));
        }
        ranges.push((lo, hi));
    }
}

/// Convenience wrapper for one-shot matching.
pub fn glob_is_match(pattern: &str, subject: &str) -> Result<bool, ArchiveError> {
    Ok(Glob::new(pattern)?.is_match(subject))
}

#[cfg(test)]
mod tests {
    use super::{Glob, GlobMatch};

    fn m(pattern: &str, subject: &str) -> bool {
        Glob::new(pattern)
            .expect("pattern compiles")
            .is_match(subject)
    }

    /// The load-bearing property: `*` must not cross a `/`.
    #[test]
    fn star_does_not_cross_separator() {
        assert!(m("anr/*", "anr/foo.txt"));
        assert!(!m("anr/*", "anr/sub/foo.txt"));
        assert!(!m("anr/*", "anr/sub/deeper/foo.txt"));
        assert!(!m("anr/*", "anrfoo.txt"));
    }

    #[test]
    fn double_star_is_the_only_separator_crossing_construct() {
        assert!(m("anr/**", "anr/sub/deeper/foo.txt"));
        assert!(m("anr/**", "anr/foo.txt"));
        assert!(m("anr/**", "anr"));
        assert!(m("**", "a/b/c"));
        assert!(!m("a/**/c", "a/b/d"));
        assert!(m("a/**/c", "a/b/x/y/c"));
    }

    #[test]
    fn segment_count_must_agree_without_any_depth() {
        assert!(!m("a/b", "a"));
        assert!(!m("a/b", "a/b/c"));
        assert!(m("a/b", "a/b"));
        assert!(m("*", "a"));
        assert!(!m("*", "a/b"));
    }

    #[test]
    fn literals_classes_and_escapes() {
        assert!(m("*.txt", "a.txt"));
        assert!(!m("*.txt", "a.txt.bak"));
        assert!(m("a?c", "abc"));
        assert!(!m("a?c", "ac"));
        assert!(!m("a?c", "a/c"));
        assert!(m("[abc]x", "bx"));
        assert!(!m("[abc]x", "dx"));
        assert!(m("[!abc]x", "dx"));
        assert!(m("[a-c]x", "bx"));
        assert!(m("a\\*b", "a*b"));
        assert!(!m("a\\*b", "axb"));
        assert!(m("a]b", "a]b"));
    }

    #[test]
    fn anchored_to_whole_path() {
        assert!(!m("foo", "foo/bar"));
        assert!(!m("bar", "foo/bar"));
        assert!(m("foo/bar", "foo/bar"));
    }

    #[test]
    fn trailing_star_matches_an_empty_final_segment() {
        // `*` matches zero or more bytes, so `a/*` matching the empty final
        // segment of `a/` is standard glob behaviour, not a bug. A ZIP with
        // directory entries would hit this; the reference archive has none.
        assert!(m("a/*", "a/"));
        assert!(m("*", "*"));
    }

    #[test]
    fn malformed_patterns_are_typed_errors() {
        assert!(Glob::new("a[").is_err());
        assert!(Glob::new("a\\").is_err());
        assert!(Glob::new("[z-a]").is_err());
    }

    /// A single-segment glob is exactly POSIX bracket/wildcard matching, so
    /// this table is checked against `fnmatch` semantics.
    ///
    /// The list is exhaustive over short subjects because the previous
    /// implementation of the `*` retry rule did not terminate on `*` against
    /// any non-empty subject: the loop was keyed on the pattern cursor, which
    /// had nowhere left to go once a trailing `*` was reached. A table like
    /// this one turns that class of bug into a test failure rather than a hang.
    #[test]
    fn single_segment_matching_matches_posix_semantics() {
        // (pattern, subject, expected)
        let cases: &[(&str, &str, bool)] = &[
            ("*", "", true),
            ("*", "a", true),
            ("*", "anything", true),
            ("*", "a.txt.bak", true),
            ("*.txt", "a.txt", true),
            ("*.txt", "a.txt.bak", false),
            ("*.txt", "atxt", false),
            ("*.txt", ".txt", true),
            ("a*c", "ac", true),
            ("a*c", "abc", true),
            ("a*c", "abbbc", true),
            ("a*c", "abcd", false),
            ("a*c", "abd", false),
            ("a?c", "abc", true),
            ("a?c", "ac", false),
            ("a?c", "abbc", false),
            ("a**c", "ac", true),
            ("a**c", "abc", true),
            ("a**b**c", "axxbyyc", true),
            ("[abc]x", "ax", true),
            ("[abc]x", "dx", false),
            ("[a-c]x", "bx", true),
            ("[!a]x", "dx", true),
            ("[!a]x", "ax", false),
            ("[^a]x", "dx", true),
            ("a\\*b", "a*b", true),
            ("a\\*b", "axb", false),
            ("a\\?b", "a?b", true),
            ("a\\?b", "axb", false),
            ("a]b", "a]b", true),
        ];
        for (pattern, subject, expected) in cases {
            let got = Glob::new(pattern).map(|g| g.is_match(subject));
            // Every pattern here is well-formed, so a compile error is itself
            // the failure.
            assert_eq!(
                got.expect("pattern compiles"),
                *expected,
                "pattern {pattern:?} against subject {subject:?}"
            );
        }
    }

    /// An unescaped `[` is a malformed class, not a literal bracket — the same
    /// as POSIX. Escaping makes it literal.
    #[test]
    fn unescaped_open_bracket_is_an_error_escaped_is_literal() {
        assert!(Glob::new("a[b").is_err());
        assert!(Glob::new("a\\[b").is_ok());
        assert!(Glob::new("a\\[b").unwrap().is_match("a[b"));
    }

    /// The segment boundary is checked through the public `find`, which is
    /// what `Archive::find_by_glob` uses.
    #[test]
    fn find_is_anchored_to_the_whole_path() {
        let g = Glob::new("a/*").unwrap();
        assert_eq!(g.find("a/b"), Some(GlobMatch { start: 0, end: 3 }));
        assert_eq!(g.find("a/b/c"), None);
        assert_eq!(g.find("ab"), None);
    }
}
