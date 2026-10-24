//! Same-filesystem staging and atomic publication without replacement.
use super::{InsightError, InsightFile, generate_insight};
use crate::{Archive, ArchiveLimits};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_RENAME_INFO, FileRenameInfo,
    SetFileInformationByHandle,
};

#[derive(Debug)]
struct Stage {
    root: PathBuf,
    path: PathBuf,
    destination: PathBuf,
    root_created: bool,
    published: bool,
}
impl Stage {
    fn new(archive: &Path) -> Result<Self, InsightError> {
        let parent = archive
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let stem = archive
            .file_stem()
            .ok_or_else(|| InsightError::Invalid("archive filename has no stem".into()))?;
        let mut name = OsString::from(".");
        name.push(stem);
        let root = parent.join(name);
        let destination = root.join("insight");
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                return Err(InsightError::Invalid(format!(
                    "output already exists: {}",
                    destination.display()
                )));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let root_created = match fs::create_dir(&root) {
            Ok(()) => true,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let m = fs::symlink_metadata(&root)?;
                if !m.is_dir() || m.file_type().is_symlink() {
                    return Err(InsightError::Invalid(format!(
                        "output root is not a regular directory: {}",
                        root.display()
                    )));
                }
                false
            }
            Err(e) => return Err(e.into()),
        };
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..128 {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(".insight-stage-{}-{serial}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        root,
                        path,
                        destination,
                        root_created,
                        published: false,
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    if root_created {
                        let _ = fs::remove_dir(&root);
                    }
                    return Err(e.into());
                }
            }
        }
        if root_created {
            let _ = fs::remove_dir(&root);
        }
        Err(InsightError::Invalid(
            "unable to allocate a unique staging directory".into(),
        ))
    }
    fn write(&self, files: &[InsightFile]) -> Result<(), InsightError> {
        for file in files {
            if ![
                "README.md",
                "system.json",
                "apps.json",
                "anr.json",
                "crashes.json",
                "memory.json",
                "cpu.json",
            ]
            .contains(&file.name)
            {
                return Err(InsightError::Invalid("unexpected artifact filename".into()));
            }
            let mut out = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.path.join(file.name))?;
            out.write_all(&file.contents)?;
            out.sync_all()?;
        }
        // The staging directory is synced on Unix, where a directory handle
        // can be opened for fsync. Windows cannot open a directory this way,
        // and does not require the directory entry itself to be flushed.
        #[cfg(unix)]
        fs::File::open(&self.path)?.sync_all()?;
        Ok(())
    }
    fn publish(mut self, files: &[InsightFile]) -> Result<PathBuf, InsightError> {
        self.write(files)?;
        rename_exclusive(&self.path, &self.destination)?;
        self.published = true;
        Ok(self.destination.clone())
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
            if self.root_created {
                let _ = fs::remove_dir(&self.root);
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    // SAFETY: both C strings own live, NUL-terminated buffers for the duration
    // of this call. The flags request an atomic rename that refuses any target,
    // including an empty directory created after the initial collision check.
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
/// Renames `source` to `destination`, refusing an existing target.
///
/// `fs::rename` replaces an existing directory on Windows, which would
/// silently overwrite a previous analysis. `SetFileInformationByHandle` with
/// `FileRenameInfo` fails when the destination exists, which is the
/// fail-without-overwrite policy the publication contract requires.
#[cfg(target_os = "windows")]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;

    // A directory handle on Windows needs `FILE_FLAG_BACKUP_SEMANTICS`; without
    // it `CreateFileW` fails with `ERROR_ACCESS_DENIED`. `DELETE` is required
    // to rename through the handle.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .access_mode(DELETE)
        .open(source)?;
    // `SetFileInformationByHandle` needs an absolute path. The destination
    // does not exist yet, so its parent is canonicalized and the file name
    // joined back on. `canonicalize` yields a verbatim (`\\?\`) path, which
    // is what the rename call expects.
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("destination has no parent directory"))?
        .canonicalize()?;
    let name = parent.join(
        destination
            .file_name()
            .ok_or_else(|| io::Error::other("destination has no file name"))?,
    );
    let name = name
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    // `FILE_RENAME_INFO` is a variable-length record: a fixed header followed
    // by the NUL-terminated destination name at `FileName`'s own offset.
    const NAME_OFFSET: usize = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    let mut buffer = vec![0u8; NAME_OFFSET + name.len() * std::mem::size_of::<u16>()];
    // SAFETY: the buffer is at least as large as the fixed header.
    let info = unsafe { &mut *(buffer.as_mut_ptr() as *mut FILE_RENAME_INFO) };
    info.RootDirectory = std::ptr::null_mut();
    info.FileNameLength = (name.len() * std::mem::size_of::<u16>()) as u32;
    // SAFETY: `FileName` starts at `NAME_OFFSET` in the record, and the buffer
    // holds the full NUL-terminated name from that offset.
    unsafe {
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!(info.FileName) as *mut u16,
            name.len(),
        );
    }

    // SAFETY: `file` is a live handle and `buffer` is a valid, fully
    // initialized `FILE_RENAME_INFO` record of the declared size.
    let ok = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileRenameInfo,
            buffer.as_ptr().cast(),
            buffer.len() as u32,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn rename_exclusive(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace insight publication is unsupported on this platform",
    ))
}

