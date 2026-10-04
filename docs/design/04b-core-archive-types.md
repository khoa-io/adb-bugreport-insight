# 04b — `crates/archive` types

**Part 2 of 2.** Entry metadata, the bounded reader, detection, search, and identity. Begin with
[`04a-core-archive-api.md`](./04a-core-archive-api.md) for the contract, limits, and errors.

## `archive.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMeta {
    pub id: EntryId,
    pub path: String,                  // lossy UTF-8, display only
    pub path_bytes_valid_utf8: bool,
    pub path_bytes_len: usize,         // raw byte length, from name_raw()
    pub is_dir: bool,
    pub is_symlink: bool,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compression: CompressionMethodName,
    pub data_offset: u64,              // ZipFile::data_start() — NOT optional
    pub central_header_offset: u64,
    pub crc32: u32,
    pub modified: Option<ModifiedStamp>,
    pub modified_valid: bool,
    pub encrypted: bool,
}

/// Raw MS-DOS packed fields. No calendar interpretation, no timezone claim.
/// zip's optional `time` feature is absent offline — see 02-amendment-1.md §4.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiedStamp {
    pub dos_date: u16,
    pub dos_time: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CompressionMethodName {
    Stored,
    Deflated,
    /// A known method this build cannot decode. The code is retained.
    Unsupported(u16),
}

/// Immutable. Built once at open by walking the central directory. No decompression.
#[derive(Debug, Clone)]
pub struct ArchiveIndex {
    entries: Vec<EntryMeta>,
    central_directory_start: u64,
    declared_entry_count: usize,
    archive_file_size: u64,
}

/// Send + Sync. No file handle, no lock, no interior mutability.
/// Read tools take &self; content tools take &self and open a fresh handle per call.
#[derive(Debug, Clone)]
pub struct Archive {
    path: Arc<PathBuf>,
    index: Arc<ArchiveIndex>,
    limits: ArchiveLimits,
}

impl Archive {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ArchiveError>;
    pub fn open_with(
        path: impl Into<PathBuf>,
        limits: ArchiveLimits,
    ) -> Result<Self, ArchiveError>;

    pub fn path(&self) -> &Path;
    pub fn index(&self) -> &ArchiveIndex;
    pub fn limits(&self) -> ArchiveLimits;

    pub fn entry_count(&self) -> usize;
    pub fn entry(&self, id: EntryId) -> Option<&EntryMeta>;
    pub fn entries(&self) -> &[EntryMeta];

    /// Fresh handle per call. One open plus a central-directory parse.
    pub fn reader(&self, id: EntryId) -> Result<EntryReader<'_>, ArchiveError>;

    /// Pass 1 — metadata only, no bytes.
    pub fn find_by_glob(&self, pattern: &str) -> Result<Vec<EntryId>, ArchiveError>;
    pub fn detect(&self, id: EntryId) -> Result<DetectionReport, ArchiveError>;
    pub fn scan(&self, req: &ScanRequest) -> Result<ScanOutcome, ArchiveError>;
}

#[derive(Debug, Clone)]
pub struct ScanRequest {
    pub scope: Vec<EntryId>,          // empty = every entry
    pub pattern: String,
    pub mode: PatternMode,
    pub budget: ScanBudget,
}
```

### Why `data_offset` is `u64`

**Amendment 1 corrected this from `Option<u64>`.** `ZipFile::data_start()` is public
(`zip-2.4.2/src/read.rs:1776`) and a probe against a synthetic two-entry archive returned
`43` and `103`, consistent with the local headers.

The original record had marked it provisional because `plans/mcp_detailed.md` — which would have
shown the API usage — had never been compiled and was deleted. It can now be confirmed, and the
provisional was wrong in the pessimistic direction. **The O(entry-size) decompress-and-discard
fallback is deleted**: byte-range reads are O(1) seeks.

## `reader.rs` and `text.rs`

```rust
#[derive(Debug)]
pub struct EntryReader<'a> { /* private: bounded take() over the entry stream */ }

