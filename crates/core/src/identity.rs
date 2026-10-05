//! Archive identity: "am I looking at the archive I think I am?"
//!
//! The SHA-256 is computed **once per `Archive`** and memoised in a
//! `OnceLock`, so repeated calls cost nothing. `format_version` and
//! `main_entry` are read **verbatim** and never parsed or branched on
//! (`docs/draft/artifact-schemas/conventions-envelope.md` §2.1.2).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::archive::Archive;
use crate::caps;
use crate::error::ArchiveError;

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
        path: &std::path::Path,
        with_sha256: bool,
    ) -> Result<Option<DigestRecord>, ArchiveError> {
        if let Some(hit) = self.0.get() {
            return Ok(*hit);
        }
        let record = if with_sha256 {
            Some(compute(path)?)
        } else {
            None
        };
        // Racing threads may both compute; the first write wins and both
        // observe the same value, which is the memoisation contract.
        let _ = self.0.set(record);
        Ok(*self.0.get().unwrap_or(&record))
    }
}

fn compute(path: &std::path::Path) -> Result<DigestRecord, ArchiveError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|source| ArchiveError::Open {
        path: path.to_path_buf(),
        source,
    })?;
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
    /// `version.txt` verbatim, stripped, capped at 256 bytes. Never parsed.
    pub format_version: Option<String>,
    /// `main_entry.txt` verbatim. Never interpreted.
    pub main_entry: Option<String>,
    pub main_entry_size_bytes: Option<u64>,
    pub notes: Vec<ArchiveNote>,
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

        let digest = self
            .digest_cache()
            .get_or_compute(self.path(), with_sha256)?;
        if digest.is_none() && !with_sha256 {
            notes.push(ArchiveNote::Sha256Omitted);
        }

        if index.declared_entry_count() != self.entry_count() {
            notes.push(ArchiveNote::EntriesCapped);
        }

        let format_version = match self.read_marker("version.txt")? {
            Some(v) => Some(v),
            None => {
                // Absence means format 0, and is reported rather than defaulted.
                notes.push(ArchiveNote::ArchiveFormat0);
                None
            }
        };
        let main_entry = self.read_marker("main_entry.txt")?;

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
            notes,
        })
    }

    /// Reads a marker entry verbatim, stripped, capped at 256 bytes.
    fn read_marker(&self, name: &str) -> Result<Option<String>, ArchiveError> {
        let Some(meta) = self.index().entries().iter().find(|e| e.path == name) else {
            return Ok(None);
        };
        let mut reader = self.reader(meta.id)?;
        let take = meta
            .uncompressed_size
            .min(caps::MAX_FORMAT_VERSION_BYTES as u64);
        let bytes = reader.read_byte_range(0, take)?;
        let text = String::from_utf8_lossy(&bytes).trim().to_string();
        Ok(Some(text))
    }
}
