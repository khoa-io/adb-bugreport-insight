//! Hostile-archive fixtures, built synthetically.
//!
//! No dependency on the reference archive, so these run anywhere and pin the
//! behaviour `06-hostile-archive.md` §6 specifies. Each fixture is a ZIP
//! written to disk and read back through the public API only.

use std::io::Write;
use std::path::{Path, PathBuf};

use bugreport_insight_core as core;
use core::caps;
use core::{Archive, ArchiveLimits, LimitViolation, PatternMode};

fn scratch_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/hostile-fixtures");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Minimal stored-entry ZIP writer. Stored (method 0) keeps the fixture
/// byte-exact, so a test can corrupt it at a known offset.
struct ZipBuilder {
    buf: Vec<u8>,
}

impl ZipBuilder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn add(self, name: &str, data: &[u8]) -> Self {
        self.add_raw(name.as_bytes(), data)
    }

    fn add_raw(mut self, name: &[u8], data: &[u8]) -> Self {
        let crc = crc32(data);
        self.buf.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // flags
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // stored
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // time
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // date
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.buf
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // extra length
        self.buf.extend_from_slice(name);
        self.buf.extend_from_slice(data);
        self
    }

    fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buf.len() as u32;
        let mut central = Vec::new();
        let mut count = 0u16;

        let mut i = 0usize;
        while i + 30 <= self.buf.len() && self.buf[i..i + 4] == [0x50, 0x4b, 0x03, 0x04] {
            let rd16 = |at: usize| u16::from_le_bytes(self.buf[at..at + 2].try_into().unwrap());
            let rd32 = |at: usize| u32::from_le_bytes(self.buf[at..at + 4].try_into().unwrap());

            let crc = rd32(i + 14);
            let csize = rd32(i + 18) as usize;
            let usize_ = rd32(i + 22) as usize;
            let nlen = rd16(i + 26) as usize;
            let elen = rd16(i + 28) as usize;
            let name = self.buf[i + 30..i + 30 + nlen].to_vec();

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&0u16.to_le_bytes()); // flags
            central.extend_from_slice(&0u16.to_le_bytes()); // stored
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(csize as u32).to_le_bytes());
            central.extend_from_slice(&(usize_ as u32).to_le_bytes());
            central.extend_from_slice(&(nlen as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra
            central.extend_from_slice(&0u16.to_le_bytes()); // comment
            central.extend_from_slice(&0u16.to_le_bytes()); // disk
            central.extend_from_slice(&0u16.to_le_bytes()); // internal
            central.extend_from_slice(&0u32.to_le_bytes()); // external
            central.extend_from_slice(&(i as u32).to_le_bytes()); // local offset
            central.extend_from_slice(&name);
            count += 1;

            i = (i + 30 + nlen + elen + csize).max(i + 4);
        }

        let cd_size = central.len() as u32;
        self.buf.extend_from_slice(&central);
        self.buf.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());
        self.buf.extend_from_slice(&count.to_le_bytes());
        self.buf.extend_from_slice(&count.to_le_bytes());
        self.buf.extend_from_slice(&cd_size.to_le_bytes());
        self.buf.extend_from_slice(&cd_offset.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());
        self.buf
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn write_fixture(name: &str, bytes: &[u8]) -> PathBuf {
    let path = scratch_dir().join(name);
    let mut f = std::fs::File::create(&path).expect("create fixture");
    f.write_all(bytes).expect("write fixture");
    path
}

fn read_fixture(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("read fixture")
}

fn find(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len())
        .position(|w| w == needle)
        .expect("needle present")
}

fn open(path: &Path) -> Result<Archive, core::ArchiveError> {
    Archive::open(path)
}

// ── Fixture 1: line_bomb ────────────────────────────────────────────────────

