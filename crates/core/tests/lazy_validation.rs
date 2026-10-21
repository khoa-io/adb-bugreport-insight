//! Lazy validation distinguishes metadata acceptance from inspected payload safety.

use std::io::{Cursor, Write};

use bugreport_insight_core::{Archive, ArchiveError, ArchiveLimits, EntryId, LimitViolation, caps};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

fn archive(name: &str, bytes: &[u8]) -> Archive {
    let path = scratch_dir().join(name);
    std::fs::write(&path, bytes).unwrap();
    Archive::open(path).unwrap()
}

#[test]
fn chunk_stream_preserves_every_byte_for_stored_and_deflated_entries() {
    let data: Vec<u8> = (0..caps::LINE_SCRATCH_BYTES * 2 + 19)
        .map(|i| (i.wrapping_mul(31) % 251) as u8)
        .collect();
    for (name, compression) in [
        ("lazy-stored.zip", zip::CompressionMethod::Stored),
        ("lazy-deflated.zip", zip::CompressionMethod::Deflated),
    ] {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "payload",
                zip::write::SimpleFileOptions::default().compression_method(compression),
            )
            .unwrap();
        writer.write_all(&data).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let archive = archive(name, &bytes);
        let mut reader = archive.reader(EntryId(0)).unwrap();
        let chunks = reader
            .stream()
            .take(4)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.len() <= caps::LINE_SCRATCH_BYTES)
        );
        assert!(
            chunks.concat() == data,
            "chunks must preserve exact payload bytes"
        );
    }
}

#[test]
fn chunk_stream_reports_overflow_instead_of_eof_and_accepts_exact_limit() {
    let data = b"0123456789";
    let archive = archive(
        "lazy-chunk-limits.zip",
        &ZipBuilder::new().add("payload", data).finish(),
    );
    for limit in [0, 4, 9, 10] {
        let mut reader = archive.reader(EntryId(0)).unwrap();
        reader.take_bounded(limit);
        let result = reader.stream().collect::<Result<Vec<_>, _>>();
        if limit < data.len() as u64 {
            let error = result.expect_err("a byte ceiling must not become successful EOF");
            assert!(matches!(
                error
                    .get_ref()
                    .and_then(|e| e.downcast_ref::<ArchiveError>()),
                Some(ArchiveError::LimitExceeded(
                    LimitViolation::EntryTooLarge { .. }
                ))
            ));
        } else {
            assert_eq!(result.unwrap().concat(), data);
        }
    }
}

#[test]
fn lying_declared_size_is_accepted_at_open_but_rejected_when_read() {
    let mut bytes = ZipBuilder::new().add("payload", &[b'x'; 32]).finish();
    let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    bytes[22..26].copy_from_slice(&4u32.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&4u32.to_le_bytes());
    let path = scratch_dir().join("lazy-lying-size.zip");
    std::fs::write(&path, bytes).unwrap();
    let archive = Archive::open_with(
        path,
        ArchiveLimits {
            max_entry_bytes: 8,
            ..ArchiveLimits::DEFAULT
        },
    )
    .expect("open inspects declared metadata, not the payload");
    for operation in 0..3 {
        let mut reader = archive.reader(EntryId(0)).unwrap();
        let result = match operation {
            0 => reader.read_line(1).map(|_| ()),
            1 => reader.read_line_span(1, 1).map(|_| ()),
            _ => reader.for_each_line(|_| Ok(true)).map(|_| ()),
        };
        assert!(matches!(
            result,
            Err(ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge {
                limit: 8,
                ..
            }))
        ));
    }
    let mut reader = archive.reader(EntryId(0)).unwrap();
    assert!(reader.stream().collect::<Result<Vec<_>, _>>().is_err());
}

#[test]
fn skipped_bytes_and_lines_count_toward_the_read_ceiling() {
    let archive = archive(
        "lazy-skipped-prefix.zip",
        &ZipBuilder::new().add("payload", b"a\nb\nc\nd\n").finish(),
    );
    for operation in 0..4 {
        let mut reader = archive.reader(EntryId(0)).unwrap();
        reader.take_bounded(4);
        let result = match operation {
            0 => reader.read_byte_range(6, 1).map(|_| ()),
            1 => reader.read_byte_range(0, 5).map(|_| ()),
            2 => reader.read_line(4).map(|_| ()),
            _ => reader.read_line_span(4, 4).map(|_| ()),
        };
        assert!(matches!(
            result,
            Err(ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge {
                limit: 4,
                ..
            }))
        ));
    }
}

#[test]
fn declared_total_and_stored_file_limits_accept_boundary_and_reject_above() {
    let bytes = ZipBuilder::new()
        .add("a", b"1234")
        .add("b", b"5678")
        .finish();
    let path = scratch_dir().join("lazy-open-boundaries.zip");
    std::fs::write(&path, &bytes).unwrap();
    let limits = ArchiveLimits {
        max_entry_bytes: 4,
        max_total_bytes: 8,
        max_archive_file_bytes: bytes.len() as u64,
        ..ArchiveLimits::DEFAULT
    };
    Archive::open_with(&path, limits).unwrap();
    assert!(matches!(
        Archive::open_with(
            &path,
            ArchiveLimits {
                max_total_bytes: 7,
                ..limits
            }
        ),
        Err(ArchiveError::LimitExceeded(
            LimitViolation::TotalTooLarge { .. }
        ))
    ));
    assert!(matches!(
        Archive::open_with(
            &path,
            ArchiveLimits {
                max_archive_file_bytes: bytes.len() as u64 - 1,
                ..limits
            }
        ),
        Err(ArchiveError::LimitExceeded(
            LimitViolation::FileTooLarge { .. }
        ))
    ));
}

#[test]
fn stopping_after_first_line_does_not_validate_corrupt_unread_tail() {
    let mut data = vec![b'x'; caps::LINE_SCRATCH_BYTES * 2];
    data[..3].copy_from_slice(b"ok\n");
    let mut bytes = ZipBuilder::new().add("payload", &data).finish();
    let payload_start = 30 + "payload".len();
    bytes[payload_start + data.len() - 1] ^= 1;
    let archive = archive("lazy-corrupt-tail.zip", &bytes);
    let mut reader = archive.reader(EntryId(0)).unwrap();
    assert_eq!(reader.read_line(1).unwrap().bytes, b"ok");
    let mut reader = archive.reader(EntryId(0)).unwrap();
    assert_eq!(reader.for_each_line(|_| Ok(false)).unwrap(), 1);
    let mut reader = archive.reader(EntryId(0)).unwrap();
    assert!(reader.stream().collect::<Result<Vec<_>, _>>().is_err());
}