impl<'a> EntryReader<'a> {
    /// At most `limit` bytes. Overflow is LimitExceeded, NOT silent truncation.
    pub fn take_bounded(&mut self, limit: u64) -> EntryReader<'a>;
    /// 1-based inclusive. O(1) memory regardless of line length.
    pub fn read_line(&mut self, line_no: u64) -> Result<LineSlice, ArchiveError>;
    pub fn read_line_span(&mut self, start: u64, end: u64)
        -> Result<Vec<LineSlice>, ArchiveError>;
    pub fn read_byte_range(&mut self, off: u64, len: u64) -> Result<Vec<u8>, ArchiveError>;
    pub fn stream(&mut self) -> impl Iterator<Item = std::io::Result<Vec<u8>>> + '_;
}

#[derive(Debug, Clone)]
pub struct LineSlice {
    pub line_no: u64,          // 1-based
    pub bytes: Vec<u8>,        // may be shorter than the full line
    pub byte_offset: u64,      // uncompressed offset of the line start
    pub byte_len_full: u64,    // full line length, even when bytes is truncated
    pub outcome: ExcerptOutcome,
}

/// Lazily built per entry, capped. Never materialises a 192,666-char line.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// (byte_offset, line_no) checkpoints every N lines.
    checkpoints: Vec<(u64, u64)>,
    total_lines: Option<u64>,
    capped: bool,
}