/// A single line far larger than `MAX_LINE_BYTES` must produce
/// `LineTooLong` with the **true** length, and must not retain the tail.
#[test]
fn line_bomb_reports_true_length_and_stays_bounded() {
    let bomb_len = caps::MAX_LINE_BYTES as usize + 4096;
    let data: Vec<u8> = std::iter::repeat_n(b'x', bomb_len)
        .chain(std::iter::once(b'\n'))
        .collect();

    let path = write_fixture(
        "line_bomb.zip",
        &ZipBuilder::new().add("bomb.txt", &data).finish(),
    );
    let archive = open(&path).expect("archive opens");
    let id = archive.id_for_path("bomb.txt").expect("entry present");

    let slice = archive
        .reader(id)
        .expect("reader")
        .read_line(1)
        .expect("line 1 reads");

    assert_eq!(slice.byte_len_full, bomb_len as u64, "true length recorded");
    assert_eq!(
        slice.bytes.len() as u64,
        caps::MAX_LINE_BYTES,
        "retained exactly the cap, not the whole line"
    );
    match slice.outcome {
        core::ExcerptOutcome::LineTooLong { capped_at, skipped } => {
            assert_eq!(capped_at, caps::MAX_LINE_BYTES);
            assert_eq!(skipped, 4096);
        }
        other => panic!("expected LineTooLong, got {other:?}"),
    }
}

/// The measured proof of boundedness: the retained buffer is the cap while the
/// line is cap + 4096. An implementation that buffered the line would report
/// `skipped: 0` here.
#[test]
fn line_bomb_retention_is_independent_of_line_length() {
    let base = caps::MAX_LINE_BYTES as usize;
    let mut retentions = Vec::new();
    for extra in [4096usize, 64 * 1024, 512 * 1024] {
        let data: Vec<u8> = std::iter::repeat_n(b'x', base + extra)
            .chain(std::iter::once(b'\n'))
            .collect();
        let path = write_fixture(
            &format!("bomb_{extra}.zip"),
            &ZipBuilder::new().add("b.txt", &data).finish(),
        );
        let archive = open(&path).expect("opens");
        let id = archive.id_for_path("b.txt").unwrap();
        let slice = archive.reader(id).unwrap().read_line(1).unwrap();
        assert_eq!(slice.byte_len_full, (base + extra) as u64);
        retentions.push(slice.bytes.len());
    }
    // 16× the line length, one retained size.
    assert!(
        retentions.windows(2).all(|w| w[0] == w[1]),
        "retention must not grow with line length: {retentions:?}"
    );
    assert_eq!(retentions[0], base);
}

/// A line exactly at the cap is complete, not truncated.
#[test]
fn line_at_exactly_the_cap_is_complete() {
    let n = caps::MAX_LINE_BYTES as usize;
    let data: Vec<u8> = std::iter::repeat_n(b'y', n)
        .chain(std::iter::once(b'\n'))
        .collect();
    let path = write_fixture(
        "line_at_cap.zip",
        &ZipBuilder::new().add("at.txt", &data).finish(),
    );
    let archive = open(&path).expect("opens");
    let id = archive.id_for_path("at.txt").unwrap();

    let slice = archive.reader(id).unwrap().read_line(1).unwrap();
    assert_eq!(slice.byte_len_full, n as u64);
    assert_eq!(slice.bytes.len(), n);
    assert_eq!(slice.outcome, core::ExcerptOutcome::Complete);
}

/// Reading line 3 after a bomb on line 2 must still land on line 3: the skip
/// counted the bomb's newline rather than losing track of it.
#[test]
fn skip_past_a_bomb_line_still_lands_correctly() {
    let bomb_len = caps::MAX_LINE_BYTES as usize + 4096;
    let mut data = b"first\n".to_vec();
    data.extend(std::iter::repeat_n(b'x', bomb_len));
    data.push(b'\n');
    data.extend_from_slice(b"third\n");

    let path = write_fixture(
        "bomb_middle.zip",
        &ZipBuilder::new().add("m.txt", &data).finish(),
    );
    let archive = open(&path).expect("opens");
    let id = archive.id_for_path("m.txt").unwrap();
    let mut reader = archive.reader(id).unwrap();

    let third = reader.read_line(3).expect("line 3");
    assert_eq!(third.line_no, 3);
    assert_eq!(third.bytes, b"third");
    // The offset accounts for the whole bomb, not the capped prefix.
    let expected = 6 + bomb_len as u64 + 1;
    assert_eq!(third.byte_offset, expected, "offset tracks true length");
}

// ── Fixture 2: nul_bytes ───────────────────────────────────────────────────

