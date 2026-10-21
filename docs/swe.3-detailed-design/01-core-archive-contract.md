# Core archive contract

## Scope and API owner

`bugreport-insight-core` implements ARCH-ARCHIVE.
It opens read-only ZIPs, indexes central-directory metadata and exposes EntryId-based listing, detection, bounded reads, search and identity.
It does not extract files or recursively open nested archives.
REQ-GEN-01/02, REQ-LIB-01, REQ-MCP-03/09 and REQ-MCP-ARC-01…07 motivate this contract; [traceability](../traceability.md) joins individual obligations to source and verification.

The executable API is [lib.rs](../../crates/core/src/lib.rs) and its public modules.
Build local API documentation with `cargo doc --workspace --offline --no-deps`.

## Open and identity

[Archive::open/open_with](../../crates/core/src/archive.rs) opens the file once and delegates to `Archive::from_file(file, display_path, limits)`, which validates file size, configured limits, declared entry count/size/total/ratio, and constructs an immutable ArchiveIndex without decompressing.
`display_path` supplies diagnostics/identity labels only; `from_file` never reopens that pathname and performs no writes even if the supplied descriptor permits writing.
Private `source::FileCursor` retains the descriptor through `Arc<Mutex<File>>`; its independent positions serialize only each seek/read pair.
This portable core mechanism blocks pathname redirection, not in-place changes to the pinned file.
EntryId is the central-directory position.
Duplicate names and lossy path encodings do not change that key.
Paths/raw-name flags are metadata, not read addresses.
ArchiveLimits is configurable through the library; CLI runs select DEFAULT or FORCED through `--force` under REQ-CLI-12.
REQ-GEN-02 requires lazy validation: actual decompressed-size guards execute during selected reads.
The aggregate limit uses declared sizes; no whole-archive actual-expansion or cumulative-session counter is implied.
Opening and partial reads leave uninspected content unverified; see [ADR 0004](../decisions/0004-lazy-archive-validation.md).

[Archive::identity](../../crates/core/src/identity.rs) reports basename, stored size, entry counts, optional SHA-256 and compute time, format marker, main-entry marker/size, and notes.
It uses sha2.
Marker reads accept full UTF-8 payloads of at most 256 bytes without NUL, removing one terminal LF/CRLF and preserving all remaining characters/spaces.
Declared oversized markers are withheld before payload reads; the shared guarded stream probes at most one additional byte to catch actual output above 256 when metadata understates size.
Marker errors become explicit per-marker metadata rather than failing archive identity; requested hashing errors retain their independent failure path.
ArchiveFormat0 is recorded only for absent version.txt.
The index supplies the named main entry's declared size after an exact match against the valid marker value; valid marker syntax does not prove the target entry exists.
[ADR 0008](../decisions/0008-validated-marker-metadata.md) defines the bounded validation contract.
SHA-256 caches the first successful choice, including a declined digest; racing callers can compute before one OnceLock value wins.
Repeated identity calls still read markers.
No strict once-only IO is promised.

## Read and search

[Archive::reader](../../crates/core/src/archive.rs) creates an owned EntryReader with an independent logical cursor and ZIP parser over the pinned source file.
[EntryReader](../../crates/core/src/reader.rs) supports 1-based inclusive line spans and zero-based byte offset/length requests.
The MCP interface expresses bytes with an exclusive end.
MCP uses partial range methods to preserve available EOF content under ADR 0012; strict core methods still reject unavailable spans.
Byte ranges replay/discard decompression: O(offset + length) time, fixed discard scratch, output memory proportional to the returned range.
data_offset metadata does not provide O(1) access to an arbitrary decompressed offset.

