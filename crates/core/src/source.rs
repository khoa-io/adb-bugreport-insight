use crate::error::Error;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Component, Path},
};
use zip::ZipArchive;

pub const MAX_REPORT_BYTES: usize = 128 * 1024 * 1024;

/// Selects the canonical primary bugreport text file: `main_entry.txt`'s referenced entry when
/// present (per the AOSP bugreport ZIP format, version 1.0+, which survives the archive being
/// renamed by the user), otherwise the largest top-level `bugreport-*.txt` entry.
pub fn read_primary_report(
    archive: &mut ZipArchive<BufReader<File>>,
) -> Result<(String, String), Error> {
    let advertised_entry = match archive.by_name("main_entry.txt") {
        Ok(mut main_entry) => {
            let mut name = String::new();
            main_entry.read_to_string(&mut name)?;
            Some(name)
        }
        Err(_) => None,
    };
    if let Some(name) = advertised_entry.as_deref().map(str::trim)
        && !name.is_empty()
        && safe_archive_path(name)
        && let Ok(entry) = archive.by_name(name)
    {
        return read_report_entry(entry);
    }

    let mut candidate = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        if !entry.is_dir()
            && !name.contains('/')
            && name.to_ascii_lowercase().starts_with("bugreport-")
            && name.to_ascii_lowercase().ends_with(".txt")
            && candidate.is_none_or(|(_, size)| entry.size() > size)
        {
            candidate = Some((index, entry.size()));
        }
    }
    let (index, _) = candidate.ok_or(Error::NoReport)?;
    read_report_entry(archive.by_index(index)?)
}

fn read_report_entry(entry: zip::read::ZipFile<'_>) -> Result<(String, String), Error> {
    let source = entry.name().to_owned();
    let mut bytes = Vec::new();
    entry
        .take((MAX_REPORT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(Error::EntryTooLarge(source));
    }
    Ok((source, String::from_utf8_lossy(&bytes).into_owned()))
}

fn safe_archive_path(name: &str) -> bool {
    let path = Path::new(name);
    !path.is_absolute()
        && !name.contains('\\')
        && path
            .components()
            .all(|item| matches!(item, Component::Normal(_) | Component::CurDir))
}