#[test]
fn nul_bytes_become_replacement_char_and_never_break_the_read() {
    // b"before\0\0\0\0after" — 6 + 4 + 5 = 15 bytes before the newline.
    let data = b"before\0\0\0\0after\n".to_vec();
    let path = write_fixture("nul.zip", &ZipBuilder::new().add("n.txt", &data).finish());
    let archive = open(&path).expect("opens");
    let id = archive.id_for_path("n.txt").unwrap();

    let slice = archive.reader(id).unwrap().read_line(1).unwrap();
    assert_eq!(slice.byte_len_full, 15);

    let excerpt = slice.to_excerpt();
    assert!(
        !excerpt.text.contains('\u{0}'),
        "a raw NUL must never reach a JSON string"
    );
    assert_eq!(excerpt.text.matches('\u{FFFD}').count(), 4);
}

// ── Fixture 3: lying header / oversized stream ─────────────────────────────

/// Oversized declared metadata is rejected without inspecting payload bytes.
#[test]
fn declared_oversized_entry_is_rejected_before_payload_read() {
    let real = vec![b'z'; 300 * 1024];
    let path = write_fixture(
        "lying.zip",
        &ZipBuilder::new().add("lie.txt", &real).finish(),
    );

    let limits = ArchiveLimits {
        max_entry_bytes: 280 * 1024,
        ..ArchiveLimits::DEFAULT
    };
    match Archive::open_with(&path, limits) {
        Err(core::ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge { .. })) => {}
        other => panic!("expected open-time rejection, got {other:?}"),
    }
}

/// A ceiling below the declared size is rejected at open, before any read.
#[test]
fn entry_over_the_declared_ceiling_is_rejected_at_open() {
    let real = vec![b'z'; 300 * 1024];
    let path = write_fixture(
        "lying2.zip",
        &ZipBuilder::new().add("lie.txt", &real).finish(),
    );

    let limits = ArchiveLimits {
        max_entry_bytes: 256 * 1024,
        ..ArchiveLimits::DEFAULT
    };
    match Archive::open_with(&path, limits) {
        Err(core::ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge {
            limit: 262_144,
            ..
        })) => {}
        other => panic!("expected open-time rejection, got {other:?}"),
    }
}

#[test]
fn take_bounded_overflow_is_an_error_not_a_truncation() {
    let data = b"0123456789\n".to_vec();
    let path = write_fixture(
        "bounded.zip",
        &ZipBuilder::new().add("b.txt", &data).finish(),
    );
    let archive = open(&path).expect("opens");
    let id = archive.id_for_path("b.txt").unwrap();

    let mut reader = archive.reader(id).unwrap();
    reader.take_bounded(4);
    let err = reader.read_line(1).unwrap_err();
    match err {
        core::ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge { limit: 4, .. }) => {}
        other => panic!("expected EntryTooLarge, got {other:?}"),
    }
}

// ── Fixture 4: too_many_entries ────────────────────────────────────────────

#[test]
fn too_many_entries_is_rejected_at_open() {
    let mut b = ZipBuilder::new();
    for i in 0..12 {
        b = b.add(&format!("e{i}.txt"), b"x");
    }
    let path = write_fixture("many.zip", &b.finish());

    let limits = ArchiveLimits {
        max_entries: 10,
        ..ArchiveLimits::DEFAULT
    };
    match Archive::open_with(&path, limits).unwrap_err() {
        core::ArchiveError::LimitExceeded(LimitViolation::TooManyEntries { limit: 10, .. }) => {}
        other => panic!("expected TooManyEntries, got {other:?}"),
    }
}

// ── Fixture 5: corrupt payload ─────────────────────────────────────────────

/// The important one: it catches an implementation that returns `Empty` when it
/// actually failed to read.
#[test]
fn corrupt_entry_is_listable_but_unreadable_never_empty() {
    let path = write_fixture(
        "corrupt.zip",
        &ZipBuilder::new().add("c.txt", b"hello\n").finish(),
    );
    let mut bytes = read_fixture(&path);
    let at = find(&bytes, b"hello");
    bytes[at] = 0x00;
    bytes[at + 1] = 0xff;
    let path = write_fixture("corrupt2.zip", &bytes);

    let archive = open(&path).expect("index builds from the central directory");
    let id = archive.id_for_path("c.txt").expect("entry still listable");
    assert_eq!(archive.entry(id).map(|e| e.path.as_str()), Some("c.txt"));

    let result = archive.reader(id).unwrap().read_line(1);
    assert!(
        result.is_err(),
        "a corrupt payload must not read cleanly, and must not look like an empty result"
    );
}