impl LineIndex {
    pub fn build(
        reader: &mut EntryReader<'_>,
        max_checkpoints: usize,
    ) -> Result<Self, ArchiveError>;
    pub fn total_lines(&self) -> Option<u64>;
    pub fn is_capped(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Excerpt {
    pub text: String,
    pub bytes: usize,
    pub truncated: bool,
    pub complete: bool,
    pub reason: Option<StoppedReason>,
    pub decoding: TextDecoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExcerptOutcome {
    Complete,
    /// Line exceeded MAX_LINE_BYTES. The remainder is SKIPPED by newline counting,
    /// never buffered.
    LineTooLong { capped_at: u64, skipped: u64 },
    Cut { at: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TextDecoding { Utf8, Utf8Lossy, Binary }
```

`LineIndex.checkpoints` is the mechanism that makes reading line _N_ cost **O(1) memory**
regardless of line length — see
[`06` §2](./06-hostile-archive.md#2-the-line-bomb-is-the-design-driver).

## `detect.rs`

```rust
/// Fifteen closed values. Unknown is legal only with DetectionBasis::None.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContentType {
    Text, Json, Xml, Html, Csv,
    ImagePng, ImageJpeg, ImageGif, ImageWebp, ImageBmp,
    ArchiveZip, ArchiveGzip, ArchiveBzip2, ArchiveXz, ArchiveSevenZ,
    Binary, Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DetectionBasis {
    MagicBytes,
    ZipEntryName,
    UniformExtension,
    None,   // the ONLY basis legal with ContentType::Unknown
}

/// A sample-based classification, and it says so. Never a bare confident label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub content_type: ContentType,
    pub basis: DetectionBasis,
    pub sample_bytes: u64,   // how much was actually inspected
    pub entry_size: u64,
    pub is_text: bool,
    pub contains_nul: bool,
    pub nested: Option<NestedKind>,
    pub notes: Vec<ArchiveNote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NestedKind { Zip, Gzip, Bzip2, Xz, SevenZ }

/// v1 DETECTS and REPORTS. v1 never OPENS. Opening is DRAFT-REQ-GEN-03.
pub fn detect(sample: &[u8], entry_name: &str, entry_size: u64) -> DetectionReport;
pub fn nested_archive_magic(sample: &[u8]) -> Option<NestedKind>;
```

## `search.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PatternMode {
    Literal,   // available offline
    Glob,      // available offline
    /// DRAFT-REQ-MCP-ARC-08. Returns PatternModeUnavailable — a typed refusal,
    /// never a silent fallback.
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MatchKind { Literal, Glob, Regex }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchLocator {
    pub entry_id: EntryId,
    pub path: String,
    pub line: u64,            // 1-based
    pub line_end: u64,        // == line unless the match spans lines
    pub byte_offset: u64,     // uncompressed offset of the line start
    pub byte_len_match: u64,
    pub excerpt: Excerpt,
}

/// Per-call budget. Defaults come from the crate constants.
#[derive(Debug, Clone, Copy)]
pub struct ScanBudget {
    pub max_matches: usize,
    pub max_scan_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_excerpt_bytes: usize,
}

impl ScanBudget {
    pub const DEFAULT: Self = Self {
        max_matches: caps::MAX_GREP_MATCHES,          // 50
        max_scan_bytes: caps::MAX_GREP_SCAN_BYTES,    // 32 MiB
        max_entry_bytes: caps::MAX_ENTRY_BYTES,       // 256 MiB
        max_excerpt_bytes: caps::MAX_EXCERPT_BYTES,   // 512
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOutcome {
    pub matches: Vec<MatchLocator>,
    pub scanned_entries: usize,
    pub scanned_bytes: u64,
    pub stopped: Option<StoppedReason>,
    pub omitted_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StoppedReason {
    ItemCap, MatchCap, ScanBudget, ByteBudget, LineTooLong,
}
```

### The hand-written glob

`regex` and `globset` are both absent from the offline cache, and no glob crate of any name is
available. `Literal` and `Glob` are therefore hand-written — see
[`02` §4.2](./02-amendment-1.md#42-regex--globset--replace-do-not-defer) for why regex alone is
deferred rather than hand-written.

**One real correctness trap:** the glob must be **segment-wise, not character-wise**. Otherwise
`*` crosses `/` and matches paths the caller never named. That gets its own test, and
[`07` §5](./07-build-plan.md#5-acceptance-tests) requires a second reviewer before Stage 4
merges.

## `identity.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIdentity {
    pub basename: String,
    pub size_bytes: u64,
    /// Lowercase hex, 64 chars, over the file as stored. Computed ONCE per Archive,
    /// memoised in a OnceLock. None only when the caller declined it.
    pub sha256: Option<String>,
    pub sha256_computed_at_ms: Option<u64>,
    pub entry_count: usize,
    /// The central directory's claim, beside what was actually walked.
    pub declared_entry_count: usize,
    pub format_version: Option<String>, // version.txt verbatim, stripped, <= 256 bytes
    pub main_entry: Option<String>,     // main_entry.txt verbatim
    pub main_entry_size_bytes: Option<u64>,
    pub main_entry_line_count: Option<u64>,
    pub notes: Vec<ArchiveNote>,
}

impl Archive {
    pub fn identity(&self, with_sha256: bool) -> Result<ArchiveIdentity, ArchiveError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ArchiveNote { ArchiveFormat0, ContainsNul, EntriesCapped, Sha256Omitted }
```

`format_version` is **read verbatim and never parsed or branched on**, matching
`artifact-schemas.md` §2.1.2. Absence means format 0 and is reported, never defaulted.

`sha256` uses `ring`, not `sha2` — `sha2` is absent and `ring 0.17.14` was verified against the
NIST vector for `"abc"`. See
[`06` §7](./06-hostile-archive.md#7-verification-appendix).

## `ArchiveStatus` — why it lives here

**This enum belongs in `crates/archive`, not `crates/mcp`.** It is the four-way outcome of an
_archive_ operation, so it belongs beside `ArchiveError`. `crates/mcp` maps it onto the wire
envelope; it does not define it. Engineer B needs it to compile the tool layer.

```rust
/// Four-value projection of the draft eight-value SectionStatus
/// (artifact-schemas.md §2.2):
///     ok -> Ok,  empty -> Empty,  not_found -> NotFound,  unreadable -> Unreadable
/// The four skipped_* values arrive with DRAFT-REQ-ART-*.
/// See docs/mcp-protocol.md for the full projection table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveStatus {
    Ok,
    Empty,
    NotFound,
    Unreadable,
}

/// The rule REQ-MCP-08 exists to enforce:
///   Empty     reachable ONLY when the operation completed and found nothing.
///   NotFound  ONLY when the addressed thing does not exist.
///   Unreadable ONLY when it exists and could not be read.
impl ArchiveStatus {
    pub fn from_error(e: &ArchiveError) -> Self {
        match e {
            ArchiveError::EntryNotFound(_) => Self::NotFound,
            ArchiveError::EntryIsDirectory(_)
            | ArchiveError::EntryEncrypted { .. }
            | ArchiveError::UnsupportedCompression { .. }
            | ArchiveError::UnreadableEntry { .. } => Self::Unreadable,
            // Open, malformed-archive and limit failures are transport errors,
            // not statuses: they mean no archive is bound at all.
            _ => Self::NotFound,
        }
    }
    pub fn code(&self) -> &'static str; // "ok" | "empty" | "not_found" | "unreadable"
}
```