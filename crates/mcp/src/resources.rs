//! Read-only access to the complete artifact set for one bound archive.
//! No protocol types or artifact generation belong in this provider.
use bugreport_insight_core::{Archive, ArchiveError};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;
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
    parent: Result<File, Arc<std::io::Error>>,
    root_name: std::ffi::OsString,
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
        let parent_handle = File::open(parent)
            .and_then(|file| {
                if file.metadata()?.is_dir() {
                    Ok(file)
                } else {
                    Err(std::io::Error::other("archive parent is not a directory"))
                }
            })
            .map_err(Arc::new);
        Self {
            archive,
            parent: parent_handle,
            root_name: name,
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

    fn output_directory(&self) -> Result<Option<File>, ResourceError> {
        let parent = self
            .parent
            .as_ref()
            .map_err(|error| ResourceError::Io(std::io::Error::other(error.to_string())))?;
        let Some(root) = open_directory_at(parent, &self.root_name)? else {
            return Ok(None);
        };
        open_directory_at(&root, std::ffi::OsStr::new("insight"))
    }

    fn file_size(directory: &File, name: &str) -> Result<u64, ResourceError> {
        let file = open_file_at(directory, name)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
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
        let directory = self
            .output_directory()?
            .ok_or_else(|| ResourceError::NotFound(name.to_owned()))?;
        read_file_text(&directory, name)
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
        let Some(directory) = self.output_directory()? else {
            return Ok(Vec::new());
        };
        let sizes: Vec<_> = FILES
            .iter()
            .map(|name| Self::file_size(&directory, name))
            .collect::<Result<_, _>>()?;
        let digest = self.digest()?;
        let system = read_file_text(&directory, "system.json")?;
        self.validate_json("system.json", &system, &digest)?;
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

#[cfg(unix)]
fn open_directory_at(parent: &File, name: &std::ffi::OsStr) -> Result<Option<File>, ResourceError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| ResourceError::Invalid("invalid artifact directory name".into()))?;
    // SAFETY: the name is NUL-terminated and the parent descriptor remains
    // alive for the call. O_NOFOLLOW makes the opened handle authoritative.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            Some(libc::ELOOP) | Some(libc::ENOTDIR) => Err(ResourceError::Invalid(
                "artifact directory must be a regular directory, not a symlink".into(),
            )),
            _ => Err(error.into()),
        };
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(Some(unsafe { File::from_raw_fd(fd) }))
}

#[cfg(unix)]
fn open_file_at(directory: &File, name: &str) -> Result<File, ResourceError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let name = std::ffi::CString::new(name)
        .map_err(|_| ResourceError::Invalid("invalid artifact name".into()))?;
    // SAFETY: the name is NUL-terminated and the directory descriptor remains
    // alive for the call. O_NOFOLLOW rejects a leaf replaced by a symlink.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ELOOP) {
            return Err(ResourceError::Invalid(format!(
                "{name:?} is not a regular artifact file"
            )));
        }
        return Err(error.into());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(not(unix))]
fn open_directory_at(
    _parent: &File,
    _name: &std::ffi::OsStr,
) -> Result<Option<File>, ResourceError> {
    Err(ResourceError::Invalid(
        "race-safe artifact access is unavailable on this platform".into(),
    ))
}

#[cfg(not(unix))]
fn open_file_at(_directory: &File, _name: &str) -> Result<File, ResourceError> {
    Err(ResourceError::Invalid(
        "race-safe artifact access is unavailable on this platform".into(),
    ))
}

fn read_file_text(directory: &File, name: &str) -> Result<String, ResourceError> {
    let file = open_file_at(directory, name)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_RESOURCE_BYTES {
        return Err(ResourceError::Invalid(format!(
            "{name} is not a regular or size-bounded artifact file"
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn pinned_directory_and_nofollow_leaf_reject_replacements() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "resource-openat-race-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join(".report");
        let live_directory = root.join("insight");
        let outside = base.join("outside");
        std::fs::create_dir_all(&live_directory).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(live_directory.join("apps.json"), b"inside").unwrap();
        std::fs::write(outside.join("apps.json"), b"outside").unwrap();

        let parent = File::open(&base).unwrap();
        let root_handle = open_directory_at(&parent, std::ffi::OsStr::new(".report"))
            .unwrap()
            .unwrap();
        let pinned_directory = open_directory_at(&root_handle, std::ffi::OsStr::new("insight"))
            .unwrap()
            .unwrap();

        // Replace the checked path after its directory handle has been opened.
        // Reads remain rooted in the pinned directory that the provider checked.
        let moved_directory = root.join("saved-insight");
        std::fs::rename(&live_directory, &moved_directory).unwrap();
        symlink(&outside, &live_directory).unwrap();
        assert_eq!(
            read_file_text(&pinned_directory, "apps.json").unwrap(),
            "inside"
        );

        // A leaf swapped for a symlink after metadata inspection is rejected at
        // the single open operation; no pathname check/open gap is trusted.
        let size = InsightResources::file_size(&pinned_directory, "apps.json").unwrap();
        assert_eq!(size, 6);
        std::fs::remove_file(moved_directory.join("apps.json")).unwrap();
        symlink(outside.join("apps.json"), moved_directory.join("apps.json")).unwrap();
        assert!(matches!(
            open_file_at(&pinned_directory, "apps.json"),
            Err(ResourceError::Invalid(_))
        ));

        std::fs::remove_dir_all(base).unwrap();
    }
}
