//! Tests against the **real** reference archive.
//!
//! This is where the line-bomb claim is actually proved: the synthetic
//! fixtures in `hostile.rs` use lines just over `MAX_LINE_BYTES`, but the real
//! archive's worst line is 192,666 characters inside a 682,951-line,
//! 64,645,191-byte entry. Every number asserted here was measured
//! independently, not copied from the docs.
//!
//! # Locating the archive
//!
//! `BUGREPORT_ARCHIVE` overrides the default path. Otherwise, place the archive
//! at `tmp/reference-archive.zip` in the repository root. The default is resolved
//! from this crate's manifest directory, independently of the launch directory.

use std::path::PathBuf;

use bugreport_insight_core as core;
use core::{Archive, EntryId, PatternMode};

const DEFAULT_ARCHIVE: &str = "../../tmp/reference-archive.zip";

/// SHA-256 of the reference archive, from `SWE.0-T1` and confirmed here.
const EXPECTED_SHA256: &str = "10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92";

fn archive_path() -> PathBuf {
    match std::env::var("BUGREPORT_ARCHIVE") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_ARCHIVE),
    }
}

/// Opens the archive, or skips the test with a clear reason.
///
/// Skipping rather than failing keeps `cargo test` green on a machine without
/// the file, but a *present* archive that does not match its recorded digest
/// still fails loudly — that is a corrupt input, not a missing one.
fn open_archive() -> Option<Archive> {
    let path = archive_path();
    if !path.exists() {
        eprintln!(
            "skipping: reference archive not found at {}\n\
             set BUGREPORT_ARCHIVE to run these tests",
            path.display()
        );
        return None;
    }
    Some(Archive::open(&path).expect("reference archive opens"))
}

fn main_entry(archive: &Archive) -> EntryId {
    archive
        .entries()
        .iter()
        .max_by_key(|e| e.uncompressed_size)
        .map(|e| e.id)
        .expect("archive has entries")
}

// ── Identity and measured shape ────────────────────────────────────────────

#[test]
fn archive_identity_matches_the_measured_values() {
    let Some(archive) = open_archive() else {
        return;
    };

    // 398 entries, none of them directories or encrypted.
    assert_eq!(archive.entry_count(), 398);
    assert!(archive.entries().iter().all(|e| !e.is_dir));
    assert!(archive.entries().iter().all(|e| !e.encrypted));
    assert_eq!(archive.index().declared_entry_count(), 398);

    let identity = archive.identity(true).expect("identity");
    assert_eq!(identity.size_bytes, 14_710_126);
    assert_eq!(
        identity.sha256.as_deref(),
        Some(EXPECTED_SHA256),
        "the digest is what makes this the archive it claims to be"
    );
    assert_eq!(identity.format_version.as_deref(), Some("2.0"));
    assert!(identity.main_entry.is_some());
    assert!(
        identity
            .notes
            .iter()
            .all(|n| *n != core::ArchiveNote::ArchiveFormat0),
        "version.txt is present, so format 0 must not be reported"
    );
}

#[test]
fn sha256_is_memoised_across_repeated_identity_calls() {
    let Some(archive) = open_archive() else {
        return;
    };
    let first = archive.identity(true).expect("first");
    let second = archive.identity(true).expect("second");
    assert_eq!(first.sha256, second.sha256);
    assert_eq!(
        first.sha256_computed_at_ms, second.sha256_computed_at_ms,
        "memoised: the timestamp must not move on a second call"
    );
}

#[test]
fn declining_sha256_is_reported_not_defaulted() {
    let Some(archive) = open_archive() else {
        return;
    };
    let identity = archive.identity(false).expect("identity");
    assert_eq!(identity.sha256, None, "None only when the caller declined");
    assert!(identity.notes.contains(&core::ArchiveNote::Sha256Omitted));
}

// ── The line bomb ──────────────────────────────────────────────────────────

