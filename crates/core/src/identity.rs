//! Archive identity: "am I looking at the archive I think I am?"
//!
//! The SHA-256 is computed **once per `Archive`** and memoised in a
//! `OnceLock`; marker reads are independently bounded and validated on each call.
//! Marker values preserve UTF-8 and spaces, removing only one terminal LF/CRLF;
//! unavailable values retain explicit status and source identity.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::archive::{Archive, EntryId};
use crate::caps;
use crate::error::ArchiveError;
use crate::limits::LimitViolation;

/// Memoised digest, shared by every clone of an `Archive`.
#[derive(Debug, Clone, Default)]
pub(crate) struct DigestCache(OnceLock<Option<DigestRecord>>);

#[derive(Debug, Clone, Copy)]
pub(crate) struct DigestRecord {
    pub sha256: [u8; 32],
    pub computed_at_ms: u64,
}

impl DigestCache {
    /// Returns the digest, computing it at most once.
    ///
    /// `Ok(None)` when the caller declined; that answer is memoised too, so a
    /// decline never costs a re-read either.
    pub(crate) fn get_or_compute(
        &self,
        archive: &Archive,
        with_sha256: bool,
    ) -> Result<Option<DigestRecord>, ArchiveError> {
        if let Some(hit) = self.0.get() {
            return Ok(*hit);
        }
        let record = if with_sha256 {
            Some(compute(archive)?)
        } else {
            None
        };
        // Racing threads may both compute; the first write wins and both
        // observe the same value, which is the memoisation contract.
        let _ = self.0.set(record);
        Ok(*self.0.get().unwrap_or(&record))
    }
}

