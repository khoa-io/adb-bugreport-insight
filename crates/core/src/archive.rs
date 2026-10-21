//! The ZIP central-directory index and the read-only handle over it.
//!
//! The index is built **once**, by walking the central directory and
//! decompressing nothing. Everything addressable from a path is exposed as an
//! [`EntryId`]; paths are output, never input.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::caps;
use crate::error::ArchiveError;
use crate::glob::Glob;
use crate::identity::DigestCache;
use crate::limits::{ArchiveLimits, LimitViolation};
use crate::reader::EntryReader;
use crate::search::{ScanOutcome, ScanRequest};
use crate::source::FileCursor;

/// The only key tools address entries by.
///
/// Paths are output. This removes path traversal, encoding mismatch, duplicate
/// names, and glob-versus-literal ambiguity in one move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub u32);

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Raw MS-DOS packed fields. No calendar interpretation, no timezone claim —
/// the `zip` crate's optional `time` feature is not available offline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiedStamp {
    pub dos_date: u16,
    pub dos_time: u16,
}

/// A known method this build cannot decode retains its numeric code, so a
/// caller can report *which* method rather than only that it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CompressionMethodName {
    Stored,
    Deflated,
    Unsupported(u16),
}

impl std::fmt::Display for CompressionMethodName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stored => f.write_str("stored"),
            Self::Deflated => f.write_str("deflated"),
            Self::Unsupported(code) => write!(f, "unsupported({code})"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMeta {
    pub id: EntryId,
    /// Lossy UTF-8, display only. Never used as a key.
    pub path: String,
    pub path_bytes_valid_utf8: bool,
    /// Raw byte length from `name_raw()`.
    pub path_bytes_len: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compression: CompressionMethodName,
    pub compression_code: u16,
    /// `ZipFile::data_start()` — not optional.
    pub data_offset: u64,
    pub central_header_offset: u64,
    pub crc32: u32,
    pub modified: Option<ModifiedStamp>,
    pub modified_valid: bool,
    pub encrypted: bool,
}

/// Immutable. Built once at open by walking the central directory.
/// No decompression, no file handle, no lock, no interior mutability.
#[derive(Debug, Clone)]
pub struct ArchiveIndex {
    entries: Vec<EntryMeta>,
    central_directory_start: u64,
    /// The central directory's own claim, recorded beside what was walked.
    declared_entry_count: usize,
    archive_file_size: u64,
    uncompressed_total: u64,
}

impl ArchiveIndex {
    pub fn entries(&self) -> &[EntryMeta] {
        &self.entries
    }

    pub fn central_directory_start(&self) -> u64 {
        self.central_directory_start
    }

    pub fn declared_entry_count(&self) -> usize {
        self.declared_entry_count
    }

    pub fn archive_file_size(&self) -> u64 {
        self.archive_file_size
    }

    pub fn uncompressed_total(&self) -> u64 {
        self.uncompressed_total
    }
}

/// `Send + Sync`. Each reader has an independent logical cursor over the same
/// pinned descriptor; individual seek/read pairs serialize briefly.
#[derive(Debug, Clone)]
pub struct Archive {
    path: Arc<PathBuf>,
    source: FileCursor,
    index: Arc<ArchiveIndex>,
    limits: ArchiveLimits,
    /// Shared across clones so the digest is computed once per archive.
    digest_cache: Arc<DigestCache>,
}

impl Archive {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ArchiveError> {
        Self::open_with(path, ArchiveLimits::DEFAULT)
    }

    pub fn open_with(
        path: impl Into<PathBuf>,
        limits: ArchiveLimits,
    ) -> Result<Self, ArchiveError> {
        limits.validate()?;
        let path = path.into();

        let file = File::open(&path).map_err(|source| ArchiveError::Open {
            path: path.clone(),
            source,
        })?;
        Self::from_file(file, path, limits)
    }

    /// Bind an already authorized descriptor. `display_path` is used only for
    /// identity labels and error diagnostics; readers and hashes never reopen it.
    /// No writes are performed, even if the caller passes a writable descriptor.
    /// Mutations to the same underlying file remain visible; pinning prevents
    /// pathname/symlink replacement from redirecting access to another file.
    pub fn from_file(
        file: File,
        display_path: impl Into<PathBuf>,
        limits: ArchiveLimits,
    ) -> Result<Self, ArchiveError> {
        limits.validate()?;
        let path = display_path.into();
        let source = FileCursor::new(file);
        let archive_file_size = source.len().map_err(|source| ArchiveError::Open {
            path: path.clone(),
            source,
        })?;
        if archive_file_size > limits.max_archive_file_bytes {
            return Err(ArchiveError::LimitExceeded(LimitViolation::FileTooLarge {
                found: archive_file_size,
                limit: limits.max_archive_file_bytes,
            }));
        }

        // The EOCD record's claim, read independently of what we walk.
        let declared_entry_count =
            declared_entry_count(source.fresh(), archive_file_size).unwrap_or(0);

        let mut zip = ZipArchive::new(source.fresh()).map_err(|source| {
            if is_not_an_archive(&source) {
                ArchiveError::NotAZipArchive { path: path.clone() }
            } else {
                ArchiveError::Malformed(source)
            }
        })?;

        if zip.len() > limits.max_entries
            || (declared_entry_count != 0 && declared_entry_count > limits.max_entries)
        {
            return Err(ArchiveError::LimitExceeded(
                LimitViolation::TooManyEntries {
                    found: declared_entry_count.max(zip.len()),
                    limit: limits.max_entries,
                },
            ));
        }

        let index = build_index(&mut zip, archive_file_size, limits)?;
        drop(zip);

        Ok(Self {
            path: Arc::new(path),
            source,
            index: Arc::new(index),
            limits,
            digest_cache: Arc::new(DigestCache::default()),
        })
    }

    pub(crate) fn source(&self) -> FileCursor {
        self.source.fresh()
    }

    /// Clone the pinned binding with a fresh digest cache, for consumers needing
    /// a mandatory SHA-256 after a previous caller memoized a digest decline.
    /// The source descriptor and admitted index remain shared; no path is reopened.
    pub fn with_fresh_digest(&self) -> Self {
        Self {
            digest_cache: Arc::new(DigestCache::default()),
            ..self.clone()
        }
    }

    pub(crate) fn digest_cache(&self) -> &DigestCache {
        &self.digest_cache
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn index(&self) -> &ArchiveIndex {
        &self.index
    }

    pub fn limits(&self) -> ArchiveLimits {
        self.limits
    }

    pub fn entry_count(&self) -> usize {
        self.index.entries.len()
    }

    pub fn entry(&self, id: EntryId) -> Option<&EntryMeta> {
        self.index.entries.get(id.0 as usize)
    }

    pub fn entries(&self) -> &[EntryMeta] {
        &self.index.entries
    }

    /// The `EntryId` for `path`, if one exists. Convenience for tests and for
    /// callers holding a path; the *tools* still address by `EntryId`.
    pub fn id_for_path(&self, path: &str) -> Option<EntryId> {
        self.index
            .entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.id)
    }

    /// Independent logical cursor per call, parsing the pinned central directory.
    pub fn reader(&self, id: EntryId) -> Result<EntryReader, ArchiveError> {
        let meta = self.entry(id).ok_or(ArchiveError::EntryNotFound(id))?;
        let meta = meta.clone();
        let zip = ZipArchive::new(self.source()).map_err(ArchiveError::Malformed)?;
        if zip.len() <= id.0 as usize {
            return Err(ArchiveError::EntryNotFound(id));
        }
        Ok(EntryReader::new(zip, id.0 as usize, meta, self.limits))
    }

    /// Pass 1 — metadata only, no bytes.
    ///
    /// Matching is **segment-wise**: `*` does not cross `/`.
    pub fn find_by_glob(&self, pattern: &str) -> Result<Vec<EntryId>, ArchiveError> {
        let glob = Glob::new(pattern)?;
        Ok(self
            .index
            .entries
            .iter()
            .filter(|e| glob.is_match(&e.path))
            .map(|e| e.id)
            .collect())
    }

    pub fn detect(&self, id: EntryId) -> Result<crate::detect::DetectionReport, ArchiveError> {
        let meta = self
            .entry(id)
            .ok_or(ArchiveError::EntryNotFound(id))?
            .clone();
        let mut reader = self.reader(id)?;
        let sample_len = caps::DETECT_SAMPLE_BYTES.min(meta.uncompressed_size) as usize;
        let bytes = reader.read_byte_range(0, sample_len as u64)?;
        Ok(crate::detect::detect(
            &bytes,
            &meta.path,
            meta.uncompressed_size,
        ))
    }

    pub fn scan(&self, req: &ScanRequest) -> Result<ScanOutcome, ArchiveError> {
        crate::search::run_scan(self, req)
    }
}

/// Walks the central directory exactly once and enforces the open-time limits.
///
/// Uses [`ZipArchive::by_index_raw`], not `by_index`. `by_index_raw` returns a
/// *raw* handle — it never decrypts and never decompresses — which is exactly
/// the access this pass needs, and it still yields full metadata for an
/// encrypted entry. That matters because `by_index` refuses an encrypted entry
/// outright, which would make the whole archive unlistable because of one
/// bad entry. Here an encrypted entry is *recorded* as `encrypted`, and the
/// failure surfaces when something tries to read its bytes.
///
/// The guard is dropped before the next iteration: one live borrow of `zip` at
/// a time. That is the two-pass rule the borrow checker forces, applied at the
/// index level.
fn build_index(
    zip: &mut ZipArchive<FileCursor>,
    archive_file_size: u64,
    limits: ArchiveLimits,
) -> Result<ArchiveIndex, ArchiveError> {
    let central_directory_start = zip.central_directory_start();
    let count = zip.len();
    let mut entries: Vec<EntryMeta> = Vec::with_capacity(count);
    let mut uncompressed_total: u64 = 0;

    for i in 0..count {
        let id = EntryId(i as u32);
        let file = zip.by_index_raw(i).map_err(ArchiveError::Malformed)?;

        let raw = file.name_raw().to_vec();
        let (path, path_bytes_valid_utf8) = match std::str::from_utf8(&raw) {
            Ok(s) => (s.to_string(), true),
            // Never reject the read; decode lossily and record the fact.
            Err(_) => (String::from_utf8_lossy(&raw).into_owned(), false),
        };
        let path_bytes_len = raw.len();

        let is_dir = file.is_dir();
        let is_symlink = file.is_symlink();
        let compressed_size = file.compressed_size();
        let uncompressed_size = file.size();
        let compression_code = compression_code(file.compression());
        let compression = match compression_code {
            0 => CompressionMethodName::Stored,
            8 => CompressionMethodName::Deflated,
            other => CompressionMethodName::Unsupported(other),
        };
        let encrypted = file.encrypted();
        let data_offset = file.data_start();
        let central_header_offset = file.central_header_start();
        let crc32 = file.crc32();
        let (modified, modified_valid) = match file.last_modified() {
            Some(dt) => (
                Some(ModifiedStamp {
                    dos_date: dt.datepart(),
                    dos_time: dt.timepart(),
                }),
                dt.is_valid(),
            ),
            None => (None, false),
        };

        // Limits, applied to the declared sizes at open. A lying header is
        // additionally caught by the streaming guard in `EntryReader`.
        if uncompressed_size > limits.max_entry_bytes {
            return Err(ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge {
                entry_id: id,
                found: uncompressed_size,
                limit: limits.max_entry_bytes,
            }));
        }
        // A zero compressed size would divide by zero; `checked_div` makes that a
        // `None` rather than a panic, and `None` means "no ratio to judge".
        if let Some(ratio) = uncompressed_size.checked_div(compressed_size)
            && compressed_size > 0
            && ratio > limits.max_compression_ratio
        {
            return Err(ArchiveError::LimitExceeded(LimitViolation::RatioExceeded {
                entry_id: id,
                ratio,
                limit: limits.max_compression_ratio,
            }));
        }
        uncompressed_total = uncompressed_total.saturating_add(uncompressed_size);
        if uncompressed_total > limits.max_total_bytes {
            return Err(ArchiveError::LimitExceeded(LimitViolation::TotalTooLarge {
                found: uncompressed_total,
                limit: limits.max_total_bytes,
            }));
        }

        entries.push(EntryMeta {
            id,
            path,
            path_bytes_valid_utf8,
            path_bytes_len,
            is_dir,
            is_symlink,
            compressed_size,
            uncompressed_size,
            compression,
            compression_code,
            data_offset,
            central_header_offset,
            crc32,
            modified,
            modified_valid,
            encrypted,
        });
        // `file` drops here: one live borrow of `zip` at a time.
    }

    Ok(ArchiveIndex {
        entries,
        central_directory_start,
        declared_entry_count: count,
        archive_file_size,
        uncompressed_total,
    })
}

