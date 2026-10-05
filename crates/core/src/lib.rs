//! Read-only, bounded Android bugreport ZIP access. Never extracts, never
//! writes, never follows a path as a key.
//!
//! # Addressing
//!
//! Everything is addressed by [`EntryId`]. Paths are **output**, never input.
//! That removes path traversal, encoding mismatch, duplicate entry names, and
//! glob-versus-literal ambiguity in one move.
//!
//! # The line bomb
//!
//! The reference archive's main entry has 682,951 lines and one line of
//! 192,666 characters. Every line walk in this crate streams through a fixed
//! 64 KiB scratch buffer and accumulates nothing until the target line, which
//! is itself capped at [`caps::MAX_LINE_BYTES`]. Reading line _N_ costs O(1)
//! memory, not O(N). `BufRead::lines` is never called.
//!
//! # ZIP only
//!
//! No TAR. Multi-format support is `DRAFT-REQ-MCP-ARC-09` and is deliberately
//! not implemented.

pub mod archive;
pub mod caps;
pub mod detect;
pub mod error;
pub mod glob;
pub mod identity;
pub mod limits;
pub mod reader;
pub mod search;
pub mod text;

pub use archive::{
    Archive, ArchiveIndex, CompressionMethodName, EntryId, EntryMeta, ModifiedStamp,
};
pub use caps::*;
pub use detect::{ContentType, DetectionBasis, DetectionReport, NestedKind};
pub use error::{ArchiveError, ArchiveStatus};
pub use glob::{Glob, GlobMatch};
pub use identity::{ArchiveIdentity, ArchiveNote};
pub use limits::{ArchiveLimits, LimitViolation};
pub use reader::{EntryChunks, EntryReader};
pub use search::{
    MatchKind, MatchLocator, PatternMode, ScanBudget, ScanOutcome, ScanRequest, StoppedReason,
};
pub use text::{Excerpt, ExcerptOutcome, LineIndex, LineSlice, TextDecoding};
