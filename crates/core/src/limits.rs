//! Hard limits and the violations they produce. Reject, never truncate.

use std::fmt;

use crate::archive::EntryId;
use crate::caps;
use crate::error::ArchiveError;

/// Selected archive admission and produced-read limits (`REQ-GEN-02`).
/// Reject, never truncate; explicit force selects a finite raised profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
    pub max_archive_file_bytes: u64,
    pub max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ArchiveLimits {
    pub const DEFAULT: Self = Self {
        max_entries: caps::MAX_ENTRIES,
        max_entry_bytes: caps::MAX_ENTRY_BYTES,
        max_total_bytes: caps::MAX_TOTAL_BYTES,
        max_archive_file_bytes: caps::MAX_ARCHIVE_FILE_BYTES,
        max_compression_ratio: caps::MAX_COMPRESSION_RATIO,
    };

    /// Explicit opt-in profile for unusually large bugreports. Integrity checks,
    /// retained-line and protocol response bounds remain independent.
    pub const FORCED: Self = Self {
        max_entries: 100_000,
        max_entry_bytes: 4 * 1024 * 1024 * 1024,
        max_total_bytes: 16 * 1024 * 1024 * 1024,
        max_archive_file_bytes: 8 * 1024 * 1024 * 1024,
        max_compression_ratio: 10_000,
    };

    /// Rejects a limit set that could not be enforced coherently. Called by
    /// [`crate::Archive::open_with`] so a nonsensical custom limit fails at
    /// open rather than mid-read.
    pub fn validate(&self) -> Result<(), ArchiveError> {
        let sane = self.max_entries > 0
            && self.max_entry_bytes > 0
            && self.max_total_bytes > 0
            && self.max_archive_file_bytes > 0
            && self.max_compression_ratio > 0
            // A single entry can never be allowed to exceed the whole budget,
            // or the two bounds contradict each other.
            && self.max_entry_bytes <= self.max_total_bytes;
        if sane {
            Ok(())
        } else {
            Err(ArchiveError::InvalidLimits {
                reason: LIMITS_REASON,
            })
        }
    }
}

const LIMITS_REASON: &str = "limit set is empty or internally contradictory";

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitViolation {
    TooManyEntries {
        found: usize,
        limit: usize,
    },
    EntryTooLarge {
        entry_id: EntryId,
        found: u64,
        limit: u64,
    },
    TotalTooLarge {
        found: u64,
        limit: u64,
    },
    FileTooLarge {
        found: u64,
        limit: u64,
    },
    RatioExceeded {
        entry_id: EntryId,
        ratio: u64,
        limit: u64,
    },
}

impl LimitViolation {
    /// Stable machine code. This is the contract; `Display` is prose.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooManyEntries { .. } => "entries",
            Self::EntryTooLarge { .. } => "entry_bytes",
            Self::TotalTooLarge { .. } => "total_bytes",
            Self::FileTooLarge { .. } => "file_bytes",
            Self::RatioExceeded { .. } => "ratio",
        }
    }
}

impl fmt::Display for LimitViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries { found, limit } => {
                write!(
                    f,
                    "central directory declares {found} entries, limit {limit}"
                )
            }
            Self::EntryTooLarge {
                entry_id,
                found,
                limit,
            } => write!(
                f,
                "entry {} declares {found} uncompressed bytes, limit {limit}",
                entry_id.0
            ),
            Self::TotalTooLarge { found, limit } => {
                write!(
                    f,
                    "archive declares {found} uncompressed bytes total, limit {limit}"
                )
            }
            Self::FileTooLarge { found, limit } => {
                write!(f, "archive file is {found} bytes, limit {limit}")
            }
            Self::RatioExceeded {
                entry_id,
                ratio,
                limit,
            } => write!(
                f,
                "entry {} has compression ratio {ratio}, limit {limit}",
                entry_id.0
            ),
        }
    }
}

impl std::error::Error for LimitViolation {}