/// Read, analyze, and atomically publish all seven artifacts beside the input.
/// Existing output is preserved even if another process creates it during analysis.
pub fn generate_to(archive_path: impl AsRef<Path>) -> Result<PathBuf, InsightError> {
    generate_to_with_limits(archive_path, ArchiveLimits::DEFAULT)
}

/// Publish with the selected admission/read policy. Only the exact
/// [`ArchiveLimits::FORCED`] profile also raises retained analysis budgets.
/// Integrity, atomic publication and response bounds remain unchanged.
pub fn generate_to_with_limits(
    archive_path: impl AsRef<Path>,
    limits: ArchiveLimits,
) -> Result<PathBuf, InsightError> {
    let path = archive_path.as_ref();
    let archive = Archive::open_with(path.to_path_buf(), limits)?;
    let stage = Stage::new(path)?;
    let files = generate_insight(&archive)?;
    stage.publish(&files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bugreport_insight_test_fixtures::scratch_dir;
    fn location() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        scratch_dir().join(format!(
            "publish-unit-{}-{}.zip",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn write_failure_removes_staging_and_leaves_no_final_directory() {
        let archive = location();
        let stage = Stage::new(&archive).unwrap();
        let root = stage.root.clone();
        let final_path = stage.destination.clone();
        fs::create_dir(stage.path.join("apps.json")).unwrap();
        let files = [
            InsightFile {
                name: "README.md",
                contents: b"ready".to_vec(),
            },
            InsightFile {
                name: "apps.json",
                contents: b"{}".to_vec(),
            },
        ];
        assert!(stage.publish(&files).is_err());
        assert!(!final_path.exists());
        assert!(!root.exists());
    }
    #[test]
    fn racing_destination_is_never_replaced() {
        let archive = location();
        let stage = Stage::new(&archive).unwrap();
        let root = stage.root.clone();
        let final_path = stage.destination.clone();
        fs::create_dir(&final_path).unwrap();
        fs::write(final_path.join("sentinel"), b"keep").unwrap();
        assert!(
            stage
                .publish(&[InsightFile {
                    name: "README.md",
                    contents: vec![]
                }])
                .is_err()
        );
        assert_eq!(fs::read(final_path.join("sentinel")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn even_a_racing_empty_destination_is_preserved() {
        let archive = location();
        let stage = Stage::new(&archive).unwrap();
        let root = stage.root.clone();
        let final_path = stage.destination.clone();
        fs::create_dir(&final_path).unwrap();
        assert!(stage.publish(&[]).is_err());
        assert!(final_path.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
