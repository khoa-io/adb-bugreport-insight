//! Default archive limits and independent response/retention bounds.
//! Explicit force selects [`crate::ArchiveLimits::FORCED`].
//!
//! Every value here is a normative requirement number recorded at
//! `REQ-GEN-02` (metadata and lazy read safety) and `REQ-MCP-06` (bounded tool results).
//! Changing one is a requirements change, not a tuning knob.

/// Maximum entries accepted in the central directory. `REQ-GEN-02`.
pub const MAX_ENTRIES: usize = 10_000;

/// Default maximum uncompressed size of a single entry. Enforced against the
/// *decompressed stream*, not only the declared header. `REQ-GEN-02`.
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;

/// Maximum declared total uncompressed size across all entries at open. `REQ-GEN-02`.
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

/// Maximum archive file size, from `stat` on the file itself. `REQ-GEN-02`.
pub const MAX_ARCHIVE_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Maximum uncompressed-to-compressed ratio per entry, against the declared
/// size. A streaming guard catches what the header lies about.
pub const MAX_COMPRESSION_RATIO: u64 = 1_000;

/// Hard emitted UTF-8 text cap after sanitization, before JSON escaping. `REQ-MCP-06`.
pub const MAX_EXCERPT_BYTES: usize = 512;

/// Maximum bytes retained for a single line. Sized above the reference
/// archive's worst observed line (192,666 bytes) so a real bugreport never
/// trips it, while still bounding a hostile one.
pub const MAX_LINE_BYTES: u64 = 1024 * 1024;

/// Default item cap for list-shaped results. `REQ-MCP-06`.
pub const MAX_RESULT_ITEMS: usize = 200;

/// Default match cap for content search. `REQ-MCP-06`.
pub const MAX_GREP_MATCHES: usize = 50;

/// Default scanned-bytes budget per content-search call. `REQ-MCP-06`.
pub const MAX_GREP_SCAN_BYTES: u64 = 32 * 1024 * 1024;

/// Maximum accepted raw UTF-8 marker size, including terminal LF/CRLF. `REQ-MCP-06`.
pub const MAX_FORMAT_VERSION_BYTES: usize = 256;

/// Fixed scratch buffer used to count newlines while skipping the tail of an
/// over-long line. The tail is consumed and counted, never buffered, so this
/// value — not the line length — bounds reader memory.
pub const LINE_SCRATCH_BYTES: usize = 64 * 1024;

/// Bytes sampled for content detection. `REQ-MCP-ARC-07`.
pub const DETECT_SAMPLE_BYTES: u64 = 8 * 1024;

/// Bytes scanned backward from EOF when reading the End Of Central Directory
/// record. Bounded so a hostile archive cannot induce an unbounded read.
pub const EOCD_TAIL_BYTES: u64 = 64 * 1024;
