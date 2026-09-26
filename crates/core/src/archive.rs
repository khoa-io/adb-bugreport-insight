use crate::error::Error;
use std::{
    fs::File,
    io::BufReader,
    path::{Component, Path},
};
use zip::ZipArchive;

pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

/// Opens `path` as a ZIP archive without extracting any entries to disk.
pub fn open_archive(path: &Path) -> Result<ZipArchive<BufReader<File>>, Error> {
    let file = File::open(path).map_err(|source| Error::Open {
        path: path.display().to_string(),
        source,
    })?;
    Ok(ZipArchive::new(BufReader::new(file))?)
}

/// Rejects archives with unsafe entry names (path traversal, absolute paths, backslashes) or
/// that exceed the entry-count/entry-size/total-size limits, before any entry is read.
pub fn validate_archive(archive: &mut ZipArchive<BufReader<File>>) -> Result<(), Error> {
    if archive.len() > MAX_ENTRIES {
        return Err(Error::TooManyEntries);
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let path = Path::new(&name);
        if path.is_absolute()
            || name.contains('\\')
            || !path
                .components()
                .all(|item| matches!(item, Component::Normal(_) | Component::CurDir))
        {
            return Err(Error::UnsafeEntry(name));
        }
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(Error::EntryTooLarge(name));
        }
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_BYTES {
            return Err(Error::ArchiveTooLarge);
        }
    }
    Ok(())
}
