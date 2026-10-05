//! The error model. No `unwrap`, no `expect`, no panic anywhere in this crate.

use std::path::PathBuf;

use thiserror::Error;

use crate::archive::{CompressionMethodName, EntryId};
use crate::limits::LimitViolation;
use crate::search::PatternMode;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ArchiveError {
    #[error("cannot open `{path}`: {source}")]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("not a ZIP archive: `{path}`")]
    NotAZipArchive { path: PathBuf },

    #[error("{0}")]
    LimitExceeded(#[from] LimitViolation),

    #[error("no entry with id {0}")]
    EntryNotFound(EntryId),

    #[error("entry {0} is a directory, not a file")]
    EntryIsDirectory(EntryId),

    #[error("entry {id} is encrypted")]
    EntryEncrypted { id: EntryId },

    #[error("entry {id} uses unsupported compression method {method} (code {code})")]
    UnsupportedCompression {
        id: EntryId,
        method: CompressionMethodName,
        code: u16,
    },

    #[error("entry {id} is unreadable: {source}")]
    UnreadableEntry {
        id: EntryId,
        source: zip::result::ZipError,
    },

    #[error("byte range {requested_start}..{requested_end} outside entry {id} of {actual} bytes")]
    ByteRangeOutOfBounds {
        id: EntryId,
        requested_start: u64,
        requested_end: u64,
        actual: u64,
    },

    #[error("line range {requested_start}..{requested_end} outside entry {id} of {actual} lines")]
    LineRangeOutOfBounds {
        id: EntryId,
        requested_start: u64,
        requested_end: u64,
        actual: u64,
    },

    #[error("supply either line_start/line_end or byte_offset/byte_length, not both")]
    ConflictingRangeParams,

    #[error("invalid glob `{pattern}`: {message}")]
    InvalidGlob { pattern: String, message: String },

    #[error("pattern mode {mode:?} is not available: {reason}")]
    PatternModeUnavailable {
        mode: PatternMode,
        reason: &'static str,
    },

    #[error("malformed archive: {0}")]
    Malformed(#[source] zip::result::ZipError),

    #[error("limit set is unusable: {reason}")]
    InvalidLimits { reason: &'static str },
}

impl ArchiveError {
    /// Stable machine code for the wire. `Display` is prose; this is the contract.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Open { .. } => "open_failed",
            Self::NotAZipArchive { .. } => "not_a_zip",
            Self::LimitExceeded(v) => v.code(),
            Self::EntryNotFound(_) => "entry_not_found",
            Self::EntryIsDirectory(_) => "entry_is_directory",
            Self::EntryEncrypted { .. } => "entry_encrypted",
            Self::UnsupportedCompression { .. } => "unsupported_compression",
            Self::UnreadableEntry { .. } => "unreadable_entry",
            Self::ByteRangeOutOfBounds { .. } => "byte_range_out_of_bounds",
            Self::LineRangeOutOfBounds { .. } => "line_range_out_of_bounds",
            Self::ConflictingRangeParams => "conflicting_range_params",
            Self::InvalidGlob { .. } => "invalid_glob",
            Self::PatternModeUnavailable { .. } => "pattern_mode_unavailable",
            Self::Malformed(_) => "malformed_archive",
            Self::InvalidLimits { .. } => "invalid_limits",
        }
    }

    /// True when the archive is usable but this one entry is not.
    ///
    /// The distinction is what lets `crates/mcp` map a per-entry failure to
    /// [`ArchiveStatus::Unreadable`] while a whole-archive failure stays a
    /// transport error.
    pub fn is_per_entry(&self) -> bool {
        match self {
            Self::EntryNotFound(_)
            | Self::EntryIsDirectory(_)
            | Self::EntryEncrypted { .. }
            | Self::UnsupportedCompression { .. }
            | Self::UnreadableEntry { .. }
            | Self::ByteRangeOutOfBounds { .. }
            | Self::LineRangeOutOfBounds { .. } => true,
            Self::Open { .. }
            | Self::NotAZipArchive { .. }
            | Self::LimitExceeded(_)
            | Self::ConflictingRangeParams
            | Self::InvalidGlob { .. }
            | Self::PatternModeUnavailable { .. }
            | Self::Malformed(_)
            | Self::InvalidLimits { .. } => false,
        }
    }
}

/// Four-value projection of the draft eight-value SectionStatus
/// (`artifact-schemas.md` §2.2).
///
/// `ok -> Ok`, `empty -> Empty`, `not_found -> NotFound`,
/// `unreadable -> Unreadable`. The four `skipped_*` values arrive with
/// `DRAFT-REQ-ART-*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ArchiveStatus {
    Ok,
    Empty,
    NotFound,
    Unreadable,
}

/// The rule `REQ-MCP-08` exists to enforce:
///
/// * `Empty`     reachable ONLY when the operation completed and found nothing.
/// * `NotFound`  ONLY when the addressed thing does not exist.
/// * `Unreadable` ONLY when it exists and could not be read.
impl ArchiveStatus {
    pub fn from_error(e: &ArchiveError) -> Self {
        match e {
            // The addressed thing does not exist. The *only* source of NotFound.
            ArchiveError::EntryNotFound(_) => Self::NotFound,
            // The entry exists but its bytes could not be produced. A range
            // beyond end-of-entry belongs here too: the entry is there, the
            // read is not possible, and reporting NotFound would claim the
            // entry is absent — the exact confusion REQ-MCP-08 forbids.
            ArchiveError::EntryIsDirectory(_)
            | ArchiveError::EntryEncrypted { .. }
            | ArchiveError::UnsupportedCompression { .. }
            | ArchiveError::UnreadableEntry { .. }
            | ArchiveError::ByteRangeOutOfBounds { .. }
            | ArchiveError::LineRangeOutOfBounds { .. } => Self::Unreadable,
            // Open, malformed-archive and limit failures are transport errors,
            // not statuses: they mean no archive is bound at all. Reached only
            // if a caller projects one anyway.
            _ => Self::Unreadable,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Empty => "empty",
            Self::NotFound => "not_found",
            Self::Unreadable => "unreadable",
        }
    }
}
