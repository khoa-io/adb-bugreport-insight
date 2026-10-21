# Usage

## MCP usage

Build from the repository root:

```bash
cargo build -p bugreport-insight-cli --offline
cargo run --bin bugreport-insight -- --mcp $BUGREPORT
```

Set `$BUGREPORT` to the archive path; configure client stdio command/arguments.
Ordinary MCP mode serves one ZIP; combined insight mode can serve a confined directory catalog.
The client must first send `initialize`, then `notifications/initialized`, and uses `tools/list`/`tools/call`; stdin/stdout are protocol pipes.
Closing client stdin ends the session.
`--verbose` adds diagnostics to stderr.
Use `--help` for available flags.
Supplying a positional archive without `--mcp` is a usage error.
For static artifacts, use [`--insight BUGREPORT`](01-usage.md#static-insight-usage); add `--mcp` to access analysis through insight tools.

For development, use Cargo; for MCP clients, pass the built executable and separate arguments so paths with spaces remain one argument:

```json
{"command":"/absolute/path/to/target/debug/bugreport-insight","args":["--mcp","/absolute/path/to/bugreport.zip"]}
```

Illustrative fields; enclosing configuration is client-specific.
Replace both paths with local paths.
The client launches and communicates with the process; a terminal launch waits for MCP input.
Keep the archive unchanged externally while the session runs.

### Choose a bounded call

| Step | Tool | Purpose |
| --- | --- | --- |
| Establish identity | `archive_info` | Archive shape and optional digest |
| Locate entries | `list_entries`, `find_entries`, `find_by_type` | IDs, path/type filters, and bounded lists |
| Inspect metadata | `entry_info` | Sizes, detection basis, and nested ZIP status |
| Read selected evidence | `read_entry` | One line-range or byte-range request for an entry ID |
| Search scoped content | `grep_archive` | Locator matches with offsets and coverage metadata |

Address entries with numeric `entry_id` values from index queries.
`path_prefix` filters a display subtree; it is not an entry address.
Literal and glob patterns are supported; requesting regex is refused.
For `read_entry`, supply exactly one complete range pair (`line_start`/`line_end` or `byte_start`/`byte_end`).
See the [MCP contract](../swe.3-detailed-design/03-mcp-server.md) for units and validation.

Inspect `status`, `truncated`, `stopped_reason`, `omitted_count`, and `notes` before drawing conclusions.
For grep, inspect data.scan and data.next_cursor; an empty page can have continuation, while complete coverage requires exhausted scope/delivery and no skipped content.
A truncated zero-match scan is inconclusive; narrow scope and retry.
`empty`, missing entries, unreadable entries, and tool failures are distinct.
[Qualification verdicts](../verification/assessment.md) define the demonstrated scope; this usage guide provides no additional conformance sign-off.

### Follow a locator

Content tools first return confirmation_required with a conservative cost plan and a 300-second token.
Review the plan, then explicitly retry the same normalized arguments with confirmation set to that token.
Every grep continuation page needs its own preview/retry; an earlier page's approval does not carry forward.
Resource methods use the same review flow but pass the token in _meta["bugreport-insight/confirmation"].
Tokens are single-use and bound to the operation and arguments; previews neither consume nor refresh grep cursors.
The examples below show the requested operation before that exchange; see the [confirmation contract](../swe.3-detailed-design/03-mcp-server.md#content-io-confirmation) for the error and retry shapes.

The examples below are the `params` value for an MCP `tools/call`, not complete JSON-RPC requests.
A client must complete the MCP handshake before calling tools: send `initialize`, wait for its response, then send `notifications/initialized`.
For a manual stdio session, send these JSON-RPC messages in order, one JSON object per line:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"manual-client","version":"1.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"find_entries","arguments":{"pattern":"*.txt","mode":"glob","limit":10}}}
```

Wait for the response to `initialize` before sending the notification or tool call; use a unique request `id` and match responses by that id.
The server expects MCP `initialize` before any other request; sending `tools/call` first can fail the transport and terminate the process.
Discover IDs first; replace illustrative `7` with an ID from your session.

The `params` object for `tools/call` is:

```json
{"name":"find_entries","arguments":{"pattern":"*.txt","mode":"glob","limit":10}}
```

`*` matches within one path segment; use `**/*.txt` when discovery must include nested paths.
Search a chosen ID literally so punctuation is not glob syntax:

```json
{"name":"grep_archive","arguments":{"pattern":"FATAL EXCEPTION","mode":"literal","entry_ids":[7],"max_matches":5}}
```

Read a match on line 42 with inclusive endpoints:

```json
{"name":"read_entry","arguments":{"entry_id":7,"line_start":42,"line_end":42}}
```

A byte request uses zero-based uncompressed offsets and an exclusive end: `byte_start: 0` and `byte_end: 64` request the first 64 bytes.
Do not mix the two range pairs.
Use small ranges and inspect data.past_end: EOF preserves available content with end_of_entry, while invalid endpoints and read/safety failures remain errors under ADR 0012.
Byte-path `bytes_returned` is the range read before excerpt clipping; it is not the length of the serialized excerpt.
Line, byte and grep excerpts share the 512-byte emitted UTF-8 cap; source-byte progress and full-line metadata retain their separate units under ADR 0007.

For failures, check the MCP error channel first.
A domain error contains `error.code` and structured data rather than the successful envelope fields; a protocol error rejects the call.
Missing success fields do not mean empty results.
Completed zero-match collection queries report empty with their normal data shape; unfinished/capped or incompletely classified collections report partial ok.

### Generated insight resources

The server also exposes the bound archive's seven generated insight artifacts through `resources/list` and `resources/read`.
Use [`--insight BUGREPORT`](01-usage.md#read-insight-through-mcp) first, then list resources and read the returned SHA-bound URI.
Resources return whole artifact text, including the README; the seven archive tools retain their existing behavior and bounds.
Missing output lists no resources, while malformed/foreign/oversized artifacts fail explicitly.
Reads are read-only and limited to 64 MiB per complete artifact.

## Static insight usage

Run `bugreport-insight --insight path/to/BUGREPORT.zip`.
From the workspace, use `cargo run --offline --bin bugreport-insight -- --insight path/to/BUGREPORT.zip`.
Add `--verbose` for additional stderr diagnostics.
The command reads the ZIP in place and publishes `path/to/.BUGREPORT/insight/`.

| File | Read for |
| --- | --- |
| `README.md` | Triage verdict, top findings, freshness, noise ledger, evidence and gaps. |
| `system.json` | Canonical capture/boot context, device/build facts and source inventory. |
| `apps.json` | Packages, user install states, processes and lifecycle attribution. |
| `anr.json` | Current incidents separately from historical reports and retained traces. |
| `crashes.json` | Deduplicated Java/native groups, classification thresholds and corroboration. |
| `memory.json` | RAM/PSS, swap timeline, procstats, pressure/kills and IO. |
| `cpu.json` | Sample windows, cumulative counters, sampled contention, graphics and power history. |

An existing `insight/` destination causes failure without replacement, including an empty directory.
Rename previous output before intentionally generating a new run.
A sibling `.BUGREPORT/` directory may already hold other files.
Failures remove staging and publish no partial final artifact set.
Exit codes are 0 for success, 2 for usage errors, and 1 for archive/analysis/output failures.
Add `--mcp` to enable insight tools instead of publishing files; a second positional archive remains a usage error.

Read freshness, source-channel coverage and `evidence_gaps` with every conclusion.
Retained ANR/tombstone files may describe earlier boots; absent records cannot establish device health.
CPU sample windows, since-boot counters and collector-time readings are kept separate.
Binary/nested archive contents are inventoried without recursive analysis.
Unsupported layouts or missing clocks remain explicit gaps, and budget violations fail the run.

The [requirements](../swe.1-requirements/02-software-requirements.md#static-insight-artifacts), [artifact schemas](../proposals/artifact-schemas/index.md), and [detailed design](../swe.3-detailed-design/05-insight-generation.md) own the contracts.
The [verification record](../../verification/results/insight-generation.md) records synthetic and real-report checks.

### Read insight through MCP

Generate first with `bugreport-insight --insight BUGREPORT`, then start `bugreport-insight --mcp BUGREPORT` for the same archive.
The MCP client calls `resources/list` to discover all seven artifacts and uses the returned URI in `resources/read`.
Each URI has the form `bugreport-insight://<archive-sha256>/insight/README.md` or the corresponding JSON filename.
Read returns the exact generated UTF-8 text with `text/markdown` or `application/json`; start with README and read only the relevant domain JSON next.

No output yet means an empty resource list; these resource methods never generate artifacts or write files.
Combined insight tools can generate the same artifacts in memory without requiring sibling output.
After generation completes, listing again discovers them without restarting the server.
Malformed, incomplete, symlinked, foreign-archive or oversized sets produce explicit errors.
Full resources are capped at 64 MiB each and never silently truncated.
The [MCP resource design](../swe.3-detailed-design/03-mcp-server.md#insight-resource-provider) specifies URI binding and error behavior.

## Excerpt follow-up

Each archive-tool excerpt is at most 512 emitted UTF-8 bytes before JSON escaping; text is sanitized before capping and whole characters are retained.
Excerpt.bytes counts emitted text; excerpt.source_bytes counts the represented source prefix, so continue at source byte start plus source_bytes.
For line/grep results the source start is byte_offset; for byte results it is the requested byte_start.
Clipping propagates to the success envelope without increasing omitted item/match counts; lossy replacement makes excerpt.complete false even without clipping.
Repeating a query starts a new search; passing its next_cursor resumes pagination.
Complete scan coverage does not imply complete excerpts.
[ADR 0007](../decisions/0007-emitted-utf8-excerpt-cap.md) owns this rule; marker policy follows [ADR 0008](../decisions/0008-validated-marker-metadata.md).

## Validated marker metadata

archive_info exposes optional version/main-entry values with per-marker status and source EntryId, rather than clipping identifiers.
Accepted input is at most 256 UTF-8 bytes without NUL; one terminal LF/CRLF is removed and every other space/character is preserved.
Unavailable markers leave values null while other metadata remains useful; oversized withholding is flagged in the envelope, and invalid/unreadable cases have explicit status/reasons.
Use the source entry_id with read_entry to investigate; a CRC-unreadable source can still fail while other requests continue.
Only missing version.txt establishes format 0; invalid markers remain unknown.
[ADR 0008](../decisions/0008-validated-marker-metadata.md) owns this contract.

### Search continuation

New grep calls accept response_budget (default 65,536 serialized MCP result bytes; range 2,048–262,144), scan_budget (1–33,554,432 new source bytes per page) and max_matches (1–50).

```json
{"name":"grep_archive","arguments":{"pattern":"FATAL","mode":"literal","entry_ids":[7],"response_budget":4096,"scan_budget":1048576}}
```

Follow the returned data.next_cursor until null, omitting query fields on continuation; page budgets can change.

```json
{"name":"grep_archive","arguments":{"cursor":"RETURNED_TOKEN","response_budget":4096,"scan_budget":1048576}}
```

Empty pages with a cursor mean no matches returned yet; scan.coverage_complete is the authority for full coverage.
Cursors are process-local, single-use, expire after 300 idle seconds, and at most eight searches are retained; finish searches to release capacity.
Lines exceeding 1 MiB and encrypted files are explicit skipped-content gaps even after all pages finish.
The response budget includes JSON escaping and MCP tool-result overhead, excluding the outer JSON-RPC frame/newline; increase it if an individual locator cannot fit.
[Canonical contract](../swe.3-detailed-design/03-mcp-server.md#search-pages) owns accounting and error details.

### Collection misses and classification failures

Completed list_entries, find_entries and find_by_type misses use status empty; a zero output limit withholding known matches remains partial ok with item_cap.
find_by_type returns successful matches alongside bounded failures with entry IDs and structured errors, and coverage counts successfully classified files, failed files and excluded directories.
Unknown is a successfully unknown classification, never a failed read; detection coverage concerns samples only, not complete file validation.
At most 200 matching records and 200 failure records are returned independently; omitted_matches and omitted_failures explain envelope omitted_count.
A failed classification forces partial ok and incomplete coverage even when all files fail; direct entry_info/read_entry requests still produce domain errors.
[ADR 0010](../decisions/0010-empty-results-and-detection-failures.md) and the [outcome contract](../swe.3-detailed-design/03-mcp-server.md#collection-outcomes-and-detection-coverage) own current details.

## Insight tools for a file or directory

```bash
bugreport-insight --mcp --insight BUGREPORT.zip
bugreport-insight --mcp --insight DIRECTORY
```

`--mcp` enables the MCP transport/tools; `--insight` enables insight access.
For a ZIP file, combined mode retains the seven archive tools and adds `list_bugreports`, `list_insights` and `read_insight`.
For a directory, only these three insight tools are available.
Discovery checks immediate regular ZIP children for bugreport markers/report names; it never visits parents, nested directories or child symlinks.
Use returned `bugreport` identifiers, not arbitrary filesystem paths.

Call `list_bugreports`, choose a returned identifier, then call `list_insights` and `read_insight`.
Both artifact operations return a confirmation preview first; review it and retry the same arguments with its token.
They analyze in memory without creating or overwriting an insight directory.
Read README.md first and then the relevant JSON artifact; `read_insight` defaults to 16 KiB per page, accepts at most 64 KiB and reports continuation byte offsets.
Inspect completeness fields and discovery rejections; an empty or capped catalog does not establish that every file was a valid report.
The directory scan inspects at most 1,000 immediate entries and reports at most 200 accepted/rejected records in total.
Directory mode requires Unix descriptor-relative file access; unsupported platforms reject this mode explicitly and can use a single ZIP instead.
For each `read_insight` continuation, send the returned `source_sha256` and `snapshot_id` with `next_byte_start` as `byte_start`; a changed source is an explicit error, so restart at byte zero after selecting the new report revision.
If `insight_snapshot_expired` reports an evicted artifact cache, restart the artifact at byte zero; do not concatenate pages across independent generations.

## Proceed with a larger bugreport

An over-limit report identifies the limit and suggests retrying with `--force`.

```bash
bugreport-insight --force --insight BUGREPORT.zip
bugreport-insight --force --mcp --insight DIRECTORY
```

The flag warns on stderr and raises resource ceilings to 100,000 entries, 4 GiB per entry, 16 GiB declared total, 8 GiB stored ZIP and compression ratio 10,000.
Insight retained text rises to 512 MiB and 8,000,000 lines per parsed entry.
These are finite ceilings: malformed archives, integrity errors, overlong retained lines, output conflicts and confined-path violations still fail.
MCP response/page bounds and per-call confirmation remain in effect.
If an MCP call reports a default limit, restart the server with `--force` to select the raised policy.
