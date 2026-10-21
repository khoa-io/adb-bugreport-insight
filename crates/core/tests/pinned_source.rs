//! A bound descriptor cannot be redirected by renames or symlink replacement.
use std::fs::File;

use bugreport_insight_core::{Archive, ArchiveLimits, EntryId};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
use sha2::{Digest, Sha256};

#[test]
fn from_file_reads_and_hashes_original_after_display_path_replacement() {
    let root = scratch_dir().join("pinned-source-replace");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("report.zip");
    let original = ZipBuilder::new()
        .add("main_entry.txt", b"bugreport.txt")
        .add("bugreport.txt", b"original report\n")
        .finish();
    std::fs::write(&path, &original).unwrap();
    let archive =
        Archive::from_file(File::open(&path).unwrap(), &path, ArchiveLimits::DEFAULT).unwrap();
    std::fs::rename(&path, root.join("original.zip")).unwrap();
    let outside = root.join("outside.zip");
    std::fs::write(
        &outside,
        ZipBuilder::new()
            .add("secret.txt", b"outside source")
            .finish(),
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    #[cfg(not(unix))]
    std::fs::copy(&outside, &path).unwrap();
    assert_eq!(
        archive
            .reader(EntryId(1))
            .unwrap()
            .read_line(1)
            .unwrap()
            .bytes,
        b"original report"
    );
    let identity = archive.identity(true).unwrap();
    let expected = format!("{:x}", Sha256::digest(&original));
    assert_eq!(identity.sha256.as_deref(), Some(expected.as_str()));
    assert_eq!(identity.main_entry.as_deref(), Some("bugreport.txt"));
    assert_eq!(identity.entry_count, 2);
    assert_eq!(identity.size_bytes, original.len() as u64);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ordinary_open_pins_readers_and_insight_digest_fallback_after_unlink() {
    let root = scratch_dir().join("pinned-source-unlink");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("report.zip");
    let bytes = ZipBuilder::new()
        .add("main_entry.txt", b"bugreport.txt")
        .add("bugreport.txt", b"original report\n")
        .finish();
    std::fs::write(&path, &bytes).unwrap();
    let archive = Archive::open(&path).unwrap();
    assert!(archive.identity(false).unwrap().sha256.is_none());
    std::fs::remove_file(&path).unwrap();
    let files = bugreport_insight_core::insight::generate_insight(&archive).unwrap();
    let system = files.iter().find(|f| f.name == "system.json").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&system.contents).unwrap();
    assert_eq!(
        value["source"]["archive_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(
        archive
            .reader(EntryId(1))
            .unwrap()
            .read_line(1)
            .unwrap()
            .bytes,
        b"original report"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_readers_and_hash_have_independent_seek_positions() {
    let path = scratch_dir().join("pinned-source-independent.zip");
    let first: Vec<u8> = (0..262_177).map(|n| (n % 251) as u8).collect();
    let second: Vec<u8> = (0..327_713).map(|n| (n % 199) as u8).collect();
    let bytes = ZipBuilder::new()
        .add("first", &first)
        .add("second", &second)
        .finish();
    std::fs::write(&path, &bytes).unwrap();
    let file = File::open(&path).unwrap();
    let archive =
        Archive::from_file(file.try_clone().unwrap(), &path, ArchiveLimits::DEFAULT).unwrap();
    let a = Archive::from_file(file.try_clone().unwrap(), &path, ArchiveLimits::DEFAULT).unwrap();
    let first_thread = std::thread::spawn(move || {
        a.reader(EntryId(0))
            .unwrap()
            .stream()
            .inspect(|_| std::thread::yield_now())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .concat()
    });
    let a = Archive::from_file(file, &path, ArchiveLimits::DEFAULT).unwrap();
    let second_thread = std::thread::spawn(move || {
        a.reader(EntryId(1))
            .unwrap()
            .stream()
            .inspect(|_| std::thread::yield_now())
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .concat()
    });
    assert_eq!(
        archive.identity(true).unwrap().sha256.unwrap(),
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(first_thread.join().unwrap(), first);
    assert_eq!(second_thread.join().unwrap(), second);
}