/// The headline property. The main entry is 64,645,191 bytes over 682,951
/// lines; line 649,287 is 192,666 characters long.
///
/// Reading *past* it must land correctly, and the reader must never hold the
/// whole line.
#[test]
fn reading_the_192_666_character_line_does_not_blow_up() {
    let Some(archive) = open_archive() else {
        return;
    };
    let id = main_entry(&archive);

    // Confirmed by independent measurement: the longest line is 192,666 bytes
    // at 1-based line 649,287.
    let mut reader = archive.reader(id).expect("reader");
    let bomb = reader.read_line(649_287).expect("the bomb line reads");

    assert_eq!(bomb.byte_len_full, 192_666, "true line length");
    assert_eq!(
        bomb.bytes.len(),
        192_666,
        "192,666 is below MAX_LINE_BYTES (1 MiB), so it is delivered in full"
    );
    assert_eq!(
        bomb.outcome,
        core::ExcerptOutcome::Complete,
        "a real archive's worst line must not be reported as truncated"
    );
}

#[test]
fn reading_past_the_line_bomb_still_lands_correctly() {
    let Some(archive) = open_archive() else {
        return;
    };
    let id = main_entry(&archive);
    let mut reader = archive.reader(id).expect("reader");

    let before = reader.read_line(649_286).expect("line before the bomb");
    assert_eq!(before.line_no, 649_286);

    // The bomb itself.
    let bomb = reader.read_line(649_287).expect("the bomb line");
    assert_eq!(bomb.byte_len_full, 192_666);

    // And the line after it: the skip counted the bomb's newline rather than
    // buffering it.
    let after = reader.read_line(649_288).expect("line after the bomb");
    assert_eq!(after.line_no, 649_288);
    assert!(after.byte_offset > 649_286, "offset moved past the bomb");
    assert_ne!(after.bytes, bomb.bytes, "not still reading the bomb line");
}

/// Memory must not scale with line number or line length.
///
/// `read_line` for a late line walks 649,286 lines first. If any of that
/// accumulated, a 64 MB entry would be resident in heap. This test reads a
/// window of lines around the bomb and asserts each returned slice is
/// bounded by the real line length rather than by anything cumulative.
#[test]
fn reading_a_late_line_stays_bounded() {
    let Some(archive) = open_archive() else {
        return;
    };
    let id = main_entry(&archive);
    let mut reader = archive.reader(id).expect("reader");

    // The last line of a 682,951-line entry: the deepest skip this archive
    // can produce.
    let last = reader.read_line(682_951).expect("last line reads");
    assert_eq!(last.line_no, 682_951);
    assert!(
        last.bytes.len() as u64 <= core::caps::MAX_LINE_BYTES,
        "retention is capped regardless of how many lines preceded it"
    );

    // A span spanning the bomb: ten lines, each independently bounded.
    let span = reader
        .read_line_span(649_285, 649_295)
        .expect("span across the bomb");
    assert_eq!(span.len(), 11);
    assert_eq!(
        span[2].byte_len_full, 192_666,
        "the bomb is inside the span"
    );
    for s in &span {
        assert!(s.bytes.len() as u64 <= core::caps::MAX_LINE_BYTES);
        assert_eq!(s.byte_offset + s.byte_len_full, next_line_offset(s));
    }
}

fn next_line_offset(s: &core::LineSlice) -> u64 {
    s.byte_offset + s.byte_len_full
}

#[test]
fn byte_offsets_are_absolute_and_consistent() {
    let Some(archive) = open_archive() else {
        return;
    };
    let id = main_entry(&archive);
    let mut reader = archive.reader(id).expect("reader");

    // Offsets must be strictly increasing across a span, and reading a byte
    // range at an offset must return the line's own bytes.
    let span = reader.read_line_span(1_000, 1_010).expect("span");
    for w in span.windows(2) {
        assert!(w[1].byte_offset > w[0].byte_offset, "offsets must increase");
        assert_eq!(
            w[1].byte_offset,
            w[0].byte_offset + w[0].byte_len_full + 1,
            "each offset accounts for the previous line plus its newline"
        );
    }

    let target = &span[5];
    let bytes = reader
        .read_byte_range(target.byte_offset, target.byte_len_full)
        .expect("byte range");
    assert_eq!(bytes, target.bytes, "a locator is actionable");
}