fn compute(archive: &Archive) -> Result<DigestRecord, ArchiveError> {
    let path = archive.path();
    use std::io::Read;
    let file = archive.source();
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; caps::LINE_SCRATCH_BYTES];
    let mut rdr = file;
    loop {
        let n = rdr.read(&mut buf).map_err(|source| ArchiveError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let out = hasher.finalize();
    let mut sha256 = [0u8; 32];
    sha256.copy_from_slice(&out);
    Ok(DigestRecord {
        sha256,
        computed_at_ms: now_ms(),
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIdentity {
    pub basename: String,
    pub size_bytes: u64,
    /// Lowercase hex, 64 chars, over the file as stored. Computed once per
    /// `Archive`, memoised. `None` only when the caller declined it.
    pub sha256: Option<String>,
    pub sha256_computed_at_ms: Option<u64>,
    pub entry_count: usize,
    /// The central directory's claim, beside what was actually walked.
    pub declared_entry_count: usize,
    /// Valid version marker, with one terminal LF/CRLF removed; never parsed.
    pub format_version: Option<String>,
    /// Valid main-entry marker, preserving all spaces after line-ending removal.
    pub main_entry: Option<String>,
    pub main_entry_size_bytes: Option<u64>,
    pub markers: ArchiveMarkers,
    pub notes: Vec<ArchiveNote>,
}

/// Validation state for the optional identity-marker entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMarkers {
    pub version: MarkerMetadata,
    pub main_entry: MarkerMetadata,
}

/// Source provenance and validation result, separate from the optional value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerMetadata {
    pub status: MarkerStatus,
    pub entry_id: Option<EntryId>,
    pub declared_size_bytes: Option<u64>,
    /// Actual complete source length when a bounded read succeeded, before LF/CRLF removal.
    pub source_bytes: Option<usize>,
    pub line_ending_removed: bool,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerStatus {
    Valid,
    Missing,
    Oversized,
    InvalidUtf8,
    ContainsNul,
    Unreadable,
}

impl MarkerStatus {
    pub fn code(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Missing => "missing",
            Self::Oversized => "oversized",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::ContainsNul => "contains_nul",
            Self::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ArchiveNote {
    ArchiveFormat0,
    ContainsNul,
    EntriesCapped,
    Sha256Omitted,
}

impl ArchiveNote {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ArchiveFormat0 => "ARCHIVE_FORMAT_0",
            Self::ContainsNul => "CONTAINS_NUL",
            Self::EntriesCapped => "ENTRIES_CAPPED",
            Self::Sha256Omitted => "SHA256_OMITTED",
        }
    }
}

impl Archive {
    pub fn identity(&self, with_sha256: bool) -> Result<ArchiveIdentity, ArchiveError> {
        self.identity_cached(with_sha256)
    }

    pub(crate) fn identity_cached(
        &self,
        with_sha256: bool,
    ) -> Result<ArchiveIdentity, ArchiveError> {
        let index = self.index();
        let mut notes = Vec::new();

        let digest = self.digest_cache().get_or_compute(self, with_sha256)?;
        if digest.is_none() && !with_sha256 {
            notes.push(ArchiveNote::Sha256Omitted);
        }

        if index.declared_entry_count() != self.entry_count() {
            notes.push(ArchiveNote::EntriesCapped);
        }

        let (format_version, version_marker) = self.read_marker("version.txt");
        if version_marker.status == MarkerStatus::Missing {
            notes.push(ArchiveNote::ArchiveFormat0);
        }
        let (main_entry, main_entry_marker) = self.read_marker("main_entry.txt");

        let main_entry_size_bytes = main_entry
            .as_deref()
            .and_then(|name| self.index().entries().iter().find(|e| e.path == name))
            .map(|e| e.uncompressed_size);

        Ok(ArchiveIdentity {
            basename: self
                .path()
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size_bytes: index.archive_file_size(),
            sha256: digest.map(|d| hex32(&d.sha256)),
            sha256_computed_at_ms: digest.map(|d| d.computed_at_ms),
            entry_count: self.entry_count(),
            declared_entry_count: index.declared_entry_count(),
            format_version,
            main_entry,
            main_entry_size_bytes,
            markers: ArchiveMarkers {
                version: version_marker,
                main_entry: main_entry_marker,
            },
            notes,
        })
    }

    /// Rejects incomplete or invalid identifiers instead of returning guessed prefixes.
    fn read_marker(&self, name: &str) -> (Option<String>, MarkerMetadata) {
        let meta = self.index().entries().iter().find(|e| e.path == name);
        let mut state = MarkerMetadata {
            status: MarkerStatus::Missing,
            entry_id: meta.map(|m| m.id),
            declared_size_bytes: meta.map(|m| m.uncompressed_size),
            source_bytes: None,
            line_ending_removed: false,
            error_code: None,
        };
        let Some(meta) = meta else {
            return (None, state);
        };
        if meta.uncompressed_size > caps::MAX_FORMAT_VERSION_BYTES as u64 {
            state.status = MarkerStatus::Oversized;
            state.error_code = Some("marker_too_large".into());
            return (None, state);
        }
        let mut reader = match self.reader(meta.id) {
            Ok(reader) => reader,
            Err(error) => {
                state.status = MarkerStatus::Unreadable;
                state.error_code = Some(error.code().into());
                return (None, state);
            }
        };
        // The shared stream guard probes one byte beyond 256 to reject lying
        // declared sizes without silently accepting a prefix or reading the whole entry.
        reader.take_bounded(caps::MAX_FORMAT_VERSION_BYTES as u64);
        let bytes = reader.stream().try_fold(Vec::new(), |mut bytes, chunk| {
            bytes.extend_from_slice(&chunk?);
            Ok::<_, std::io::Error>(bytes)
        });
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                let cause = error
                    .get_ref()
                    .and_then(|e| e.downcast_ref::<ArchiveError>());
                state.status = if matches!(
                    cause,
                    Some(ArchiveError::LimitExceeded(
                        LimitViolation::EntryTooLarge { found, .. }
                    )) if *found > caps::MAX_FORMAT_VERSION_BYTES as u64
                ) {
                    MarkerStatus::Oversized
                } else {
                    MarkerStatus::Unreadable
                };
                state.error_code = Some(
                    cause
                        .map(ArchiveError::code)
                        .unwrap_or("marker_read_failed")
                        .into(),
                );
                return (None, state);
            }
        };
        state.source_bytes = Some(bytes.len());
        if bytes.contains(&0) {
            state.status = MarkerStatus::ContainsNul;
            state.error_code = Some("marker_contains_nul".into());
            return (None, state);
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(_) => {
                state.status = MarkerStatus::InvalidUtf8;
                state.error_code = Some("marker_invalid_utf8".into());
                return (None, state);
            }
        };
        let value = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'));
        state.line_ending_removed = value.is_some();
        state.status = MarkerStatus::Valid;
        (Some(value.unwrap_or(text).to_owned()), state)
    }
}
