//! Provider checks for REQ-MCP-07 without protocol types.
use bugreport_insight_core::{Archive, insight::generate_to};
use bugreport_insight_mcp::resources::{InsightResources, MAX_RESOURCE_BYTES, ResourceError};
use bugreport_insight_test_fixtures::insight_archive;
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

struct Case {
    root: PathBuf,
    archive: PathBuf,
    output: PathBuf,
    provider: InsightResources,
}
impl Case {
    fn new(generate: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "insight-resource-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let archive = root.join("report case.zip");
        fs::copy(insight_archive(), &archive).unwrap();
        let opened = Arc::new(Archive::open(&archive).unwrap());
        // Resource identity must survive a prior digest-declining archive_info call.
        opened.identity(false).unwrap();
        let provider = InsightResources::new(opened);
        let output = root.join(".report case/insight");
        if generate {
            generate_to(&archive).unwrap();
        }
        Self {
            root,
            archive,
            output,
            provider,
        }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn complete_set_returns_exact_bytes_mimes_sizes_and_identity_without_writes() {
    let c = Case::new(true);
    let before = fs::read(&c.archive).unwrap();
    let modified = fs::metadata(&c.archive).unwrap().modified().unwrap();
    let list = c.provider.list().unwrap();
    assert_eq!(list.len(), 7);
    let digest = Archive::open(&c.archive)
        .unwrap()
        .identity(true)
        .unwrap()
        .sha256
        .unwrap();
    for resource in list {
        assert_eq!(
            resource.uri,
            format!("bugreport-insight://{digest}/insight/{}", resource.name)
        );
        let bytes = fs::read(c.output.join(resource.name)).unwrap();
        let file_modified = fs::metadata(c.output.join(resource.name))
            .unwrap()
            .modified()
            .unwrap();
        let (metadata, text) = c.provider.read(&resource.uri).unwrap();
        assert_eq!(text.as_bytes(), bytes);
        assert_eq!(metadata.size, bytes.len() as u64);
        assert_eq!(
            resource.mime_type,
            if resource.name == "README.md" {
                "text/markdown"
            } else {
                "application/json"
            }
        );
        assert_eq!(
            file_modified,
            fs::metadata(c.output.join(resource.name))
                .unwrap()
                .modified()
                .unwrap()
        );
    }
    assert_eq!(before, fs::read(&c.archive).unwrap());
    assert_eq!(
        modified,
        fs::metadata(&c.archive).unwrap().modified().unwrap()
    );
    assert_eq!(fs::read_dir(&c.output).unwrap().count(), 7);
}

#[test]
fn absent_output_is_empty_and_later_publication_is_discovered_without_restart() {
    let c = Case::new(false);
    assert!(c.provider.list().unwrap().is_empty());
    assert!(!c.output.parent().unwrap().exists());
    generate_to(&c.archive).unwrap();
    assert_eq!(c.provider.list().unwrap().len(), 7);
}

#[test]
fn incomplete_output_and_foreign_system_identity_are_errors() {
    let c = Case::new(true);
    fs::remove_file(c.output.join("cpu.json")).unwrap();
    assert!(c.provider.list().is_err());
    fs::write(c.output.join("cpu.json"), b"{}").unwrap();
    fs::write(
        c.output.join("system.json"),
        b"{\"artifact\":\"system\",\"source\":{\"archive_sha256\":\"foreign\"}}",
    )
    .unwrap();
    assert!(matches!(c.provider.list(), Err(ResourceError::Invalid(_))));
}

#[test]
fn selected_json_and_readme_must_match_bound_archive() {
    let c = Case::new(true);
    let list = c.provider.list().unwrap();
    fs::write(
        c.output.join("apps.json"),
        b"{\"artifact\":\"apps\",\"source\":{\"archive_sha256\":\"foreign\"}}",
    )
    .unwrap();
    let apps = list.iter().find(|r| r.name == "apps.json").unwrap();
    assert!(matches!(
        c.provider.read(&apps.uri),
        Err(ResourceError::Invalid(_))
    ));
    fs::write(c.output.join("README.md"), b"wrong report").unwrap();
    assert!(matches!(
        c.provider.read(&list[0].uri),
        Err(ResourceError::Invalid(_))
    ));
}

#[test]
fn malformed_and_non_utf8_content_fail_without_partial_text() {
    let c = Case::new(true);
    let uri = c
        .provider
        .list()
        .unwrap()
        .into_iter()
        .find(|r| r.name == "apps.json")
        .unwrap()
        .uri;
    for bytes in [&b"not json"[..], &b"\xff\xfe"[..]] {
        fs::write(c.output.join("apps.json"), bytes).unwrap();
        assert!(matches!(
            c.provider.read(&uri),
            Err(ResourceError::Invalid(_))
        ));
    }
}

#[test]
fn over_limit_resource_is_refused_before_reading() {
    let c = Case::new(true);
    let file = fs::OpenOptions::new()
        .write(true)
        .open(c.output.join("apps.json"))
        .unwrap();
    file.set_len(MAX_RESOURCE_BYTES + 1).unwrap();
    assert!(matches!(c.provider.list(), Err(ResourceError::Invalid(_))));
}

#[test]
fn unknown_cross_archive_and_traversal_uris_are_refused() {
    let c = Case::new(true);
    let uri = c.provider.list().unwrap()[0].uri.clone();
    for invalid in [
        "file:///etc/passwd".into(),
        uri.replace("README.md", "../system.json"),
        uri.replace("README.md", "%2e%2e/system.json"),
        uri.replace("README.md", "README.md?raw=1"),
        uri.replacen("://", "://foreign", 1),
    ] {
        assert!(matches!(
            c.provider.read(&invalid),
            Err(ResourceError::NotFound(_))
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlink_files_and_directories_are_never_served() {
    use std::os::unix::fs::symlink;
    let c = Case::new(true);
    fs::remove_file(c.output.join("apps.json")).unwrap();
    symlink(&c.archive, c.output.join("apps.json")).unwrap();
    assert!(matches!(c.provider.list(), Err(ResourceError::Invalid(_))));
    let backup = c.root.join("saved");
    fs::rename(c.output.parent().unwrap(), &backup).unwrap();
    symlink(&backup, c.output.parent().unwrap()).unwrap();
    assert!(matches!(c.provider.list(), Err(ResourceError::Invalid(_))));
}