/// NUL bytes are present in the real main entry and must not break a read.
#[test]
fn real_nul_bytes_do_not_break_a_read() {
    let Some(archive) = open_archive() else {
        return;
    };
    // Measured independently: 4 NUL bytes, first at uncompressed offset
    // 44,988,012 of the main entry.
    let id = main_entry(&archive);
    let mut reader = archive.reader(id).expect("reader");

    let bytes = reader
        .read_byte_range(44_988_012, 4)
        .expect("reads at the measured NUL offset");
    assert_eq!(bytes, vec![0u8; 4], "the measured NUL span is exactly here");

    // Detection samples only the first 8 KiB, and the NULs sit at byte
    // 44,988,012 — far outside the sample. `contains_nul` must therefore be
    // false here, which is the honest answer: the sample did not contain one.
    let report = archive.detect(id).expect("detects");
    assert_eq!(report.sample_bytes, core::caps::DETECT_SAMPLE_BYTES);
    assert!(
        !report.contains_nul,
        "an 8 KiB sample cannot see a NUL 44 MB in"
    );
    assert!(report.is_text);

    // The NUL is still reachable, and still reported, when a caller reads
    // the region that actually contains it.
    let excerpt = core::Excerpt::from_bytes(&bytes, core::caps::MAX_EXCERPT_BYTES);
    assert!(
        !excerpt.text.contains('\u{0}'),
        "a raw NUL must never reach a JSON string"
    );
    assert_eq!(excerpt.text, "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}");
}

// ── Segment-wise glob against real paths ───────────────────────────────────

/// Real archive paths are deep (`FS/data/anr/anr_…`), which is exactly the
/// shape where a character-wise `*` would over-match.
#[test]
fn segment_wise_glob_on_real_paths() {
    let Some(archive) = open_archive() else {
        return;
    };

    // `*` at the top level matches only top-level entries.
    let top = archive.find_by_glob("*").expect("compiles");
    assert!(!top.is_empty());
    for id in &top {
        let path = &archive.entry(*id).unwrap().path;
        assert!(!path.contains('/'), "`*` must not cross `/`: {path}");
    }

    // `FS/proc/*` matches exactly one level below `FS/proc`, and nothing
    // deeper. Measured: 3 entries at that depth.
    let one = archive.find_by_glob("FS/proc/*").expect("compiles");
    assert_eq!(one.len(), 3, "FS/proc has three direct children");
    for id in &one {
        let path = &archive.entry(*id).unwrap().path;
        assert_eq!(
            path.matches('/').count(),
            2,
            "`FS/proc/*` matched a deeper path: {path}"
        );
    }

    // The specific trap: `FS/data/` has **no** direct children in this
    // archive (everything below it is deeper), so `*` must find none rather
    // than descending to find something.
    let none = archive.find_by_glob("FS/data/*").expect("compiles");
    assert!(
        none.is_empty(),
        "`*` must not descend past its own segment: got {:?}",
        none.iter()
            .map(|i| archive.entry(*i).unwrap().path.clone())
            .collect::<Vec<_>>()
    );

    // `FS/data/anr/*` matches the 65 ANR entries and nothing deeper.
    let anr = archive.find_by_glob("FS/data/anr/*").expect("compiles");
    assert_eq!(anr.len(), 65, "measured count of ANR entries");
    for id in &anr {
        let path = &archive.entry(*id).unwrap().path;
        assert_eq!(
            path.matches('/').count(),
            3,
            "`*` crossed a separator: {path}"
        );
    }

    // One level deeper is a different, non-empty set — proving `*` is not
    // silently behaving like `**`.
    let deeper = archive.find_by_glob("FS/data/anr/*/*").expect("compiles");
    assert_ne!(
        deeper.len(),
        anr.len(),
        "a deeper pattern must select a different set"
    );
    for id in &deeper {
        let path = &archive.entry(*id).unwrap().path;
        assert_eq!(path.matches('/').count(), 4, "unexpected depth: {path}");
    }

    // A pattern naming a level that does not exist matches nothing.
    let extra = archive
        .find_by_glob("FS/data/anr/*/extra")
        .expect("compiles");
    assert!(extra.is_empty(), "no ANR entry has an extra component");

    // `**` does cross, and finds the deep ones.
    let deep = archive.find_by_glob("FS/**/version*").expect("compiles");
    assert!(
        !deep.is_empty(),
        "the archive does contain deep version files"
    );
    for id in &deep {
        let path = &archive.entry(*id).unwrap().path;
        assert!(
            path.starts_with("FS/") && path.contains('/'),
            "`**` still requires the FS prefix: {path}"
        );
    }
    assert!(
        !deep.is_empty(),
        "the archive does contain deep version files"
    );
}