// ── Fixture 6: non-UTF-8 name ──────────────────────────────────────────────

#[test]
fn non_utf8_name_is_listable_and_addressable_by_id() {
    let name = b"bad\xffname.txt";
    let path = write_fixture(
        "nonutf8.zip",
        &ZipBuilder::new().add_raw(name, b"data\n").finish(),
    );
    let archive = open(&path).expect("opens");

    let meta = &archive.entries()[0];
    assert!(
        !meta.path_bytes_valid_utf8,
        "raw bytes were not valid UTF-8, and that is recorded"
    );
    assert_eq!(meta.path_bytes_len, name.len());
    assert!(archive.entry(meta.id).is_some(), "still listable");
    assert_eq!(
        archive
            .reader(meta.id)
            .unwrap()
            .read_line(1)
            .unwrap()
            .byte_len_full,
        4,
        "addressable by id, never by path"
    );
}

// ── Fixture 7: encrypted entry ─────────────────────────────────────────────

#[test]
fn encrypted_entry_is_reported_not_silently_empty() {
    let path = write_fixture(
        "enc.zip",
        &ZipBuilder::new().add("e.txt", b"secret\n").finish(),
    );
    let mut bytes = read_fixture(&path);
    let local = find(&bytes, &[0x50, 0x4b, 0x03, 0x04]);
    bytes[local + 6] |= 0x01;
    let central = find(&bytes, &[0x50, 0x4b, 0x01, 0x02]);
    bytes[central + 8] |= 0x01;
    let path = write_fixture("enc2.zip", &bytes);

    let archive = open(&path).expect("index builds");
    let meta = &archive.entries()[0];
    assert!(meta.encrypted, "encryption recorded at index time");

    let err = archive.reader(meta.id).unwrap().read_line(1).unwrap_err();
    match err {
        core::ArchiveError::EntryEncrypted { .. } => {}
        other => panic!("expected EntryEncrypted, got {other:?}"),
    }
    assert_eq!(
        core::ArchiveStatus::from_error(&err),
        core::ArchiveStatus::Unreadable,
        "encrypted maps to Unreadable, never Empty"
    );
}

// ── Fixture 8: nested archives ─────────────────────────────────────────────

#[test]
fn nested_archive_is_detected_and_never_opened() {
    let path = write_fixture(
        "nested.zip",
        &ZipBuilder::new()
            .add("inner.gz", &[0x1f, 0x8b, 0x08, 0x00, 0x00])
            .add("inner.zip", b"PK\x03\x04rest")
            .finish(),
    );
    let archive = open(&path).expect("opens");

    let gz = archive.id_for_path("inner.gz").unwrap();
    let report = archive.detect(gz).expect("detects");
    assert_eq!(report.nested, Some(core::NestedKind::Gzip));
    assert!(!report.is_text);

    let zip_id = archive.id_for_path("inner.zip").unwrap();
    assert_eq!(
        archive.detect(zip_id).unwrap().nested,
        Some(core::NestedKind::Zip)
    );
}

// ── Fixture 9: no version.txt ──────────────────────────────────────────────

#[test]
fn absent_version_txt_reports_archive_format_0() {
    let path = write_fixture("nover.zip", &ZipBuilder::new().add("a.txt", b"x").finish());
    let archive = open(&path).expect("opens");
    let identity = archive.identity(false).expect("identity");

    assert_eq!(identity.format_version, None);
    assert!(identity.notes.contains(&core::ArchiveNote::ArchiveFormat0));
}

// ── Fixture 10: compression ratio boundary ─────────────────────────────────

#[test]
fn compression_ratio_limit_is_checked_at_open() {
    let data = std::iter::repeat_n(b'q', 4096).collect::<Vec<u8>>();
    let path = write_fixture("ratio.zip", &ZipBuilder::new().add("r.txt", &data).finish());

    // Stored entries are 1:1, so a limit of 1 is exactly the boundary: pass.
    let at_boundary = ArchiveLimits {
        max_compression_ratio: 1,
        ..ArchiveLimits::DEFAULT
    };
    assert!(Archive::open_with(&path, at_boundary).is_ok());

    // A limit of 0 is unsatisfiable for any non-empty entry: reject.
    let impossible = ArchiveLimits {
        max_compression_ratio: 0,
        ..ArchiveLimits::DEFAULT
    };
    assert!(Archive::open_with(&path, impossible).is_err());
}