Line walks skip with fixed scratch, retain up to MAX_LINE_BYTES per returned line, and count the full length of a skipped tail.
LineSlice::to_excerpt decodes valid UTF-8 as whole characters rather than mapping bytes individually; malformed bytes/NUL use the replacement behavior in [safety design](./04-archive-safety.md#memory-decoding-and-addressing).
The accepted [emitted-text rule](../decisions/0007-emitted-utf8-excerpt-cap.md) caps each excerpt at 512 UTF-8 bytes after decoding/sanitization, with bytes counting emitted text and source_bytes counting represented source-prefix bytes.
Source offsets/full line lengths remain unchanged; retained-line limits and archive expansion guards are independent of excerpt size.
Multi-line spans retain multiple LineSlice values; constant scratch does not bound total result memory.
Sequential for_each_line avoids restarting the decoder per line.
EntryChunks borrows the EntryReader for its lifetime and keeps one ZipFile decoder across chunks.
Its shared Cursor counts every produced chunk, errors on overflow and probes beyond an exact ceiling to distinguish EOF from excess data.
Dropping the iterator ends the borrow; callers cannot move the iterator beyond the reader's lifetime.
Decoder progress remains continuous and safety overflow remains an error.
LineIndex is a separate checkpoint-building API; normal read_line and search do not use it to seek into deflated content.

[Search](../../crates/core/src/search.rs) returns MatchLocator values with entry ID, displayed path, 1-based line and uncompressed line-start byte offset.
Literal matching uses grep-regex fixed strings; content globs are whole-line patterns and `*`/`?` exclude slash.
Regex mode is explicitly refused.
[Path Glob](../../crates/core/src/glob.rs) has its own segment-aware implementation, including `**`.
[Runtime coverage](../swe.2-architecture/03-runtime-and-interactions.md#search-coverage) documents what stopped scan counts can and cannot establish.

## Errors and verification

### Operation contracts for callers

Observed semantics do not replace conflicting requirements; linked source owns signatures.
Entry IDs and uncompressed offsets are meaningful only for the bound archive.

| Operation | Input and precondition | Outcome and important failure |
| --- | --- | --- |
| `Archive::open_with` | Existing ZIP plus a coherent `ArchiveLimits` set | Immutable metadata index or a typed open/limit error; this does not validate every decompressed payload |
| `Archive::reader` | EntryId from this archive | Owned reader; absent ID, directory, encryption or unsupported compression cannot be treated as empty content |
| `read_line_span` | `start >= 1`, `end >= start`, inclusive endpoints | Ordered LineSlice values with source offsets/full lengths; invalid or unavailable span errors rather than returning the available prefix |
| `read_byte_range` | Zero-based offset and length whose end fits declared uncompressed size | Returned bytes after replay/discard; past-end requests error; retained output grows with length |
| `read_line_span_partial` | Valid inclusive line endpoints | Available ordered slices plus past_end; empty past-end spans succeed, invalid endpoints and read/safety failures error |
| `read_byte_range_partial` | Zero-based offset and length | Available intersection with declared size plus past_end; strict guarded read checks bytes produced and refuses premature EOF inside that intersection |
| `for_each_line` | Callback accepting a LineSlice and returning whether to continue | Sequential walk until EOF, callback stop or error; callback stop is not proof that the remainder was inspected |
| `stream` | Mutable borrow of EntryReader | Sequential bounded chunks; typed ArchiveError inside IO errors; ceiling overflow is an error and early stopping does not validate the unread suffix |
| `scan` | Pattern, supported mode, scope and ScanBudget | Locators plus work/stop metadata; absent explicit IDs are errors, directories are completed without content reads and encrypted entries are explicit coverage gaps |

Byte-range length is a core API input.
MCP rejects reversed endpoints before subtraction and uses the partial APIs under [ADR 0012](../decisions/0012-range-boundaries-and-subtree-filters.md).
The strict core APIs remain available for callers requiring the whole requested span; partial APIs do not suppress decoder, CRC or produced-byte-limit failures.
Byte-range EOF uses admitted declared size; bounded reads retain lazy validation limits and do not certify the unread tail.

### Error ownership

[ArchiveError::code](../../crates/core/src/error.rs) supplies stable machine codes; Display is prose.
EntryNotFound projects to NotFound, while directory/encrypted/unsupported/corrupt/range errors project to Unreadable.
Retain failure context; status projection alone does not define wire mapping.
[MCP design](./03-mcp-server.md) owns that translation.
[Unit measures](../swe.4-unit-verification/01-verification-measures.md) identify actual checks.
[Safety](./04-archive-safety.md) owns limits, decoding and hostile-input handling.

Core ScanSession exposes resumable next_match steps with a caller-supplied remaining source-byte budget, preserving decoder, partial lines and cumulative per-entry guards across pauses.
ScanOutcome now reports complete/skipped entry counts, skipped lines and coverage_complete; the single-call scan convenience method drops its session at a boundary.
MCP uses the persistent API for [cursor pages](03-mcp-server.md#search-pages), never repeated random-access line reads.
