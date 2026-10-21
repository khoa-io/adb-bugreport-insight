//! Independent logical cursors over one pinned, read-only archive descriptor.
//! Unix and Windows use positional reads, including across cloned descriptors.
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

/// Each ZIP reader has its own position; the descriptor seek/read pair is
/// isolated so clones never share a logical seek position or reopen paths.
#[derive(Debug, Clone)]
pub(crate) struct FileCursor {
    file: Arc<Mutex<File>>,
    position: u64,
}
impl FileCursor {
    pub(crate) fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
            position: 0,
        }
    }
    pub(crate) fn fresh(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            position: 0,
        }
    }
    pub(crate) fn len(&self) -> io::Result<u64> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("archive source lock poisoned"))?
            .metadata()
            .map(|m| m.len())
    }
}
impl Read for FileCursor {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("archive source lock poisoned"))?;
        // Unix descriptors obtained by File::try_clone share their OS offset,
        // even across separately constructed Archive values and mutexes.
        // Positional reads keep those independently authorized bindings isolated.
        #[cfg(unix)]
        let n = {
            use std::os::unix::fs::FileExt;
            file.read_at(bytes, self.position)?
        };
        #[cfg(windows)]
        let n = {
            use std::os::windows::fs::FileExt;
            file.seek_read(bytes, self.position)?
        };
        #[cfg(not(any(unix, windows)))]
        let n = {
            let mut file = file;
            file.seek(SeekFrom::Start(self.position))?;
            file.read(bytes)?
        };
        self.position = self
            .position
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("archive source position overflow"))?;
        Ok(n)
    }
}
impl Seek for FileCursor {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let next = match from {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => self.len()?.checked_add_signed(delta),
        }
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid archive seek position")
        })?;
        self.position = next;
        Ok(next)
    }
}
