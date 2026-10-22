# Software requirements

All committed requirements have stable IDs and explicit anchors.
Badges link [traceability](../traceability.md): obligations ↔ architecture ↔ design ↔ source ↔ measures.
Committed scope is distinct from demonstrated conformance; see the [qualification assessment](../verification/assessment.md).
Deferred obligations live in [proposals](../proposals/02-requirements.md).

## Contents

- [General](#general) — 2
- [Core library](#core-library) — 1
- [MCP server](#mcp-server) — 13
- [MCP tools](#mcp-tools) — 7
- [Command-line interface](#command-line-interface) — 4
- [Static insight artifacts](#static-insight-artifacts) — 5

## Intent and interpretation

[UC-01 through UC-03](01-use-cases.md) define committed intent: inspect, search, and analyze one bugreport with bounded, attributable evidence.
[Traceability](../traceability.md#committed-requirement-allocation) records separate library-reuse intent.
These requirements define the project obligation baseline; design rationale does not independently authorize scope changes.

The bullets and tool table below are binding obligations.
Shared tool constraints apply to all seven tool requirements where relevant.
Diagrams summarize those obligations; they do not add acceptance clauses.
`MiB` and `GiB` mean powers of 1024; bytes refer to stored ZIP bytes or uncompressed entry bytes as stated.
Text line numbers are 1-based and inclusive; byte offsets are zero-based in the uncompressed entry and byte-range ends are exclusive.

Acceptance requires comparing **every clause** with observed behavior, including failure and limit cases.
The [requirement verification page](../swe.6-software-verification/01-requirement-verification.md) defines review cases and candidate measures.
Its mappings are not pass verdicts.
Record newly discovered ambiguous or conflicting obligations with explicit task dispositions and decision/evidence links.

## General

- **Read-only ZIP access.**
  Open bugreport ZIP archives without extracting files to disk.
  <a id="req-gen-01"></a> ([`REQ-GEN-01`](../traceability.md#req-gen-01))
- **Safety limits.**
  Validation is lazy: opening an archive indexes metadata without decompressing entries.
  At open, reject more than **10,000** entries, a declared uncompressed entry size above **256 MiB**, a sum of declared uncompressed entry sizes above **1 GiB**, or a stored archive file size above **2 GiB**.
  During content reads, count actual decompressed bytes produced from the start of the selected entry, including skipped prefixes and discarded line tails; exceeding the selected per-entry limit (**256 MiB** by default) fails that operation before the violating output is delivered.
  All public line, byte-range, sequential-line and chunk-stream paths enforce this per-entry ceiling; a violation is an **error**, never successful EOF or silent truncation.
  Bounded read-ahead may inspect bytes beyond the requested range; the ceiling applies to produced output, not just returned bytes.
  The **1 GiB** aggregate is a declared-metadata admission limit, not a measurement of actual whole-archive expansion or cumulative session reads.
  An unopened entry or unread suffix remains unverified; successful open or a partial read does not certify all payload sizes, CRCs or readability.
  The CLI selects default limits unless `--force` selects the finite raised policy in REQ-CLI-12; the reusable library may use coherent custom limits.
  Metadata admission and produced-byte guards use the same selected policy; output and protocol response bounds remain independent.
  The [lazy-validation decision](../decisions/0004-lazy-archive-validation.md) records rationale and verification boundaries.
  <a id="req-gen-02"></a> ([`REQ-GEN-02`](../traceability.md#req-gen-02))

## Core library

`bugreport-insight-core` is reusable; `bugreport-insight` is one intended Rust consumer.

- **Library distribution.**
  Distribute the archive engine as a library so other Rust crates can depend on it.
  `crates/core` depends only on external crates and never on a workspace sibling.
  <a id="req-lib-01"></a> ([`REQ-LIB-01`](../traceability.md#req-lib-01))

## MCP server

`REQ-CLI-03` defines startup through `--mcp`.
This section defines server scope, tool contracts and evidence constraints.

- **Independent crate.**
  The MCP server is a reusable library crate distinct from the CLI crate; the CLI executable may link and launch that library.
  The CLI owns argument parsing and mode selection, and depends on both `bugreport-insight-mcp` and `bugreport-insight-core`.
  MCP owns protocol handling and depends on core; it must not depend on CLI, directly or transitively.
  Core has no production dependency on either consumer or another workspace sibling, preserving REQ-LIB-01.
  Development-only fixture dependencies do not count as production coupling.
  See the [accepted dependency rule](../decisions/0005-cli-composes-mcp-and-core.md).
  <a id="req-mcp-01"></a> ([`REQ-MCP-01`](../traceability.md#req-mcp-01))
- **Single-archive scope.**
  Ordinary `--mcp BUGREPORT` binds one archive for the session lifetime.
  Combined `--mcp --insight PATH` additionally supports the confined insight scope in REQ-MCP-13; its discovery and insight queries never expand that scope.
  <a id="req-mcp-02"></a> ([`REQ-MCP-02`](../traceability.md#req-mcp-02))
- **Read-only over the archive.**
  The server reads the archive in place and never extracts it, under the same size and entry-count limits as `REQ-GEN-01` and `REQ-GEN-02`.
  Exposing an archive to an agent does not relax the read-only guarantee.
  <a id="req-mcp-03"></a> ([`REQ-MCP-03`](../traceability.md#req-mcp-03))
- **Read-only tool surface.**
  No tool mutates the archive, any output directory, or other persistent external state.
  Bounded process-local search continuations and request-confirmation tokens are permitted by ADR 0009 and ADR 0013.
  Combined insight tools may analyze an authorized archive into process memory; they never publish files or modify existing artifacts.
  <a id="req-mcp-04"></a> ([`REQ-MCP-04`](../traceability.md#req-mcp-04))
- **Tool naming and typing.**
  Every tool is named, described, and declared with a typed input schema (JSON Schema, derived via `schemars`) and a documented result shape, so an agent can call it without reading the implementation.
  Regression tests assert the `tools/list` response carries exactly the tool names enabled by the selected mode, each once, with descriptions and typed schemas.
  Discovery order is unspecified; clients select tools by name.
  Recommended call sequencing remains in initialization instructions under REQ-MCP-11, as recorded in [ADR 0011](../decisions/0011-unordered-tool-discovery.md).
  <a id="req-mcp-05"></a> ([`REQ-MCP-05`](../traceability.md#req-mcp-05))
- **Bounded responses.**
  Every successful archive-tool result, including empty or partial success, carries `truncated: bool`, `stopped_reason: enum | null`, `omitted_count: int ≥ 0`, and `notes: string[]` — **all four always present, never omitted**.
  Domain failures instead carry MCP `isError: true` and structured `error.code`, `error.message`, and `error.data`, preserving addressed-entry context when available; they do not carry the successful envelope or its four completeness fields.
  Protocol and framework failures use their rejection/error channels without a successful envelope; framework parameter failures may use a tool-error result rather than JSON-RPC error.
  A failed tool call is never presented as a complete empty or partial successful result.
  Confirmation challenges under REQ-MCP-12 are separate errors, not successful content results; the grep response budget applies to successful pages rather than those challenges.
  Successful collection queries may retain per-entry failures as explicit partial outcomes under REQ-MCP-08.
  `stopped_reason` ∈ `{item_cap, match_cap, scan_budget, line_too_long, byte_budget, skipped_content, end_of_entry}`.
  Defaults: **200** matching items per list (`list_entries`, `find_entries`, `find_by_type`), **50** grep matches (`grep_archive`), **32 MiB** new source bytes scanned per grep page.
  find_by_type failure details have a separate 200-record cap under REQ-MCP-08.
  Clients select response_budget (default 65,536; range 2,048–262,144 serialized MCP tool-result JSON bytes), scan_budget (1–33,554,432 new decompressed source bytes) and max_matches (1–50); invalid budgets are refused.
  The response bound includes envelope text escaping and cursor overhead, excluding the outer JSON-RPC frame and transport newline; an indivisible locator that cannot fit requires a larger budget.
  grep returns next_cursor until search/delivery exhaustion; continuation binds the original query and scope, permits changed budgets, and preserves decoder/line progress without replay or missed/duplicated matching lines.
  Empty pages can carry continuation; unknown remaining matches are never reported as a known omitted count.
  Coverage is complete only after exhausted scope and pending delivery with no skipped file/line content.
  Partial lines survive page boundaries; lines over 1 MiB are explicitly skipped and counted, while later lines remain searchable.
  At most eight process-local continuations exist per handler, expiring after 300 idle seconds and reclaimed on subsequent calls or shutdown; tokens are single-use.
  [ADR 0009](../decisions/0009-client-search-budgets-and-pagination.md) owns cursor lifecycle, accounting and coverage details.
  Each line, byte-range or grep-match excerpt is capped at **512 UTF-8 bytes of emitted text after decoding/sanitization, before JSON escaping**, preserving whole characters.
  The cap applies per excerpt, not to the entire MCP response or decompression work.
  Excerpt `bytes` is the emitted text length; `source_bytes` is the source-prefix length represented by that text, excluding any line terminator.
  Offsets, original line lengths and byte-range `bytes_returned` retain source-byte units; clients continue at the source start plus `source_bytes`, never by adding emitted `bytes` after lossy expansion.
  Clipping sets excerpt `truncated: true` and `complete: false`, and propagates truncation to the successful tool envelope; excerpt-only clipping uses `byte_budget` and does not increase the omitted item/match count.
  Existing scan/item/retention stop reasons take precedence over excerpt-only clipping; original source-range/full-line metadata remains available.
  Decoding replacement sets `complete: false` with explicit decoding classification even when no clipping occurred.
  [ADR 0007](../decisions/0007-emitted-utf8-excerpt-cap.md) defines the accepted byte-accounting rule.
  A capped result set reports what it omitted.
  Content-reading/search tools return located excerpts rather than unrestricted archive sections.
  Discovery and archive metadata use structured facts without artificial content locators; bounded format/main-entry marker text is an explicit metadata exception.
  Version/main-entry markers accept at most **256 actual source bytes**, including any terminal line ending; declared sizes above 256 reject the marker without reading its payload, and bounded streaming checks actual output when a declared size is smaller.
  Valid marker text must be UTF-8 without NUL; remove exactly one terminal LF or CRLF, preserving all other spaces and characters without numeric parsing.
  Oversized, invalid-UTF-8, NUL-containing or unreadable markers yield null values plus explicit per-marker status, source entry ID when present, declared size and a machine error code; never return a clipped or replacement-decoded identifier.
  archive_info remains a successful metadata result when a marker is unavailable, with explanatory notes; oversized-marker withholding sets envelope truncated true, byte_budget and omitted_count zero, while invalid/unreadable markers use their explicit validation status without claiming clipping.
  Missing markers are explicitly missing; only absent version.txt retains format-0 handling, never an invalid/oversized/unreadable version marker.
  Strict UTF-8 and removal-only normalization also keep emitted marker text within 256 bytes.
  [ADR 0008](../decisions/0008-validated-marker-metadata.md) defines marker validation and follow-up semantics.
  These archive-tool response rules do not replace the complete-artifact MCP resource contract in REQ-MCP-07.
  [ADR 0006](../decisions/0006-tool-response-scope.md) defines the success/error and metadata boundaries.
  Measurement point: the result envelope returned to the agent over stdio.
  <a id="req-mcp-06"></a> ([`REQ-MCP-06`](../traceability.md#req-mcp-06))
- **Empty is not error.**
  A completed zero-match collection query (list_entries, find_entries, find_by_type and complete grep pages) returns status empty with its normal data shape and mandatory completeness fields.
  A cap that withholds known matches, incomplete classification or unfinished grep page returns partial ok, even if its returned collection is empty.
  find_by_type retains successful matches alongside explicit bounded failure records with entry IDs and structured error code/message/data; failures never become confirmed non-matches or Unknown classifications.
  Type-query coverage counts successfully classified files, failed files and excluded directories; complete coverage concerns promised detection samples, not whole-file validation.
  Matching records are capped by limit (default/max 200); failure records are independently capped at 200, with separate omitted_matches/omitted_failures and envelope omitted_count equal to their sum.
  Failed classifications use skipped_content unless item_cap takes precedence for withheld records; all-failed collections remain partial ok with incomplete coverage.
  Direct absent/unreadable entry requests retain their separate domain-error shape; invalid parameters remain rejected calls.
  [ADR 0010](../decisions/0010-empty-results-and-detection-failures.md) defines the accepted collection and classification policy.
  `status` is one of `ok`, `empty`, `not_found`, `unreadable`, and a failure is never shaped like a successful empty result.
  <a id="req-mcp-08"></a> ([`REQ-MCP-08`](../traceability.md#req-mcp-08))
- **Evidence discipline.**
  Successful content-reading/search results state source-content evidence as a locator — archive `entry_id`, kind (`text` | `binary` | `nested_zip`), line or byte range with `byte_offset` included for `grep_archive`, and a bounded excerpt capped at **512 bytes** with truncation and completeness flagged — rather than unrestricted source content.
  Valid UTF-8 source text must retain its characters when converted into an excerpt; individual UTF-8 bytes must not be interpreted as separate characters.
  Such locators use `entry_id`, never a raw path as the entry address.
  Discovery/archive metadata is structured information, not a content excerpt, and does not require artificial line/byte locators; display paths remain metadata.
  Domain and protocol/framework failures use the error contracts in REQ-MCP-06; complete insight resources use REQ-MCP-07.
  <a id="req-mcp-09"></a> ([`REQ-MCP-09`](../traceability.md#req-mcp-09))
- **stdio is the required transport.**
  The server speaks MCP over stdio.
  SSE and HTTP transports are not part of this requirement set and are not implemented.
  stdout carries JSON-RPC frames and nothing else; every diagnostic goes to stderr, and `--verbose` sets the stderr level only.
  <a id="req-mcp-10"></a> ([`REQ-MCP-10`](../traceability.md#req-mcp-10))
- **`initialize` carries `instructions`.**
  The `initialize` response carries `instructions`: at most **40 lines** naming each enabled tool, its actual work/cost class, its bound, and the intended call order.
  Guidance distinguishes index queries, full stored-file hashing, repeated bounded marker reads, per-file samples, skipped-prefix decompression and per-page scanning; output bounds do not imply equivalent IO bounds or a universal execution-cost ranking.
  It explains per-call confirmation, tool/resource retry fields and independent approval for every search page; initialization/discovery itself reads no archive payload or full hash.
  It publishes no archive content, and it is the one place a new agent learns the intended order without spending a tool call to discover it.
  <a id="req-mcp-11"></a> ([`REQ-MCP-11`](../traceability.md#req-mcp-11))
- **Confirm potentially expensive MCP operations.**
  Before archive_info, entry_info, find_by_type, read_entry, each grep_archive page, resources/list or resources/read performs content IO, return a confirmation_required challenge containing operation, confirmation_token, cost and expires_in_seconds.
  No hash, entry sample, marker read, range decompression, content scan or artifact read occurs before that request is confirmed; archive-open metadata admission, initialize, tools/list, list_entries and find_entries remain immediate.
  The client reviews the conservative source/stored-byte estimates and retries the same operation/normalized arguments with the token only when it accepts the work; there is no automatic production retry or inherited approval.
  Tool challenges use isError with error.code/message/data; resource challenges use JSON-RPC -32000 with code and challenge fields in error.data, without successful data/completeness claims.
  Tools take optional confirmation; resources take _meta["bugreport-insight/confirmation"].
  Tokens are opaque, process-local, single-use and expire 300 seconds after issuance; exact operation/argument mismatch, expiry, replay or foreign tokens yield invalid_confirmation and do not consume another matching pending request.
  The five content tools share one 64-token pool; the resource methods share a separate 64-token pool; a full pool refuses new previews with confirmation_capacity and performs no work.
  Each bound argument object is limited to 65,536 normalized serialized JSON bytes; larger requests are refused with confirmation_arguments_too_large before retaining state or executing work.
  Expired tokens are reclaimed on subsequent valid gate checks or shutdown; normalized tool arguments include adapter defaults, exclude confirmation, and resource binding includes the URI for reads.
  Confirmation does not certify semantic validity, reserve a grep cursor, refresh its independent idle expiry or guarantee CPU/elapsed-time/physical-IO cost; approved execution can still fail, consuming its approval token.
  [ADR 0013](../decisions/0013-confirmation-before-content-io.md) and the [detailed protocol](../swe.3-detailed-design/03-mcp-server.md#content-io-confirmation) own the request/retry contract.
  <a id="req-mcp-12"></a> ([`REQ-MCP-12`](../traceability.md#req-mcp-12))

- **Insight artifact resources.**
  The server bound to `BUGREPORT` exposes its already-generated sibling insight artifacts through MCP `resources/list` and `resources/read`, including the README and all six JSON files.
  Resource URIs are `bugreport-insight://<archive-sha256>/insight/<filename>`; only the seven fixed filenames are addressable.
  Read returns the exact UTF-8 artifact text with Markdown/JSON MIME type, not a summary or silently truncated content.
  A missing final directory lists no resources; unknown URIs, missing/incomplete sets, symlinks, mismatched archive identity, invalid UTF-8 and oversized content are explicit errors.
  Resources are read-only and never generate artifacts or mutate the archive/output; run `--insight BUGREPORT` first.
  Each resource is limited to 64 MiB, enforced on metadata and actual reads; over-limit content fails without returning a prefix.
  List returns metadata for the complete seven-file set without embedding content, and verifies the system artifact's archive SHA-256.
  Read additionally verifies the selected JSON's source identity or the README's SHA-256 header.
  The single-archive server advertises resources alongside its enabled tools; it advertises no subscriptions or list-change notifications.
  <a id="req-mcp-07"></a> ([`REQ-MCP-07`](../traceability.md#req-mcp-07))

- **Insight tools and confined discovery.**
  `--mcp --insight FILE` exposes the archive tools and insight tools for that one file; `--mcp --insight DIRECTORY` exposes insight discovery and insight access for immediate bugreport ZIP children only.
  Directory discovery never recurses, searches parents, follows child symlinks, or accepts tool-supplied filesystem paths; archive selection uses identifiers returned by discovery.
  Directory sessions pin an opened directory and open regular children relative to that handle without following symlinks, so a path replacement cannot redirect a later content read.
  Directory mode requires a descriptor-relative implementation: Unix pins a directory descriptor and Windows pins a directory handle that enumerates through `GetFileInformationByHandleEx` and opens children with `FILE_FLAG_OPEN_REPARSE_POINT`.
  Both refuse a child symlink or reparse point rather than following it, and both keep serving the pinned directory after it is renamed.
  Platforms with neither implementation fail startup explicitly as unsupported while single-file mode remains available.
  A ZIP extension alone is insufficient: detection requires a root non-directory/non-symlink `main_entry.txt` or `bugreport*.txt` ZIP entry; `version.txt` alone is insufficient, and no payload is extracted to disk.
  Invalid, ordinary, nested-directory and symlink ZIP candidates are excluded; discovery reports bounded results and explicit admission failures rather than silently treating over-limit bugreports as absent.
  `list_bugreports` provides discovery (at most 1,000 directory entries inspected, 200 total accepted/rejected records); `list_insights` lists the seven generated artifact names; `read_insight` reads a selected README/domain artifact generated in memory, reusing committed artifact/evidence contracts.
  Expensive analysis requires per-call confirmation under REQ-MCP-12; the cached generated artifact set is limited to 64 MiB in aggregate; reads return explicit UTF-8 byte pages (default 16 KiB, maximum 64 KiB), source offsets and continuation/completeness metadata, never silent truncation.
  Every successful insight page identifies its source SHA-256 and process-local artifact snapshot ID; continuation at a nonzero byte offset requires both, and changed sources or regenerated snapshots are refused instead of joining evidence from different analyses.
  A continuation whose generated artifact set was evicted fails explicitly and must restart at byte zero, preventing pages from independently generated artifact sets from being joined.
  Selection outside the discovered scope, unknown artifact names and analysis failures produce explicit structured errors.
  <a id="req-mcp-13"></a> ([`REQ-MCP-13`](../traceability.md#req-mcp-13))

- **Archive tools for selected directory reports.**
  `--mcp --insight DIRECTORY` exposes the existing seven archive tools alongside the three insight tools.
  Every archive tool accepts an optional exact `bugreport` basename; use an identifier from `list_bugreports` when it is present, and never pass a filesystem path.
  In a directory session, an omitted selector is resolved only when complete discovery finds exactly one eligible report; no eligible report returns `no_bugreports`, while multiple reports or incomplete discovery returns `bugreport_required`.
  A supplied selector must resolve to a recognized immediate regular ZIP under the pinned directory, with the existing no-follow and marker checks.
  In a bound-file session, omission continues to use that file and a supplied selector must match its basename.
  Each successful directory archive-tool result includes the selected basename; `entry_id` values are local to that report, and every `grep_archive` continuation must retain the same selector.
  Content confirmation remains in the session's shared 64-token pool and also binds to the retained report-context generation, so reopening a basename cannot reuse an approval for an earlier archive snapshot.
  Existing read-only, archive safety, result-bound and per-call content-confirmation contracts remain in force.
  The server retains at most eight pinned archive-tool contexts; idle contexts expire after 300 seconds, and a full active cache returns an explicit `archive_context_capacity` error.
  <a id="req-mcp-14"></a> ([`REQ-MCP-14`](../traceability.md#req-mcp-14))

## MCP tools

Each archive tool is a product capability with its own ID; REQ-MCP-13 covers the three insight tools and REQ-MCP-14 covers archive selection in directory sessions.
Seven tools map one-to-one to `crates/core` listing, reading, finding, grepping and metadata; **none runs an analyzer**.
The [tool allocation](./allocations/03-mcp-tools.md) summarizes this table; the shared constraints below complete the tool obligations.

| ID | Tool | Returns |
| --- | --- | --- |
| <a id="req-mcp-arc-01"></a>[`REQ-MCP-ARC-01`](../traceability.md#req-mcp-arc-01) | `archive_info` | Archive identity and shape: basename, size, optional SHA-256 over the stored bytes, entry count, validated version/main-entry marker values and statuses, and the named main entry's size |
| <a id="req-mcp-arc-02"></a>[`REQ-MCP-ARC-02`](../traceability.md#req-mcp-arc-02) | `list_entries` | Entry paths and sizes, optionally scoped to one subtree prefix |
| <a id="req-mcp-arc-03"></a>[`REQ-MCP-ARC-03`](../traceability.md#req-mcp-arc-03) | `find_entries` | Paths matching a glob or literal, with no content |
| <a id="req-mcp-arc-04"></a>[`REQ-MCP-ARC-04`](../traceability.md#req-mcp-arc-04) | `read_entry` | A bounded line or byte range of one entry |
| <a id="req-mcp-arc-05"></a>[`REQ-MCP-ARC-05`](../traceability.md#req-mcp-arc-05) | `grep_archive` | Content search, returning matches as locators |
| <a id="req-mcp-arc-06"></a>[`REQ-MCP-ARC-06`](../traceability.md#req-mcp-arc-06) | `entry_info` | One entry's compressed and uncompressed size, compression method, modification time, detected content type, and nested-archive status |
| <a id="req-mcp-arc-07"></a>[`REQ-MCP-ARC-07`](../traceability.md#req-mcp-arc-07) | `find_by_type` | Paths whose entries are of a given detected content type, with the basis for each detection |

Shared tool constraints:

- **Archive digest choice is explicit and cached.**
  archive_info accepts with_sha256, default true; false permits a null digest without hashing.
  The archive instance caches its first successful identity choice, including a declined digest; a later true request does not upgrade a cached null digest.
  Concurrent uncached requests may compute before one cache value wins, so this is not a guarantee of exactly one physical hash read.
  The resource provider obtains its mandatory SHA-bound identity separately when necessary, as described in [ADR 0013](../decisions/0013-confirmation-before-content-io.md).
- **Addressing is by `entry_id`, always.**
  Tools that address a particular entry take an `entry_id: u32` from `list_entries` or `find_entries`.
  Paths are not entry-addressing keys; path filters remain supported.
  This removes path traversal, encoding mismatch, duplicate entry names, and glob-versus-literal ambiguity in one move.
  `list_entries` accepts an optional `path_prefix`, which is a display convenience scoped to one subtree and does not change how results are addressed.
  ANR and ANR/ select the same subtree, including its directory record if present, and exclude sibling ANR2; matching requires an exact path or a slash segment boundary.
  Empty or slash-only prefixes select all entries; trailing slashes are ignored, without resolving dot segments or changing path case.
- **Format scope is ZIP only.**
  `crates/core` implements no TAR.
  Multi-format support is [`DRAFT-REQ-MCP-ARC-09`](../proposals/02-requirements.md#draft-req-mcp-arc-09).
- **Pattern modes are literal and glob.**
  `regex` is a typed refusal, never a silent fallback to literal: [`DRAFT-REQ-MCP-ARC-08`](../proposals/02-requirements.md#draft-req-mcp-arc-08).
- **`read_entry` takes flat range fields.**
  Exactly one of (`line_start`, `line_end`) or (`byte_start`, `byte_end`) must be specified.
  The mutual exclusion is a validation error rather than an enum, because an externally-tagged enum produces an `inputSchema` with no root `"type"`, which is not a schema the MCP specification permits.
  A range past end-of-entry returns available content with data.past_end true, truncated true, stopped_reason end_of_entry and omitted_count zero; no nonexistent content is counted as omitted.
  Otherwise data.past_end is false; excerpt clipping remains independently visible on each excerpt.
  Line endpoints are inclusive and require 1 <= start <= end; byte endpoints are half-open and require start <= end.
  Invalid endpoints are invalid_range parameter errors; equal byte endpoints are valid empty ranges.
  A wholly past-end request returns an empty payload with the same explicit EOF indication, retaining successful status Ok.
  Read, CRC and safety failures remain errors, rather than partial EOF successes.
  [ADR 0012](../decisions/0012-range-boundaries-and-subtree-filters.md) records these boundary rules.
- **Content detection reports its basis.**
  An entry is called text or binary from its bytes; a more specific type is claimed only when archive metadata or byte content supports it.
  An unidentifiable entry is reported as unidentifiable, never guessed at.

## Command-line interface

- **`--mcp`.**
  `bugreport-insight BUGREPORT --mcp` starts a single-archive MCP server; `bugreport-insight --mcp --insight PATH` additionally enables insight tools for a ZIP file or a confined directory (`REQ-MCP-13`).
  Both modes speak MCP over stdio (`REQ-MCP-10`).
  The flag is the interface contract; [the MCP server section](#mcp-server) is the implementation contract.
  The archive is opened **before** the transport starts, so a server that cannot see its archive exits non-zero with a diagnostic on stderr rather than answering as though the archive were empty.
  <a id="req-cli-03"></a> ([`REQ-CLI-03`](../traceability.md#req-cli-03))
- **`--help`.**
  Display usage information and available options.
  <a id="req-cli-04"></a> ([`REQ-CLI-04`](../traceability.md#req-cli-04))
- **`--verbose`.**
  Enable verbose logging for debugging.
  It sets the log level on **stderr** via `eprintln!`; stdout carries JSON-RPC under `--mcp` and is never a logging channel.
  <a id="req-cli-05"></a> ([`REQ-CLI-05`](../traceability.md#req-cli-05))

- **Explicit raised limits.**
  Without `--force`, an exceeded default archive or retained-text byte/line limit fails with an actionable diagnostic identifying the limit and suggesting `--force`.
  `--force` selects finite raised limits: 100,000 entries, 4 GiB declared/produced bytes per entry, 16 GiB declared aggregate, 8 GiB stored ZIP, and compression ratio 10,000.
  Insight retained text increases from 128 MiB and 2,000,000 lines per text entry to 512 MiB and 8,000,000 lines; the 1 MiB retained-line, 20,000 section, 50,000 region, evidence and output bounds remain unchanged.
  The CLI always warns on stderr that raised limits increase resource use when `--force` is selected; MCP error results identify exceeded limits and the startup flag needed to change policy.
  Force does not bypass malformed ZIP errors, integrity checks, filesystem confinement, confirmation, read-only guarantees or output replacement protections; exceeding the raised limits still fails.
  <a id="req-cli-12"></a> ([`REQ-CLI-12`](../traceability.md#req-cli-12))

Exit codes are 0 for success and normal stdin EOF, 2 for bad usage, and 1 for archive, transport, server-task, analysis, or output failure.
Normal stdin EOF exits 0 whether it occurs before or after MCP initialization.
Transport startup/IO failures and MCP server-task failures exit 1 with an accurate stage-specific diagnostic on stderr, including a broken stdout response pipe.
Server-task failures include both failure of the SDK service task and a failed SDK send task reported inside its completion reason; neither is a normal EOF outcome.
An individual invalid or failed MCP request does not by itself terminate a session that can continue processing later requests.

The other analysis toggles remain [draft](../proposals/02-requirements.md#command-line-interface-draft).

## Static insight artifacts

`--insight` alone publishes static artifacts; combined with `--mcp`, it enables read-only insight tools under REQ-MCP-13.

- **Single-report command.**
  `bugreport-insight --insight BUGREPORT` accepts exactly one ZIP path for artifact publication.
  Combined `--mcp --insight PATH` starts an MCP session instead of publishing artifacts; providing both an insight path and a second positional archive remains a usage error.
  It runs system, apps, ANR, crash, memory, and CPU analysis without per-domain switches.
  The output directory is `<archive-parent>/.<archive-file-stem>/insight/`.
  It writes diagnostics to stderr and no analysis payload to stdout.
  <a id="req-insight-01"></a> ([`REQ-INSIGHT-01`](../traceability.md#req-insight-01))
- **Non-destructive publication.**
  The archive is read in place under `REQ-GEN-01` and `REQ-GEN-02` and remains byte-identical.
  If the final output directory already exists, the command fails without modifying it.
  The command publishes all artifacts together; a failed run does not leave a partial final artifact set.
  <a id="req-insight-02"></a> ([`REQ-INSIGHT-02`](../traceability.md#req-insight-02))
- **Complete artifact set.**
  A successful run creates exactly `README.md`, `system.json`, `apps.json`, `anr.json`, `crashes.json`, `memory.json`, and `cpu.json` in the output directory.
  Each JSON file parses as one JSON object and follows the corresponding committed contract in [`artifact-schemas/`](../proposals/artifact-schemas/index.md).
  <a id="req-insight-03"></a> ([`REQ-INSIGHT-03`](../traceability.md#req-insight-03))
- **Evidence-based findings.**
  Every finding and count follows the promoted schemas' evidence, freshness, identity, coverage, and uncertainty rules.
  The output does not claim a cause that its recorded evidence supports only as correlation, does not silently discard stale or undated evidence, and represents unavailable or incomplete evidence explicitly.
  <a id="req-insight-04"></a> ([`REQ-INSIGHT-04`](../traceability.md#req-insight-04))
- **Bounded presentation.**
  The README is at most 400 lines, the top-findings table at most ten rows, and every evidence excerpt at most 512 bytes with elision flagged.
  The command obeys the archive entry and expansion limits in `REQ-GEN-02`; no output contains an unbounded copy of an archive section.
  <a id="req-insight-05"></a> ([`REQ-INSIGHT-05`](../traceability.md#req-insight-05))

The output conflict policy is fail-without-overwrite so reruns cannot silently replace prior analysis; users can remove or rename the prior output before a new run.
The proposal's manifest, NDJSON batch output, and session segmentation remain deferred because they have no committed schema and are not part of `UC-03`.

## Requirement risk

Risk selects verification; it is not a conformance verdict.
Executable measure candidates and their assertion limits are in [SWE.6](../swe.6-software-verification/01-requirement-verification.md).

Ratings express engineering impact, not probability or automotive safety classification.
High risk covers archive mutation/resource exhaustion and misleading evidence, status or bounds: review both successful and hostile/partial outcomes.
Medium covers reusable boundaries, metadata, discovery and lifecycle: inspect interfaces and representative integration paths.
Low covers help and diagnostic usability: focused process checks suffice, while stdout purity is also reviewed under REQ-MCP-10.
Ratings do not downgrade known conflicts.

| Requirement | Risk |
| --- | --- |
| [REQ-GEN-01](#req-gen-01) | high |
| [REQ-GEN-02](#req-gen-02) | high |
| [REQ-LIB-01](#req-lib-01) | medium |
| [REQ-MCP-01](#req-mcp-01) | medium |
| [REQ-MCP-02](#req-mcp-02) | medium |
| [REQ-MCP-03](#req-mcp-03) | high |
| [REQ-MCP-04](#req-mcp-04) | high |
| [REQ-MCP-05](#req-mcp-05) | high |
| [REQ-MCP-06](#req-mcp-06) | high |
| [REQ-MCP-08](#req-mcp-08) | high |
| [REQ-MCP-09](#req-mcp-09) | high |
| [REQ-MCP-10](#req-mcp-10) | medium |
| [REQ-MCP-11](#req-mcp-11) | medium |
| [REQ-MCP-12](#req-mcp-12) | high |
| [REQ-MCP-13](#req-mcp-13) | high |
| [REQ-MCP-14](#req-mcp-14) | high |
| [REQ-MCP-ARC-01](#req-mcp-arc-01) | medium |
| [REQ-MCP-ARC-02](#req-mcp-arc-02) | medium |
| [REQ-MCP-ARC-03](#req-mcp-arc-03) | medium |
| [REQ-MCP-ARC-04](#req-mcp-arc-04) | high |
| [REQ-MCP-ARC-05](#req-mcp-arc-05) | high |
| [REQ-MCP-ARC-06](#req-mcp-arc-06) | medium |
| [REQ-MCP-ARC-07](#req-mcp-arc-07) | medium |
| [REQ-CLI-03](#req-cli-03) | medium |
| [REQ-CLI-04](#req-cli-04) | low |
| [REQ-CLI-05](#req-cli-05) | low |
| [REQ-CLI-12](#req-cli-12) | high |
| [REQ-MCP-07](#req-mcp-07) | medium |
| [REQ-INSIGHT-01](#req-insight-01) | medium |
| [REQ-INSIGHT-02](#req-insight-02) | high |
| [REQ-INSIGHT-03](#req-insight-03) | medium |
| [REQ-INSIGHT-04](#req-insight-04) | high |
| [REQ-INSIGHT-05](#req-insight-05) | high |
