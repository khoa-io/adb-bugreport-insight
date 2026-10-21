# Archive safety design

ARCH-ARCHIVE handles untrusted ZIP metadata and content.
REQ-GEN-01/02 and REQ-MCP-06/08/09 motivate this design.
[Executable unit measures](../swe.4-unit-verification/01-verification-measures.md) and [fixture definitions](../verification/02-fixtures-and-environments.md) own the check selection.
Archive safety depends on explicit admission, decoding and retention bounds.

## Limits and enforcement points

[Constants](../../crates/core/src/caps.rs) set 10,000 entries, 256 MiB per entry, 1 GiB declared total, 2 GiB stored archive, ratio 1,000, 1 MiB retained line bytes, 64 KiB scratch, 8 KiB detection sample, 512-byte emitted UTF-8 excerpt text and 256-byte accepted marker input, with at most one overflow-probe byte for understated metadata.
List/grep defaults are 200 items, 50 matches, 32 MiB new source bytes per search page and 65,536 serialized tool-result bytes.
Search clients can lower scan budgets or choose response budgets within the [page contract](03-mcp-server.md#search-pages).
[ArchiveLimits](../../crates/core/src/limits.rs) exposes custom library limits, checked for coherence.
[Archive opening](../../crates/core/src/archive.rs) checks metadata; [reader progress guards](../../crates/core/src/reader.rs) enforce actual produced-byte limits during reads under the [lazy-validation decision](../decisions/0004-lazy-archive-validation.md).
The selected aggregate (1 GiB by default) applies to declared sizes only; unopened entries and unread tails remain unverified.
Line and chunk walks share Cursor validation before exposing a produced chunk, including skipped prefixes and discarded tails.
Byte-range reads check progress during both prefix discard and retained output.
Line/chunk refills request at most the remaining ceiling plus one probe byte, capped by scratch size; byte-range reads check each scratch-sized block.
These limits concern decompressed output observed through Read, not a bound on internal decoder work or CPU time.
A chunk iterator retains one borrowed decoder and reports over-limit output as an IO error containing ArchiveError; reaching the ceiling alone never signals EOF.
Assess streaming guarantees per reader method, not limit constant.

## Memory, decoding and addressing

Line skipping counts newlines in fixed scratch/carry buffers without retaining skipped lines.
Returned lines retain up to 1 MiB each; true byte_len_full and LineTooLong identify discarded tails.
Multi-line ranges and byte output still allocate retained results.
Search refills request no more than the remaining page source-byte budget; partial lines carry across pages and overlong lines are explicitly skipped, never searched only through a prefix.
A per-entry demand-driven worker preserves the decoder and cumulative safety guard across pages; internal decoder work remains outside source-byte accounting.
LineIndex is public; normal line reads do not seek via checkpoints.

EntryId addresses central-directory positions and distinguishes duplicate/non-UTF-8 names.
Display paths are decoded lossily with raw-byte validity/length flags; paths are never extraction targets.
Readers reject encryption, preserve unsupported compression codes and report corruption as error, never empty success.
Nested archive magic is reported without recursive opening.
Detection samples do not prove a classification for every byte.

Excerpt::from_bytes and LineSlice::to_excerpt decode byte sequences as UTF-8, preserving valid multibyte characters, replacing malformed sequences and substituting NUL with U+FFFD.
LineSlice::to_excerpt marks NUL-containing input as Binary, malformed input without NUL as Utf8Lossy, and valid input without NUL as Utf8; source bytes and offsets remain unchanged.
[UTF-8 regression evidence](../../verification/results/utf8-line-excerpts.md) covers the former byte-per-character corruption through both the core converter and real MCP line reads.
Both excerpt paths share a 512-byte emitted-text cap before JSON escaping, with separate source_bytes progress.
The converter inspects at most cap plus four source bytes, emits whole UTF-8 characters and maps each replacement to its consumed source span; it does not allocate/sanitize an entire retained line to create a small excerpt.
Clipping makes truncated true and complete false; malformed-byte/NUL replacement also makes complete false even without clipping.
The original line-retention limit remains a separate reader bound; overlong retained lines keep line_too_long, while excerpt-only clipping uses byte_budget.
[Collection outcomes and explicit detection failures](../decisions/0010-empty-results-and-detection-failures.md) are distinct from the [accepted excerpt cap](../decisions/0007-emitted-utf8-excerpt-cap.md); classification coverage never proves full-file validity.

## Failure and evidence boundaries

Absent entries, unreadable entries, empty successful search, startup failure and budget stops are different outcomes.
[MCP error design](./03-mcp-server.md) defines their actual wire paths.
`Archive::from_file` retains an already-authorized file descriptor; `open/open_with` open once and use the same path-independent source mechanism.
Indexing, hash reads and decompression use independent logical cursors over that descriptor, preventing a pathname/symlink replacement from redirecting later reads.
The immutable index is not a file snapshot against in-place external mutation.
Stored-file hashing establishes identity of bytes read, not causal diagnosis.
Reference measurements are recorded in [fixtures](../verification/02-fixtures-and-environments.md); they are not universal worst cases.

## Explicit raised policy

`ArchiveLimits::FORCED` raises finite archive limits to 100,000 entries, 4 GiB per entry, 16 GiB declared total, 8 GiB stored bytes and ratio 10,000.
The chosen policy governs both metadata admission and actual produced-byte checks, including prefixes discarded before a requested range.
The CLI warns on stderr whenever this policy is selected; an ordinary limit failure explains the measured/allowed values and suggests `--force`.
The override leaves line retention, marker validation, excerpt/page bounds, CRC handling, confirmation, path confinement and publication safeguards intact.
This is an explicit resource-use tradeoff, not an assertion that the entire ZIP is validated at open.
See REQ-GEN-02 and REQ-CLI-12 in [requirements](../swe.1-requirements/02-software-requirements.md).
