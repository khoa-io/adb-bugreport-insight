//! Bounded marker validation preserves identifier identity instead of clipped guesses.

use bugreport_insight_core::{Archive, ArchiveNote, EntryId, MarkerStatus};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

fn archive(name: &str, bytes: &[u8]) -> Archive {
    let path = scratch_dir().join(name);
    std::fs::write(&path, bytes).unwrap();
    Archive::open(path).unwrap()
}

#[test]
fn valid_markers_remove_one_line_ending_and_preserve_spaces_and_unicode() {
    let name = " café .txt ";
    let archive = archive(
        "marker-valid.zip",
        &ZipBuilder::new()
            .add("version.txt", b" 2 \r\n")
            .add("main_entry.txt", format!("{name}\r\n").as_bytes())
            .add(name, b"payload")
            .finish(),
    );
    let identity = archive.identity(false).unwrap();
    assert_eq!(identity.format_version.as_deref(), Some(" 2 "));
    assert_eq!(identity.main_entry.as_deref(), Some(name));
    assert_eq!(identity.main_entry_size_bytes, Some(7));
    assert_eq!(identity.markers.main_entry.status, MarkerStatus::Valid);
    assert_eq!(identity.markers.main_entry.entry_id, Some(EntryId(1)));
    assert_eq!(
        identity.markers.main_entry.source_bytes,
        Some(name.len() + 2)
    );
    assert!(identity.markers.main_entry.line_ending_removed);
    for (index, input, expected) in [(0, "2\n\n", "2\n"), (1, "2\r", "2\r"), (2, "", "")] {
        let a = archive_for_line_ending(index, input);
        assert_eq!(
            a.identity(false).unwrap().format_version.as_deref(),
            Some(expected)
        );
    }
}

fn archive_for_line_ending(index: usize, input: &str) -> Archive {
    archive(
        &format!("marker-ending-{index}.zip"),
        &ZipBuilder::new()
            .add("version.txt", input.as_bytes())
            .finish(),
    )
}

#[test]
fn marker_source_boundary_accepts_256_and_withholds_257_without_a_prefix() {
    for size in [255, 256, 257, 300] {
        let archive = archive(
            &format!("marker-size-{size}.zip"),
            &ZipBuilder::new()
                .add("version.txt", &vec![b'a'; size])
                .finish(),
        );
        let identity = archive.identity(false).unwrap();
        assert_eq!(
            identity.markers.version.declared_size_bytes,
            Some(size as u64)
        );
        assert_eq!(identity.markers.version.entry_id, Some(EntryId(0)));
        if size <= 256 {
            assert_eq!(identity.format_version, Some("a".repeat(size)));
            assert_eq!(identity.markers.version.status, MarkerStatus::Valid);
            assert_eq!(identity.markers.version.source_bytes, Some(size));
        } else {
            assert_eq!(identity.format_version, None);
            assert_eq!(identity.markers.version.status, MarkerStatus::Oversized);
            assert!(identity.markers.version.error_code.is_some());
            assert!(!identity.notes.contains(&ArchiveNote::ArchiveFormat0));
        }
    }
}

#[test]
fn missing_invalid_and_unreadable_markers_are_distinct_and_identity_survives() {
    let missing = archive(
        "marker-missing.zip",
        &ZipBuilder::new().add("other", b"ok").finish(),
    )
    .identity(false)
    .unwrap();
    assert_eq!(missing.markers.version.status, MarkerStatus::Missing);
    assert_eq!(missing.markers.version.entry_id, None);
    assert!(missing.notes.contains(&ArchiveNote::ArchiveFormat0));
    for (name, input, status) in [
        (
            "marker-invalid-utf8.zip",
            b"2\xff".as_slice(),
            MarkerStatus::InvalidUtf8,
        ),
        (
            "marker-nul.zip",
            b"2\0".as_slice(),
            MarkerStatus::ContainsNul,
        ),
    ] {
        let identity = archive(
            name,
            &ZipBuilder::new()
                .add("version.txt", input)
                .add("other", b"ok")
                .finish(),
        )
        .identity(false)
        .unwrap();
        assert_eq!(identity.entry_count, 2);
        assert_eq!(identity.format_version, None);
        assert_eq!(identity.markers.version.status, status);
        assert!(identity.markers.version.error_code.is_some());
        assert!(!identity.notes.contains(&ArchiveNote::ArchiveFormat0));
    }
    let mut bytes = ZipBuilder::new().add("version.txt", b"2\n").finish();
    bytes[30 + "version.txt".len()] ^= 1;
    let identity = archive("marker-corrupt.zip", &bytes)
        .identity(false)
        .unwrap();
    assert_eq!(identity.format_version, None);
    assert_eq!(identity.markers.version.status, MarkerStatus::Unreadable);
    assert_eq!(identity.markers.version.entry_id, Some(EntryId(0)));
}

#[test]
fn understated_marker_size_is_rejected_by_the_actual_stream_guard() {
    let mut bytes = ZipBuilder::new().add("version.txt", &[b'x'; 300]).finish();
    let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    bytes[22..26].copy_from_slice(&4u32.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&4u32.to_le_bytes());
    let identity = archive("marker-lying-size.zip", &bytes)
        .identity(false)
        .unwrap();
    assert_eq!(identity.markers.version.declared_size_bytes, Some(4));
    assert_eq!(identity.markers.version.status, MarkerStatus::Oversized);
    assert_eq!(identity.format_version, None);
    assert_eq!(identity.markers.version.source_bytes, None);

    let mut bytes = ZipBuilder::new().add("version.txt", b"12345678").finish();
    let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    bytes[22..26].copy_from_slice(&4u32.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&4u32.to_le_bytes());
    let path = scratch_dir().join("marker-smaller-reader-limit.zip");
    std::fs::write(&path, bytes).unwrap();
    let identity = Archive::open_with(
        path,
        bugreport_insight_core::ArchiveLimits {
            max_entry_bytes: 4,
            ..Default::default()
        },
    )
    .unwrap()
    .identity(false)
    .unwrap();
    assert_eq!(identity.markers.version.status, MarkerStatus::Unreadable);
    assert_eq!(identity.format_version, None);
    assert!(identity.markers.version.error_code.is_some());
}
