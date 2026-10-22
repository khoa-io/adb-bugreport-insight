//! Windows handle-relative directory access.
//!
//! Unix opens children with `openat(dirfd, name, O_NOFOLLOW)`, so a child that
//! is replaced by a symlink after discovery cannot redirect a later read.
//! Windows has no `openat`, but the same guarantee is available in two pieces:
//!
//! 1. The directory is enumerated with `GetFileInformationByHandleEx` and
//!    `FileFullDirectoryInfo`, which reads the directory *through its handle*.
//!    A directory renamed after the session started is therefore still read as
//!    the object the handle names, not as whatever now occupies the original
//!    pathname.
//! 2. A child is opened with `FILE_FLAG_OPEN_REPARSE_POINT`, so a symlink or
//!    junction is opened as itself and is then refused by its reparse-point
//!    attribute instead of being silently followed.
//!
//! The child open is by name under the resolved directory path, so it is not
//! atomic against a replacement race the way `openat` is. The reparse-point
//! check closes the symlink case, which is the case the requirement names.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_HANDLE_EOF, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND,
    HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_FULL_DIR_INFO, FILE_GENERIC_READ, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileFullDirectoryInfo,
    FileFullDirectoryRestartInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
    OPEN_EXISTING,
};

const INVALID: HANDLE = INVALID_HANDLE_VALUE;

/// An owned Windows handle that is safe to move between threads.
///
/// `HANDLE` is a raw pointer, so it is neither `Send` nor `Sync`. The handle
/// is only ever used as an opaque token passed to Win32 calls, and those calls
/// are thread-safe, so the wrapper asserts that rather than forbidding it.
#[derive(Debug)]
struct OwnedHandle(HANDLE);

unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl OwnedHandle {
    /// Takes ownership of a handle returned by a Win32 call.
    ///
    /// # Safety
    ///
    /// The handle must be a fresh, uniquely owned handle that is not closed by
    /// any other owner.
    unsafe fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.0 != INVALID {
            // SAFETY: the handle is uniquely owned by this value.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// The share mode used for every handle here.
///
/// `FILE_SHARE_DELETE` is required: without it the handle would block a
/// directory rename, and the session must tolerate the directory it pinned
/// being renamed.
const SHARE_MODE: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;

/// A pinned directory handle.
///
/// The handle is the authority for the session: the directory is enumerated
/// through it, never through the pathname the session started with.
#[derive(Debug)]
pub(crate) struct PinnedDirectory {
    handle: OwnedHandle,
}

fn last_error() -> std::io::Error {
    std::io::Error::last_os_error()
}

/// Opens a directory handle that survives a rename of the directory.
pub(crate) fn open_directory(path: &Path) -> std::io::Result<PinnedDirectory> {
    let wide = wide_null(path.as_os_str());
    // SAFETY: `wide` is NUL-terminated and lives for the call.
    //
    // `FILE_LIST_DIRECTORY` is required: the directory is read through this
    // handle with `GetFileInformationByHandleEx`, not through a pathname.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            SHARE_MODE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID {
        return Err(last_error());
    }
    // SAFETY: `CreateFileW` returned a fresh, uniquely owned handle.
    let handle = unsafe { OwnedHandle::new(handle) };
    Ok(PinnedDirectory { handle })
}

impl PinnedDirectory {
    /// The path the pinned handle actually names.
    ///
    /// After a rename this is the new name, so a later child open addresses the
    /// same directory the session pinned rather than a replacement planted at
    /// the original path.
    fn resolved_path(&self) -> std::io::Result<PathBuf> {
        let mut buffer = vec![0u16; 512];
        loop {
            // SAFETY: the handle is live and `buffer` is a valid writable slice.
            let length = unsafe {
                windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(
                    self.handle.raw(),
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    0,
                )
            };
            if length == 0 {
                return Err(last_error());
            }
            if length as usize >= buffer.len() {
                buffer.resize(length as usize + 1, 0);
                continue;
            }
            let resolved = &buffer[..length as usize];
            // `GetFinalPathNameByHandleW` returns a `\\?\` path. The `\\?\`
            // prefix disables path parsing, so it is kept for the opens below.
            return Ok(PathBuf::from(OsString::from_wide(resolved)));
        }
    }

    /// The immediate children of the pinned directory, read through its handle.
    ///
    /// Subdirectories and reparse points are excluded, matching the Unix
    /// implementation's candidate filter. The caller bounds the result.
    pub(crate) fn entries(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut names = Vec::new();
        // The path is resolved from the handle on every use, so a directory
        // renamed since the session started is still read at its new name.
        let root = self.resolved_path()?;
        // `FILE_FULL_DIR_INFO` is a variable-length record, so the buffer is
        // sized for a batch and the entries are walked through
        // `NextEntryOffset`.
        let mut buffer = vec![0u8; 64 * 1024];
        let mut restart = true;
        loop {
            // SAFETY: `buffer` is a valid writable slice of the declared length.
            //
            // The first call uses the `Restart` class so the enumeration starts
            // at the beginning of the directory; the `Info` class continues it
            // from where the previous call stopped.
            let ok = unsafe {
                GetFileInformationByHandleEx(
                    self.handle.raw(),
                    if restart {
                        FileFullDirectoryRestartInfo
                    } else {
                        FileFullDirectoryInfo
                    },
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                )
            };
            if ok == 0 {
                let error = last_error();
                let code = error.raw_os_error().unwrap_or(0) as u32;
                if code == ERROR_NO_MORE_FILES || code == ERROR_HANDLE_EOF {
                    break;
                }
                return Err(error);
            }
            restart = false;
            let mut offset = 0usize;
            while offset < buffer.len() {
                // SAFETY: the buffer holds `FILE_FULL_DIR_INFO` records; the
                // record header is a fixed layout and the name follows it.
                let record =
                    unsafe { &*(buffer.as_ptr().add(offset) as *const FILE_FULL_DIR_INFO) };
                let name_length = record.FileNameLength as usize;
                if !name_length.is_multiple_of(2) {
                    return Err(std::io::Error::other(
                        "directory entry name length is not UTF-16 aligned",
                    ));
                }
                // SAFETY: `FileName` immediately follows the fixed header, and
                // `FileNameLength` bytes of it are valid.
                let name = unsafe {
                    let start = std::ptr::addr_of!(record.FileName) as *const u16;
                    std::slice::from_raw_parts(start, name_length / 2)
                };
                let attributes = record.FileAttributes;
                if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0 {
                    names.push(root.join(OsString::from_wide(name)));
                }
                let next = record.NextEntryOffset as usize;
                if next == 0 {
                    break;
                }
                offset += next;
            }
        }
        Ok(names)
    }

    /// Opens a child directory of the pinned directory.
    ///
    /// Returns `None` when the child does not exist, and an error when it
    /// exists but is a reparse point rather than a real directory.
    pub(crate) fn open_child_directory(
        &self,
        name: &str,
    ) -> std::io::Result<Option<PinnedDirectory>> {
        let child = self.child_path(name)?;
        // `FILE_FLAG_OPEN_REPARSE_POINT` means a junction is opened as itself,
        // so the reparse-point attribute can refuse it rather than the open
        // silently following it to another directory.
        let handle = match self.open_child_handle(
            &child,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            FILE_FLAG_BACKUP_SEMANTICS,
        )? {
            Some(handle) => handle,
            None => return Ok(None),
        };
        // SAFETY: `CreateFileW` returned a fresh, uniquely owned handle, which
        // `PinnedDirectory` keeps. The attributes are read from the raw handle
        // so no second owner of the handle is created.
        let handle = unsafe { OwnedHandle::new(handle) };
        let attributes = attributes_of_handle(handle.raw())?;
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::other(
                "artifact directory must be a regular directory, not a reparse point",
            ));
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
            return Err(std::io::Error::other(
                "artifact directory must be a regular directory",
            ));
        }
        Ok(Some(PinnedDirectory { handle }))
    }

    /// The path of a child of the pinned directory.
    ///
    /// The parent path is resolved from the handle, so a directory renamed
    /// since the session started is addressed at its new name rather than at
    /// the pathname the session was bound to.
    fn child_path(&self, name: &str) -> std::io::Result<PathBuf> {
        // `PathBuf::push` replaces the path after a `\\?\` prefix, so the
        // separator and name are appended to the string instead.
        let mut child = self.resolved_path()?.as_os_str().to_os_string();
        child.push("\\");
        child.push(name);
        Ok(PathBuf::from(child))
    }

    /// Opens a child of the pinned directory, or `None` when it is missing.
    fn open_child_handle(
        &self,
        child: &Path,
        access: u32,
        extra_flags: u32,
    ) -> std::io::Result<Option<HANDLE>> {
        let wide = wide_null(child.as_os_str());
        // SAFETY: `wide` is NUL-terminated and lives for the call.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                access,
                SHARE_MODE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | extra_flags,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID {
            // A missing child is absence, not a failure; anything else is an
            // error the caller must see rather than read as "not present".
            let error = last_error();
            if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32)
                || error.raw_os_error() == Some(ERROR_PATH_NOT_FOUND as i32)
            {
                return Ok(None);
            }
            return Err(error);
        }
        Ok(Some(handle))
    }

    /// Opens a regular child file of the pinned directory.
    ///
    /// Returns `None` when the child is a reparse point, a directory, or is
    /// missing, so the caller can refuse it.
    pub(crate) fn open_child_file(&self, name: &str) -> std::io::Result<Option<std::fs::File>> {
        let child = self.child_path(name)?;
        let Some(handle) = self.open_child_handle(&child, FILE_GENERIC_READ, 0)? else {
            return Ok(None);
        };
        // SAFETY: `CreateFileW` returned a fresh, uniquely owned handle, and
        // `from_raw_handle` takes that ownership.
        let file = unsafe { std::fs::File::from_raw_handle(handle) };
        let attributes = attributes_of(&file)?;
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Ok(None);
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Ok(None);
        }
        if !file.metadata()?.is_file() {
            return Ok(None);
        }
        Ok(Some(file))
    }

    /// Opens a child of the pinned directory without following a reparse point.
    ///
    /// Returns `None` when the child is a reparse point, a directory, or is
    /// missing, so the caller can refuse it as an invalid candidate.
    pub(crate) fn open_child(&self, name: &str) -> std::io::Result<Option<std::fs::File>> {
        self.open_child_file(name)
    }
}

/// The file attributes of an open handle.
fn attributes_of(file: &std::fs::File) -> std::io::Result<u32> {
    use std::os::windows::io::AsRawHandle;
    attributes_of_handle(file.as_raw_handle() as HANDLE)
}

/// The file attributes of a raw handle.
fn attributes_of_handle(handle: HANDLE) -> std::io::Result<u32> {
    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `info` is a valid, fully initialized output buffer of the size
    // `GetFileInformationByHandle` writes.
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    if ok == 0 {
        return Err(last_error());
    }
    Ok(info.dwFileAttributes)
}

/// A NUL-terminated UTF-16 copy of an OS string.
fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(std::iter::once(0)).collect()
}