/// Maps a method to its MS-DOS code.
///
/// `CompressionMethod` is `#[non_exhaustive]`, and under this build's features
/// (`default-features = false, features = ["deflate"]`) every method other
/// than `Stored` and `Deflated` is already collapsed into `Unsupported(code)`.
/// The catch-all keeps the code stable if features are ever widened.
fn compression_code(m: zip::CompressionMethod) -> u16 {
    #[allow(deprecated)]
    match m {
        zip::CompressionMethod::Stored => 0,
        zip::CompressionMethod::Deflated => 8,
        zip::CompressionMethod::Unsupported(code) => code,
        other => {
            // Unreachable with this feature set; recorded rather than panicking
            // so a future feature addition degrades instead of crashing.
            eprintln!("bugreport-insight: unmapped compression method {other:?}");
            0xffff
        }
    }
}

fn is_not_an_archive(e: &zip::result::ZipError) -> bool {
    matches!(e, zip::result::ZipError::InvalidArchive(_))
}

/// Reads the total-entry count from the End Of Central Directory record,
/// independently of what the walk finds.
///
/// Bounded to the last [`caps::EOCD_TAIL_BYTES`], so a hostile archive cannot
/// induce an unbounded read. Returns `None` when no EOCD is found, which is
/// legal — ZIP64 and streamed archives do not always carry one in range.
fn declared_entry_count(mut file: FileCursor, file_size: u64) -> Option<usize> {
    let tail_len = caps::EOCD_TAIL_BYTES.min(file_size);
    let start = file_size - tail_len;
    file.seek(SeekFrom::Start(start)).ok()?;

    let mut buf = vec![0u8; tail_len as usize];
    file.read_exact(&mut buf).ok()?;

    // EOCD signature 0x06054b50, scanning backwards from EOF.
    let sig = [0x50u8, 0x4b, 0x05, 0x06];
    let mut i = buf.len().checked_sub(22)?;
    loop {
        if buf[i..].starts_with(&sig) {
            let b = &buf[i + 10..i + 12];
            return Some(u16::from_le_bytes([b[0], b[1]]) as usize);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}