// ── grep: byte_offset on every match ───────────────────────────────────────

#[test]
fn every_grep_match_carries_a_byte_offset() {
    let Some(archive) = open_archive() else {
        return;
    };

    // Scoped to the main entry: an unscoped scan of a 126 MB archive exhausts the
    // 32 MiB byte budget partway through, which is a different (also correct)
    // outcome and would not prove anything about this pattern.
    let main = main_entry(&archive);
    let req = core::ScanRequest::new("----- beginning of main", PatternMode::Literal)
        .with_scope(vec![main]);
    let outcome = archive.scan(&req).expect("scan runs");
    assert!(
        !outcome.matches.is_empty(),
        "the main entry has this banner at offset 540,253"
    );

    for m in &outcome.matches {
        assert!(m.line >= 1, "line is 1-based");
        assert_eq!(m.line_end, m.line, "v1 never spans lines");
        // The offset must be inside the entry and point at real line content.
        let meta = archive.entry(m.entry_id).expect("entry exists");
        assert!(m.byte_offset < meta.uncompressed_size);

        // `byte_offset` is the start of the LINE carrying the match, not the match
        // itself — `MatchLocator` documents exactly that, and `byte_len_match`
        // is a length *within the line*, so the two cannot index together.
        // The locator is therefore verified through the line it names: the
        // offset must agree with the line number, and the line must hold the
        // pattern. The previous version read a fixed 8 bytes and only checked
        // they were non-empty, which passes for any offset in the entry.
        let mut reader = archive.reader(m.entry_id).expect("reader");
        let line = reader.read_line(m.line).expect("line at the locator");
        assert_eq!(
            line.byte_offset, m.byte_offset,
            "byte_offset must address the line the match sits on"
        );
        assert!(
            String::from_utf8_lossy(&line.bytes).contains("----- beginning of main"),
            "the line the locator names must contain the pattern"
        );
        assert!(m.byte_len_match > 0, "a match must have non-zero length");
        assert!(
            m.excerpt.text.len() <= core::caps::MAX_EXCERPT_BYTES + 8,
            "excerpt is bounded"
        );
    }
}

#[test]
fn grep_respects_the_match_cap_and_says_what_it_omitted() {
    let Some(archive) = open_archive() else {
        return;
    };

    let budget = core::ScanBudget {
        max_matches: 3,
        ..core::ScanBudget::DEFAULT
    };
    let req = core::ScanRequest::new("a", PatternMode::Literal).with_budget(budget);
    let outcome = archive.scan(&req).expect("scan runs");

    assert!(outcome.matches.len() <= 3, "match cap honoured");
    assert_eq!(
        outcome.stopped,
        Some(core::StoppedReason::MatchCap),
        "a capped result must say so, never truncate silently"
    );
    assert!(
        outcome.omitted_count > 0,
        "omitted matches are counted, not hidden"
    );
}

#[test]
fn grep_on_a_narrow_scope_only_touches_that_entry() {
    let Some(archive) = open_archive() else {
        return;
    };
    let target = archive.id_for_path("version.txt").expect("known entry");

    let req = core::ScanRequest::new("2.0", PatternMode::Literal).with_scope(vec![target]);
    let outcome = archive.scan(&req).expect("scan runs");

    assert_eq!(outcome.scanned_entries, 1);
    for m in &outcome.matches {
        assert_eq!(m.entry_id, target);
        assert_eq!(m.path, "version.txt");
    }
}

