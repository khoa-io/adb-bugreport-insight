//! Explicit force raises finite admission limits and retains integrity checks.
use std::io::{Cursor, Write};

use bugreport_insight_core::{Archive, ArchiveError, ArchiveLimits, EntryId, LimitViolation};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

#[test]
fn force_admits_entry_counts_above_default_and_keeps_archive_read_only() {
    let mut builder = ZipBuilder::new();
    for i in 0..=ArchiveLimits::DEFAULT.max_entries {
        builder = builder.add(&format!("entry-{i}"), b"");
    }
    let bytes = builder.finish();
    let path = scratch_dir().join("force-many-entries.zip");
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        Archive::open(path.clone()),
        Err(ArchiveError::LimitExceeded(
            LimitViolation::TooManyEntries { .. }
        ))
    ));
    let archive = Archive::open_with(path.clone(), ArchiveLimits::FORCED).unwrap();
    assert_eq!(archive.entries().len(), 10_001);
    assert_eq!(archive.limits(), ArchiveLimits::FORCED);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    ArchiveLimits::FORCED.validate().unwrap();
}

#[test]
fn force_admits_and_reads_deflate_ratio_above_default() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "payload",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    let payload = vec![0; 4 * 1024 * 1024];
    writer.write_all(&payload).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let path = scratch_dir().join("force-ratio.zip");
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        Archive::open(path.clone()),
        Err(ArchiveError::LimitExceeded(
            LimitViolation::RatioExceeded { .. }
        ))
    ));
    let archive = Archive::open_with(path, ArchiveLimits::FORCED).unwrap();
    let result = archive
        .reader(EntryId(0))
        .unwrap()
        .stream()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(result.concat(), payload);
}

#[test]
fn force_does_not_suppress_crc_failures_or_narrowed_read_limits() {
    let mut bytes = ZipBuilder::new().add("payload", b"0123456789").finish();
    bytes[30 + "payload".len()] ^= 1;
    let path = scratch_dir().join("force-crc.zip");
    std::fs::write(&path, bytes).unwrap();
    let archive = Archive::open_with(path, ArchiveLimits::FORCED).unwrap();
    let mut reader = archive.reader(EntryId(0)).unwrap();
    assert!(reader.stream().collect::<Result<Vec<_>, _>>().is_err());
    let mut reader = archive.reader(EntryId(0)).unwrap();
    reader.take_bounded(4);
    assert!(reader.stream().collect::<Result<Vec<_>, _>>().is_err());
}
