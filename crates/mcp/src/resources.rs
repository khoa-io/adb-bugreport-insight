//! Read-only access to the complete artifact set for one bound archive.
//! No protocol types or artifact generation belong in this provider.
use bugreport_insight_core::{Archive, ArchiveError};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Full resources are returned or refused; never silently truncated.
pub const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;
const FILES: [&str; 7] = [
    "README.md",
    "system.json",
    "apps.json",
    "anr.json",
    "crashes.json",
    "memory.json",
    "cpu.json",
];

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("insight resource not found: {0}; generate with --insight BUGREPORT first")]
    NotFound(String),
    #[error("insight resource unavailable: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Archive(#[from] ArchiveError),
}

#[derive(Debug)]
pub struct InsightResource {
    pub name: &'static str,
    pub uri: String,
    pub mime_type: &'static str,
    pub size: u64,
}

/// Resources always refer to the archive supplied at server startup.
#[derive(Debug)]
pub struct InsightResources {
    archive: Arc<Archive>,
    root: PathBuf,
    directory: PathBuf,
    sha256: OnceLock<String>,
}
impl InsightResources {
    pub fn new(archive: Arc<Archive>) -> Self {
        let path = archive.path();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut name = std::ffi::OsString::from(".");
        name.push(path.file_stem().unwrap_or_default());
        let root = parent.join(name);
        let directory = root.join("insight");
        Self {
            archive,
            root,
            directory,
            sha256: OnceLock::new(),
        }
    }

    fn digest(&self) -> Result<String, ResourceError> {
        if let Some(digest) = self.sha256.get() {
            return Ok(digest.clone());
        }
        let mut identity = self.archive.identity(true)?;
        // Archive identity may have been cached by an earlier digest-declining tool call.
        if identity.sha256.is_none() {
            identity = self.archive.with_fresh_digest().identity(true)?;
        }
        let digest = identity
            .sha256
            .ok_or_else(|| ResourceError::Invalid("archive SHA-256 is unavailable".into()))?;
        let _ = self.sha256.set(digest.clone());
        Ok(digest)
    }

    fn present(&self) -> Result<bool, ResourceError> {
        for directory in [&self.root, &self.directory] {
            match fs::symlink_metadata(directory) {
                Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
                Ok(_) => {
                    return Err(ResourceError::Invalid(
                        "artifact directory must be a regular directory, not a symlink".into(),
                    ));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(true)
    }

    fn file_size(&self, name: &str) -> Result<u64, ResourceError> {
        let metadata = fs::symlink_metadata(self.directory.join(name))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(ResourceError::Invalid(format!(
                "{name} is not a regular artifact file"
            )));
        }
        if metadata.len() > MAX_RESOURCE_BYTES {
            return Err(ResourceError::Invalid(format!(
                "{name} exceeds the 64 MiB resource limit"
            )));
        }
        Ok(metadata.len())
    }

    fn read_text(&self, name: &str) -> Result<String, ResourceError> {
        self.file_size(name)?;
        let file = File::open(self.directory.join(name))?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_RESOURCE_BYTES {
            return Err(ResourceError::Invalid(format!(
                "{name} changed to an invalid or oversized file"
            )));
        }
        let mut bytes = Vec::new();
        file.take(MAX_RESOURCE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RESOURCE_BYTES {
            return Err(ResourceError::Invalid(format!(
                "{name} exceeds the 64 MiB resource limit"
            )));
        }
        String::from_utf8(bytes).map_err(|_| ResourceError::Invalid(format!("{name} is not UTF-8")))
    }

    fn validate_json(&self, name: &str, text: &str, digest: &str) -> Result<(), ResourceError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| ResourceError::Invalid(format!("{name} is not a JSON artifact")))?;
        if value["source"]["archive_sha256"].as_str() != Some(digest)
            || value["artifact"].as_str() != name.strip_suffix(".json")
        {
            return Err(ResourceError::Invalid(format!(
                "{name} does not identify this bound archive and artifact"
            )));
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<InsightResource>, ResourceError> {
        if !self.present()? {
            return Ok(Vec::new());
        }
        let sizes: Vec<_> = FILES
            .iter()
            .map(|name| self.file_size(name))
            .collect::<Result<_, _>>()?;
        let digest = self.digest()?;
        self.validate_json("system.json", &self.read_text("system.json")?, &digest)?;
        Ok(FILES
            .into_iter()
            .zip(sizes)
            .map(|(name, size)| InsightResource {
                name,
                uri: format!("bugreport-insight://{digest}/insight/{name}"),
                mime_type: if name == "README.md" {
                    "text/markdown"
                } else {
                    "application/json"
                },
                size,
            })
            .collect())
    }

    pub fn read(&self, uri: &str) -> Result<(InsightResource, String), ResourceError> {
        // Exact matching deliberately rejects traversal, queries, encoded paths and other archives.
        let resource = self
            .list()?
            .into_iter()
            .find(|r| r.uri == uri)
            .ok_or_else(|| ResourceError::NotFound(uri.into()))?;
        let text = self.read_text(resource.name)?;
        let digest = self.digest()?;
        if resource.name == "README.md" {
            if !text
                .lines()
                .any(|line| line == format!("SHA-256: `{digest}`"))
            {
                return Err(ResourceError::Invalid(
                    "README.md does not identify this bound archive".into(),
                ));
            }
        } else {
            self.validate_json(resource.name, &text, &digest)?;
        }
        // Reflect actual bytes if a regular file changed after the metadata listing.
        Ok((
            InsightResource {
                size: text.len() as u64,
                ..resource
            },
            text,
        ))
    }
}