#[test]
fn nonsensical_limit_sets_are_rejected_at_open() {
    let path = write_fixture("limits.zip", &ZipBuilder::new().add("a.txt", b"x").finish());
    let inverted = ArchiveLimits {
        max_entry_bytes: 4096,
        max_total_bytes: 1024, // entry budget above total budget
        ..ArchiveLimits::DEFAULT
    };
    assert!(matches!(
        Archive::open_with(&path, inverted),
        Err(core::ArchiveError::InvalidLimits { .. })
    ));
}

// ── Segment-wise glob, through the public API ─────────────────────────────

#[test]
fn find_by_glob_does_not_cross_separators() {
    let mut b = ZipBuilder::new();
    b = b.add("anr/foo.txt", b"1");
    b = b.add("anr/sub/bar.txt", b"2");
    b = b.add("anr/sub/deeper/baz.txt", b"3");
    b = b.add("other/qux.txt", b"4");
    let path = write_fixture("glob.zip", &b.finish());
    let archive = open(&path).expect("opens");

    let matched: Vec<String> = archive
        .find_by_glob("anr/*")
        .expect("glob compiles")
        .into_iter()
        .map(|id| archive.entry(id).unwrap().path.clone())
        .collect();

    assert_eq!(matched, vec!["anr/foo.txt".to_string()]);
    assert!(
        !matched.iter().any(|p| p.contains("sub")),
        "`*` must not cross `/`"
    );

    // `**` is the only construct that does cross.
    let deep: Vec<String> = archive
        .find_by_glob("anr/**")
        .expect("glob compiles")
        .into_iter()
        .map(|id| archive.entry(id).unwrap().path.clone())
        .collect();
    assert_eq!(deep.len(), 3);
}

// ── Regex mode is a typed refusal ─────────────────────────────────────────

#[test]
fn regex_mode_is_refused_not_silently_downgraded() {
    let path = write_fixture("regex.zip", &ZipBuilder::new().add("a.txt", b"x").finish());
    let archive = open(&path).expect("opens");

    let err = archive
        .scan(&core::ScanRequest::new("a.*b", PatternMode::Regex))
        .unwrap_err();
    match err {
        core::ArchiveError::PatternModeUnavailable {
            mode: PatternMode::Regex,
            ..
        } => {}
        other => panic!("expected PatternModeUnavailable, got {other:?}"),
    }
    assert_eq!(err.code(), "pattern_mode_unavailable");
}

// ── Empty is only reachable on real success ───────────────────────────────

#[test]
fn empty_versus_unreadable_are_never_conflated() {
    let path = write_fixture(
        "matrix.zip",
        &ZipBuilder::new()
            .add("hit.txt", b"needle here\n")
            .add("miss.txt", b"nothing here\n")
            .finish(),
    );
    let archive = open(&path).expect("opens");

    let hit = archive
        .scan(&core::ScanRequest::new("needle", PatternMode::Literal))
        .expect("scan runs");
    assert_eq!(hit.matches.len(), 1);
    assert_eq!(hit.matches[0].path, "hit.txt");

    let miss = archive
        .scan(&core::ScanRequest::new(
            "absent-token",
            PatternMode::Literal,
        ))
        .expect("scan runs");
    assert!(miss.matches.is_empty());
    assert_eq!(miss.stopped, None);
}

#[test]
fn scanning_a_missing_entry_id_is_an_error_not_an_empty_result() {
    let path = write_fixture(
        "scope.zip",
        &ZipBuilder::new().add("a.txt", b"x\n").finish(),
    );
    let archive = open(&path).expect("opens");

    let req =
        core::ScanRequest::new("x", PatternMode::Literal).with_scope(vec![core::EntryId(9_999)]);
    assert!(matches!(
        archive.scan(&req),
        Err(core::ArchiveError::EntryNotFound(_))
    ));
}
