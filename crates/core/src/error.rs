use crate::archive::{MAX_ENTRIES, MAX_ENTRY_BYTES, MAX_TOTAL_BYTES};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open `{path}`: {source}")]
    Open {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid or unsupported ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("archive has more than {MAX_ENTRIES} entries")]
    TooManyEntries,
    #[error("unsafe archive entry name: `{0}`")]
    UnsafeEntry(String),
    #[error("archive entry `{0}` exceeds the {MAX_ENTRY_BYTES}-byte limit")]
    EntryTooLarge(String),
    #[error("archive exceeds the {MAX_TOTAL_BYTES}-byte total read limit")]
    ArchiveTooLarge,
    #[error("no top-level bugreport text entry was found")]
    NoReport,
    #[error("output directory `{0}` is non-empty; use --force to replace it")]
    OutputExists(String),
    #[error("cannot write artifact: {0}")]
    Write(#[from] std::io::Error),
    #[error("cannot serialize artifact: {0}")]
    Json(#[from] serde_json::Error),
}