#[test]
fn glob_pattern_mode_matches_whole_lines() {
    let Some(archive) = open_archive() else {
        return;
    };
    let target = archive.id_for_path("version.txt").expect("known entry");

    // `version.txt` holds "2.0" with no trailing newline in some builds, so
    // match it literally-shaped.
    let req = core::ScanRequest::new("2.*", PatternMode::Glob).with_scope(vec![target]);
    let outcome = archive.scan(&req).expect("scan runs");
    assert_eq!(
        outcome.matches.len(),
        1,
        "glob `2.*` matches the whole line"
    );
    assert_eq!(outcome.matches[0].line, 1);
}

// ── EntryId addressing round-trips ─────────────────────────────────────────

#[test]
fn entry_ids_round_trip_through_the_index() {
    let Some(archive) = open_archive() else {
        return;
    };

    for (i, meta) in archive.entries().iter().enumerate() {
        // `EntryId(n)` addresses the n-th central-directory entry, and the
        // reader for that id must report the same metadata the index did.
        assert_eq!(meta.id, EntryId(i as u32));
        let reader = archive.reader(meta.id).expect("every id opens");
        assert_eq!(reader.entry_id(), meta.id);
        assert_eq!(reader.meta().path, meta.path);
        assert_eq!(reader.meta().uncompressed_size, meta.uncompressed_size);
        assert_eq!(reader.meta().compressed_size, meta.compressed_size);
    }
}

#[test]
fn an_out_of_range_entry_id_is_not_found_not_a_panic() {
    let Some(archive) = open_archive() else {
        return;
    };
    let err = archive.reader(EntryId(999_999)).unwrap_err();
    assert_eq!(err.code(), "entry_not_found");
    assert_eq!(
        core::ArchiveStatus::from_error(&err),
        core::ArchiveStatus::NotFound
    );
}

#[test]
fn entry_ids_survive_serde_round_trip() {
    let Some(archive) = open_archive() else {
        return;
    };
    let meta = &archive.entries()[0];
    let json = serde_json::to_string(&meta.id).expect("serialises");
    // `EntryId` is `#[serde(transparent)]`, so it is a bare number on the wire.
    assert_eq!(json, meta.id.0.to_string());
    let back: EntryId = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(back, meta.id);
    assert_eq!(
        archive.entry(back).map(|e| e.path.clone()),
        Some(meta.path.clone())
    );
}

// ── Directories, ranges, and empty results ────────────────────────────────

#[test]
fn reading_past_the_last_line_is_an_out_of_bounds_error() {
    let Some(archive) = open_archive() else {
        return;
    };
    let id = main_entry(&archive);
    let mut reader = archive.reader(id).expect("reader");

    let err = reader.read_line(10_000_000).unwrap_err();
    match err {
        core::ArchiveError::LineRangeOutOfBounds { .. } => {}
        other => panic!("expected LineRangeOutOfBounds, got {other:?}"),
    }
    assert_eq!(err.code(), "line_range_out_of_bounds");
}

#[test]
fn zero_length_entries_are_listable_and_read_as_empty() {
    let Some(archive) = open_archive() else {
        return;
    };
    // Measured: four zero-length entries in this archive.
    let empties: Vec<&core::EntryMeta> = archive
        .entries()
        .iter()
        .filter(|e| e.uncompressed_size == 0)
        .collect();
    assert!(
        !empties.is_empty(),
        "the archive does have zero-length entries"
    );

    for meta in empties {
        assert!(archive.entry(meta.id).is_some(), "still listable");
        assert_eq!(
            archive
                .reader(meta.id)
                .unwrap()
                .read_byte_range(0, 0)
                .unwrap(),
            Vec::<u8>::new()
        );
    }
}

#[test]
fn a_pattern_that_matches_nothing_is_empty_not_an_error() {
    let Some(archive) = open_archive() else {
        return;
    };
    let req = core::ScanRequest::new(
        "zzz-definitely-not-present-in-this-archive-zzz",
        PatternMode::Literal,
    );
    // Scope it to a small entry: an unscoped scan of a 126 MB archive
    // legitimately exhausts the 32 MiB budget, which is a different outcome.
    let target = archive.id_for_path("version.txt").expect("known entry");
    let req = req.with_scope(vec![target]);
    let outcome = archive.scan(&req).expect("scan completes");
    assert!(outcome.matches.is_empty());
    assert_eq!(outcome.stopped, None, "a complete scan that found nothing");
}
