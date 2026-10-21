# Core archive types

[lib.rs](../../crates/core/src/lib.rs) re-exports and linked definitions own fields, signatures, derives and enum spellings.
ARCH-ARCHIVE type relationships appear below; [the contract](./01-core-archive-contract.md) owns operations; [traceability](../traceability.md) joins requirements/checks.

| Source | Types and purpose |
| --- | --- |
| [archive.rs](../../crates/core/src/archive.rs) | EntryId is a transparent u32 index key. EntryMeta contains display path, raw-name validity/length, directory/symlink/encryption flags, declared sizes, compression method/code, offsets, CRC and modified stamp. ArchiveIndex holds immutable metadata; Archive shares it and DigestCache via Arc. |
| [source.rs](../../crates/core/src/source.rs) | Private FileCursor shares one pinned File with independent logical positions; a mutex makes each seek/read atomic without sharing ZIP decoder state. |
| [limits.rs](../../crates/core/src/limits.rs) | ArchiveLimits defines count, entry/total/file bytes and ratio ceilings; validate rejects zero/incoherent sets. LimitViolation carries category, observed value, limit and relevant EntryId. |
| [reader.rs](../../crates/core/src/reader.rs) | EntryReader owns a ZIP parser over an independent logical cursor on the pinned source; it has no lifetime parameter. EntryChunks borrows it and retains one decoder across chunks, enforcing the same actual-byte guard as line walks. |
| [text.rs](../../crates/core/src/text.rs) | LineSlice records entry ID, 1-based line, retained bytes, byte offset/full length and ExcerptOutcome. Excerpt carries text, bytes, truncation/completeness/reason/decoding. LineIndex builds capped checkpoints and an optional total; checkpoint_for returns the nearest recorded point. |
| [detect.rs](../../crates/core/src/detect.rs) | DetectionReport records ContentType, DetectionBasis, sample/entry sizes, text/NUL flags, nested kind and notes. ContentType has 17 variants, including Binary and Unknown; classification is sample-based. |
| [search.rs](../../crates/core/src/search.rs) | ScanRequest contains scope, pattern, mode and ScanBudget. ScanSession owns resumable decoder/line progress and coverage; ScanStep yields a match, page pause or exhaustion. ScanOutcome holds matches, scanned/completed/skipped entries, skipped lines, coverage, bytes, stop reason and omitted count. MatchLocator carries an uncompressed line-start offset and matching excerpt. |
| [identity.rs](../../crates/core/src/identity.rs) | ArchiveIdentity holds optional hash/marker values, ArchiveMarkers with per-marker MarkerMetadata/MarkerStatus, and ArchiveNote values. It has no main_entry_line_count field. ArchiveNote derives ordinary enum serialization; code() uppercase strings are a separate API. |
| [error.rs](../../crates/core/src/error.rs) | ArchiveError is non-exhaustive with stable codes. ArchiveStatus has Ok, Empty, NotFound and Unreadable, serialized in snake_case. |

LineIndex memory grows with its checkpoint cap; normal reads skip lines through reader scratch.
Excerpt::from_bytes and LineSlice::to_excerpt cap emitted UTF-8 text at 512 bytes after sanitization, preserving whole characters.
Excerpt.bytes counts emitted text; source_bytes counts the source prefix represented, so source offsets can advance correctly even after replacement expansion.
LineSlice byte_len_full and byte-range bytes_returned retain source-byte meaning; [ADR 0007](../decisions/0007-emitted-utf8-excerpt-cap.md) defines clipping/completeness semantics.
