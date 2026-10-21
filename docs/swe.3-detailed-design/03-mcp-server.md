# MCP server detailed design

## Ownership and parameters

ARCH-MCP-SRV owns lifecycle/transport/error translation; ARCH-MCP-TOOLS-ARC owns seven handlers.
The CLI composes this library through its public serve entry point for MCP mode, and invokes core directly for insight mode.
MCP depends on core and has no production dependency on CLI; core has no production dependency on either consumer.
Linking MCP into the CLI executable satisfies the separate-crate requirement; [ADR 0005](../decisions/0005-cli-composes-mcp-and-core.md) records this composition boundary.
[Registry](../../crates/mcp/src/registry.rs) returns ToolHandler objects sharing one Arc<Archive>.
ToolHandler::call accepts serde_json::Value and returns ToolEnvelope or ToolError without rmcp types.
[Adapter](../../crates/mcp/src/adapter.rs) derives tool routing, invokes the handlers synchronously, and converts their outcomes.
Ordinary archive mode tools/list returns exactly the seven unique tool names below with descriptions and typed schemas; combined mode adds the insight tools described below; array order is unspecified under [ADR 0011](../decisions/0011-unordered-tool-discovery.md).
Clients dispatch by name; neither this table nor the handler vector promises discovery order.
Initialization instructions retain recommended call sequencing under REQ-MCP-11.
[Runtime design](../swe.2-architecture/03-runtime-and-interactions.md) owns startup, shared-state and IO sequencing.

[Parameter structs](../../crates/mcp/src/tools/params.rs) derive JsonSchema and deny unknown fields.
All tools have a root object schema.
The five content tools additionally accept optional confirmation, consumed by the registry wrapper before handler execution; index-only list_entries/find_entries remain immediate.
ReadEntryParams has four optional flat fields; range() requires exactly one complete line/byte pair.
Missing/partial/both pairs are refused with conflicting_range_params.
Flat fields keep the tool input schema a root object.

| Tool | Parameters | Success data |
| --- | --- | --- |
| archive_info | with_sha256, default true in serde decoding | archive identity |
| list_entries | optional path_prefix and limit | entries, entry_count |
| find_entries | pattern, optional mode and limit | entries |
| entry_info | entry_id | entry metadata, detection |
| find_by_type | content_type, optional limit | matches with entry/detection, failures, omission counts and sample coverage |
| read_entry | entry_id and one range pair | entry_id, range and lines or byte excerpt/bytes_returned |
| grep_archive | new query: pattern, optional mode/entry_ids; continuation: cursor; optional response_budget/scan_budget/max_matches on either | matches, scan coverage and next_cursor |

path_prefix matches an exact path or a slash segment boundary after removing trailing slashes; ANR and ANR/ exclude ANR2.
An empty or slash-only prefix selects all entries; no case folding or dot-segment normalization occurs.
Paths remain display filters, not keys for reading individual entries.
Line endpoints are inclusive, byte_end exclusive.
Both read modes return data.past_end; requested range fields retain the original endpoints.
Past-end reads preserve available content and use Ok, truncated true, stopped_reason end_of_entry and omitted_count zero, including an empty payload when wholly past EOF.
Exact EOF and equal byte endpoints within the entry are not past-end; zero/reversed line endpoints and reversed byte endpoints produce invalid_range before opening a reader.
EOF takes envelope reason precedence over excerpt/long-line clipping, which remains visible on the affected excerpts.
Read/CRC/safety failures are errors; EOF success does not certify unread content under lazy validation.
[ADR 0012](../decisions/0012-range-boundaries-and-subtree-filters.md) owns the boundary policy.
Literal/glob modes are accepted; regex is refused.
List limits clamp to 200; grep rejects max_matches outside 1–50.
Whole content samples and ranges have distinct costs; [runtime IO](../swe.2-architecture/03-runtime-and-interactions.md#shared-state-and-actual-io) and request previews describe actual operations rather than ranking output sizes as execution cost.

Omitting `mode` selects `glob`; literal path search compares the complete displayed path, while literal content search matches text within a line.
Omitted/empty `entry_ids` scans the whole archive, not an empty scope.
MCP grep accepts client budgets and continuation as defined below.
`list_entries.data.entry_count` is the whole archive's count, not the count under `path_prefix`.
ANR and ANR/ are equivalent segment-aware subtree filters under ADR 0012.

## Success and domain-error shapes

[ToolEnvelope](../../crates/mcp/src/tools/envelope.rs) serializes status, data, truncated, stopped_reason, omitted_count and notes.
The error shape uses only its specified code, message and data fields.
The four bounding fields are mandatory on this success shape, including empty and partial successes, under [ADR 0006](../decisions/0006-tool-response-scope.md).
Domain, protocol and framework errors do not inherit successful-result completeness fields.
Discovery/archive metadata uses structured facts rather than artificial content locators; source-content reads and grep matches retain locator/excerpt discipline.
Format/main-entry markers follow [ADR 0008](../decisions/0008-validated-marker-metadata.md): complete input must fit 256 bytes, be UTF-8 and contain no NUL; only one terminal LF/CRLF is removed.
Legacy format_version/main_entry fields hold the validated value or null, and archive.markers.version/main_entry provide the metadata below.
Bad markers do not become guessed string prefixes or fail otherwise available archive_info metadata; readable source entries can be investigated with read_entry using the marker entry_id.
Oversized markers set envelope truncated true with byte_budget and omitted_count zero; invalid/unreadable markers instead retain explicit validation status and explanatory notes, with no claim of clipped text.
Only missing version.txt produces ArchiveFormat0; invalid version markers remain unknown in insight findings and add an evidence gap.

| Marker metadata field | Meaning |
| --- | --- |
| status | valid, missing, oversized, invalid_utf8, contains_nul or unreadable |
| entry_id | Marker source EntryId, or null when absent |
| declared_size_bytes | ZIP metadata claim, or null when absent; not actual validation evidence |
| source_bytes | Actual complete input length after a successful bounded read, including any LF/CRLF; null when no complete read succeeded |
| line_ending_removed | Whether one terminal LF/CRLF was removed from a valid value |
| error_code | Machine reason for unavailable data, or null for valid/missing markers |

A valid marker may be empty or include other whitespace/newlines; no numeric-version or filename grammar is imposed beyond the accepted byte/UTF-8/NUL policy.
Missing version and empty valid version are distinct; an unmatched valid main-entry name keeps its value with main_entry_size_bytes null.
Complete generated artifacts remain under REQ-MCP-07 and the resource-provider contract below.
StoppedReason serializes snake_case.
[Handlers](../../crates/mcp/src/tools/handlers.rs) construct bounded listings and per-line/byte/grep excerpts under [ADR 0007](../decisions/0007-emitted-utf8-excerpt-cap.md).
Each excerpt includes text, emitted UTF-8 bytes, represented source_bytes, truncated, complete and decoding.
For follow-up, add source_bytes to the source byte offset, or to the requested byte_start for a byte-range excerpt; repeated requests do not advance any cursor.
Arbitrary byte endpoints may themselves split source UTF-8; replacement is then explicit, while excerpt clipping never cuts an emitted character.
Line and grep excerpt clipping propagates to the successful envelope as byte_budget unless a scan/item/retention stop reason already applies; omitted_count still counts omitted items/matches rather than bytes.
Byte-range bytes_returned continues to count source bytes read before excerpt clipping.
A complete scan can have shortened excerpts; scan coverage and excerpt completeness are separate dimensions.

```json
{"status":"empty","data":{"entries":[],"entry_count":3},"truncated":false,"stopped_reason":null,"omitted_count":0,"notes":["no matches"]}
```

Illustrative completed list_entries miss in a three-entry archive under ADR 0010; the entries data shape is unchanged.

The adapter places serialized JSON in MCP text content.
Protocol-class ToolError becomes JSON-RPC invalid_params with a machine code.
Domain-class ToolError becomes isError true with a separate body, not a ToolEnvelope:

```json
{"error":{"code":"entry_not_found","message":"no entry with id 42","data":{"entry_id":42,"status":"not_found"}}}
```

[ToolError::from_archive](../../crates/mcp/src/error.rs) classifies per-entry failures as domain; invalid glob, conflicting range, and unavailable mode as protocol; other archive errors fall back to domain once serving.
Open failure before transport remains a stderr/startup failure.
rmcp Parameters deserialization can return tool errors before the local protocol-error path.
Malformed calls can use different wire channels.

| Failure stage | Current responsibility | Caller-visible distinction |
| --- | --- | --- |
| CLI arguments or archive startup | CLI and `serve`, before transport | Process exit and stderr; no successful tool payload |
| Tool routing or parameter deserialization | rmcp wrapper, then handler decoding where reached | Rejected call; the wrapper may use a different error channel from local ToolError |
| Range-pair conflict, invalid glob or unavailable mode | Handler/local ToolError classification | JSON-RPC invalid parameters for the local protocol path |
| Absent or unreadable addressed entry | Core error mapped by ToolError and adapter | MCP `isError: true` and structured error body; session can continue |
| Result/work cap | Handler envelope | Successful partial result with stop metadata, not an exception or proof of a complete search |

Recover using `code` and structured fields; do not parse prose `message` as an identifier.
[Integration measures](../swe.5-component-and-integration-verification/02-integration-and-verification.md) separate the tested wire paths from these inspected design descriptions.

## Lifecycle and exit behavior

As accepted in [ADR 0014](../decisions/0014-transport-failures-and-process-exits.md), `serve` distinguishes archive-open, transport-start, transport-IO, and server-task failures so the CLI reports the actual failure stage.

Archive-open failures retain an archive diagnostic; stdio and service failures produce transport/server diagnostics and exit 1.
The private server supervisor accepts the SDK's real waiting future and classifies both its outer task-join error and a nested QuitReason::JoinError as ServeError::ServerTask.
Closed and Cancelled are successful service completions, subject to the stdout-disconnect check; unrecognized future SDK reasons fail as server-task errors rather than being labelled stdin EOF.
Deterministic test transports panic inside the SDK service receive task and send task; they use the same supervisor as production without a production fault-injection flag.
Ordinary stdin EOF exits 0, including before initialization.
A disconnected stdout response pipe exits 1 even if rmcp completes its service task without propagating the failed write.
On Unix, a background monitor polls stdout for pipe-disconnect state while the service runs and is joined during normal shutdown.
The monitor supplements service-task error handling; detected disconnection is classified as transport failure.
The non-Unix monitor does not detect response-pipe closure; [Open transport obligations](../verification/03-open-qualification.md#transport-exits) retains the portable implementation and native verification obligation without relaxing the exit contract.
Individual invalid requests receive their protocol error without terminating the session, allowing subsequent valid requests.
All diagnostics remain on stderr; stdout remains reserved for MCP frames.

## Status and completeness

ArchiveStatus lives in core.
Missing addressed entries are NotFound; unreadable entries/ranges are Unreadable.
Completed zero-match collection queries construct Empty; caps withholding matches and incomplete classifications remain partial Ok.
A stopped zero-hit grep uses Ok plus truncation.
find_by_type preserves bounded detection failures with IDs and structured errors, alongside successful matches and explicit sample coverage.
The separate error shape is accepted by ADR 0006 and archive-tool excerpt accounting by ADR 0007; marker validation follows ADR 0008; search budgets and pagination follow ADR 0009; collection outcomes and detection coverage follow ADR 0010.

Grep separates entry coverage from withheld-match counts.
A match locator contains entry_id, line/line_end, byte_offset and byte_len_match.
byte_offset locates the line; it is not a compressed seek position or an exact within-line match offset.
[Search coverage](../swe.2-architecture/03-runtime-and-interactions.md#search-coverage) defines the limitations for budget stops and skipped entries.

## Evidence and rationale

[Component checks](../swe.5-component-and-integration-verification/01-component-verification.md) exercise handlers without transport; [integration checks](../swe.5-component-and-integration-verification/02-integration-and-verification.md) exercise the shipped process.
Neither proves all four statuses across seven tools.
[Tool-surface ADR](../decisions/0002-seven-tools-and-entry-identity.md) records why capabilities stay separate.

## Insight resource provider

`REQ-MCP-07` is allocated to `ARCH-MCP-RES` and the existing server adapter.
[`resources::InsightResources`](../../crates/mcp/src/resources.rs) is protocol-free, owns the bound archive and fixed sibling paths, and caches only its mandatory archive SHA-256.
A prior digest-declining archive-info call cannot erase that mandatory identity; the provider obtains it through a fresh core reader when needed.
`list()` returns `InsightResource` metadata or empty absence, requiring all seven fixed files when the final directory exists.
`read(uri)` returns one descriptor plus exact text; it accepts no filesystem path, arbitrary filename, encoded traversal, query or foreign archive URI.

The archive parent directory is opened once when the provider is constructed. The fixed artifact root and `insight` directory are opened relative to that pinned handle with no-follow directory opens; every artifact is opened relative to the pinned insight-directory handle with no-follow semantics. Type and size checks use the opened file handle, so replacing a checked pathname cannot redirect a read. This descriptor-relative protection is available on Unix; resource access fails closed on other platforms.
Both declared length and an actual `Read::take(64 MiB + 1)` guard reject oversized data; UTF-8/JSON/identity failures never return a prefix.
The system JSON anchors listing identity; selected JSON must identify its own artifact name and archive hash, while README must contain the matching SHA-256 header.
Missing output is discovered without creating files; malformed/incomplete output is an error rather than a clean empty result.
External file mutation is not snapshot-isolated; the selected file is rechecked and its actual returned byte size recorded.

The adapter advertises resources and checks REQ-MCP-12 confirmation before delegating resources/list/resources/read via spawn_blocking, without adding archive tools.
Unknown resource/absent-file errors map to `-32002`; other provider failures map to `-32603`; nonempty list cursors map to `-32602` because the seven-item list is unpaginated.
Read uses `ResourceContents::text` with `text/markdown` or `application/json`; it returns artifact text, not the archive-tool envelope.
Resources do not inherit the archive tools' excerpt cap; their separate complete-file ceiling is 64 MiB.
No templates, subscription or list-change capability is advertised.
SDK symbols were verified against cached rmcp 3.5.0 source; responses use private caching and zero TTL where the negotiated protocol supports them.

[Provider checks](../../crates/mcp/tests/resources.rs) exercise exact bytes, identity, late publication, malformed/missing/symlink/oversized sources and URI rejection. A descriptor-level race regression replaces the checked artifact directory and leaf between inspection and open, proving the pinned directory remains authoritative and symlink leaves are refused.
[Wire checks](../swe.5-component-and-integration-verification/02-integration-and-verification.md#mcp-insight-resources) exercise CLI generation → MCP resource reads on both supported test protocol versions.

## Search pages

[ADR 0009](../decisions/0009-client-search-budgets-and-pagination.md) defines the approved limits and lifecycle; [pagination.rs](../../crates/mcp/src/tools/pagination.rs) implements the stateful handler.

| Input | Default | Accepted values |
| --- | --- | --- |
| response_budget | 65,536 bytes | 2,048–262,144 successful serialized MCP tool-result JSON bytes, including text escaping/cursor; excludes outer JSON-RPC frame/newline |
| scan_budget | 33,554,432 bytes | 1–33,554,432 newly consumed decompressed source bytes per page |
| max_matches | 50 | 1–50 matching lines per page |
| cursor | absent | Opaque process-local single-use token; expires after 300 idle seconds |

A new search requires pattern; mode defaults to glob and omitted/empty entry_ids means the entire archive.
Duplicate scope IDs are deduplicated in first-occurrence order; every requested ID is validated before scanning.
A continuation supplies cursor and optional budgets/max_matches, omitting pattern/mode/entry_ids; attempts to change saved query fields, invalid budgets and unknown/expired/used tokens are invalid_params.
Unsupported modes retain pattern_mode_unavailable.
Budget validation occurs before consuming a cursor, allowing corrected retries.
An individual locator too large for response_budget is an error; an existing cursor retains that pending locator for retry with a larger budget.
No total page count is promised.

Success data always contains matches, next_cursor (string or null) and scan.
scan contains cumulative scanned_entries (started files), completed_entries (EOF files plus directories), entries_in_scope, entries_not_searched (not yet visited), partial_entry_id (active decoder or null), skipped_entries, skipped_lines and total_scanned_bytes.
scanned_bytes alone is per-page new source consumption, including LF bytes and discarded long-line data, without invented LF at unterminated EOF.
coverage_complete is true only after scope and pending output are exhausted with no skipped entries/lines.
When the cap is reached before EOF observation, a later empty page may be needed to establish completion.
Carry bytes already charged to an earlier page may yield matches on a page consuming zero new source bytes.
No line matcher runs until LF or actual EOF; partial lines/UTF-8 survive page boundaries and overlong lines are skipped as explicitly counted matching gaps.
Encrypted entries are counted as skipped; directories have no content and count as completed.

A page boundary uses truncated true with scan_budget, match_cap or byte_budget and next_cursor; unknown future matches and deferred observed matches do not increase omitted_count.
After exhaustion, skipped lines use line_too_long and encrypted-only gaps use skipped_content, with no cursor and coverage false.
Complete zero-hit pages use Empty only when coverage is complete; partial/gapped zero-hit results use Ok with explanation.
Output excerpt clipping alone uses byte_budget and can leave coverage true and cursor null.
Actual read/CRC/safety failures remain domain errors and invalidate the consumed continuation.

The handler holds at most eight saved sessions behind a mutex; a ninth new search fails until a session finishes or expires.
Expired sessions are reclaimed on the next validly budgeted grep call, and all sessions release their workers when the handler is dropped.
The decoder worker retains only fixed scratch/carry plus a line bounded at 1 MiB and one pending locator; it never buffers an entire archive entry or replays an already searched prefix.
The source archive must remain externally unchanged during continuation, as for existing reader operations.

## Collection outcomes and detection coverage

[ADR 0010](../decisions/0010-empty-results-and-detection-failures.md) owns the consistent empty/partial/error policy.
list_entries and find_entries use Empty only when the query completes with no matching entries; a zero output limit withholding existing matches uses Ok with item_cap.
find_by_type always returns matches, failures, omitted_matches, omitted_failures and coverage, keeping the same data shape for empty and partial results.

| Field | Meaning |
| --- | --- |
| matches | At most limit records, default/max 200, each with entry summary and DetectionReport |
| failures | At most 200 records independently, each with entry_id, path and error.code/message/data |
| omitted_matches | Known matching records withheld by limit |
| omitted_failures | Known failure records withheld by the failure-detail cap |
| coverage.entries_in_scope | File entries attempted, excluding directories |
| coverage.classified_entries | Files whose promised sample was successfully classified, including Unknown |
| coverage.failed_entries | Files whose sample could not be classified |
| coverage.skipped_directories | Directory entries excluded from file classification |
| coverage.coverage_complete | All file samples successfully classified; independent of returned-record clipping |

The handler attempts every file, retaining only bounded match/failure detail while counting all outcomes.
A clean zero-match query uses Empty; a nonempty or capped query uses Ok; any classification failure forces partial Ok and coverage false, including all-failed queries.
Output omission uses item_cap and omitted_count = omitted_matches + omitted_failures; otherwise classification failure uses skipped_content and omitted_count zero.
Unknown future matches in failed entries are not added to omission counts.
Classification failures remain explicit when querying Unknown; a failed read is not a successfully unknown type.
Detection coverage concerns at most the existing 8 KiB sample and stated basis, never complete-file/CRC validation under lazy reads.
Direct entry_info/read_entry requests retain their domain-error channels; one partial collection does not end the server session.

## Content IO confirmation

REQ-MCP-12 is implemented by protocol-free [ConfirmationGate](../../crates/mcp/src/confirmation.rs), [registry wrappers](../../crates/mcp/src/registry.rs) for five tools, and [adapter resource gates](../../crates/mcp/src/adapter.rs).
A first request returns a challenge without entering the content handler/provider; the client may explicitly retry that operation with the token after reviewing its cost.
There is no runtime automatic acceptance; behavior-test helpers explicitly accept previews to exercise previously verified operations, while raw wire checks assess the challenge boundary itself.
Startup still admits ZIP metadata before serving; discovery and the two index queries need no approval.

| Channel | First response and retry |
| --- | --- |
| Content tool | MCP isError true; text contains error.code confirmation_required, error.message and error.data with operation, confirmation_token, cost, expires_in_seconds; retry identical arguments plus confirmation |
| Resource method | JSON-RPC error -32000; error.data contains code confirmation_required and those challenge fields; retry identical params plus _meta["bugreport-insight/confirmation"] |
| Invalid/replayed/expired token | Invalid-params channel, machine code invalid_confirmation; no content execution |
| Full pending pool | Domain/server error confirmation_capacity; no new pending request or work |
| Oversized bound arguments | Invalid-params channel, confirmation_arguments_too_large; no retained request or work |

Token state transitions are issued → confirmed/removed, issued → expired/reclaimed, or issued → unchanged after a mismatch.
Checks serialize access to each pool with a mutex; successful matching removes the token before execution, so racing retries cannot run twice with one approval.
Tools share a 64-token pool and resources a separate 64-token pool; each pending request retains at most 65,536 serialized normalized argument bytes plus bounded representation overhead.
Tokens use process-local random-state keyed opaque identifiers and expire after 300 seconds; cleanup occurs on subsequent valid gate checks or shutdown, with no timer task.
Tool binding compares the JSON object after typed adapter default insertion and removal of confirmation; object-key order is irrelevant, while arrays, strings, explicit values and optional-null distinctions at the registry seam remain significant.
Resource binding uses resources/list with an empty argument object or resources/read with the exact URI; transport request IDs and unrelated metadata are not execution arguments.
Semantic validation/domain failures can follow approval and still consume its token; a confirmation_required preview is not a promise that parameters or content will succeed.

Every grep page needs its own preview and confirmation, including zero-hit continuations.
Previewing never consumes or refreshes the search cursor; its independent 300-second idle lifetime can expire before a newer approval token does.
Confirmation and cursor tokens are distinct; changed budgets/URI/entry/range/query require a fresh matching preview, while a mismatch preserves the original approval for its original arguments.
Clients keep the original request and never infer approval from an earlier operation/page.
This is a deliberate client retry protocol, not authentication of human identity; clients are responsible for deciding whether to send the returned token.

### Cost preview scope

Tool cost fields stored_bytes_upper_estimate and source_bytes_upper_estimate describe potential hash input and decoded source consumption respectively, not all physical filesystem reads.
entry_declared_size_bytes is advisory ZIP metadata when an entry is addressed; metadata_io signals parser/metadata work during execution.
Previews themselves use only the in-memory archive index and arguments.
Hash estimates allow the full admitted stored ZIP even when cached hashing makes actual work smaller; archive_info permits 514 decoded marker bytes total including one overflow probe per marker.
entry_info allows 8 KiB; find_by_type allows 8 KiB per file even when requested output is small; read_entry allows the 256 MiB entry ceiling plus overflow probe because late offsets may require prefix decompression.
grep previews allow the page scan_budget, capped at 32 MiB; internal decoder work, physical IO, CPU and elapsed time are not guaranteed by decoded-byte limits.
A failed early request may perform less work than estimated.
The existing digest cache retains the first successful choice including declining hashing, and concurrent uncached callers may both compute before a value wins; no strict once-only IO is promised.
Resource previews permit full stored-file hashing, marker reads and a complete system.json manifest up to 64 MiB; reads additionally permit the requested artifact up to 64 MiB, so reading system.json itself may read that file twice.
Missing resources can return empty after confirmation without hashing or artifact reads; a prior digest-declining identity call can require a fresh archive reader to recover the mandatory resource hash.
[ADR 0013](../decisions/0013-confirmation-before-content-io.md) and [executed evidence](../../verification/results/content-io-confirmation.md) record acceptance and verification; physical IO/CPU timing and full qualification remain outside these estimates.

## Confined insight tools

REQ-MCP-13 is implemented by ARCH-MCP-INSIGHT alongside ARCH-MCP-SRV.
`list_bugreports` accepts no path argument and returns candidate basename IDs plus explicit rejected-candidate diagnostics.
Directory discovery inspects at most 1,000 immediate entries and returns at most 200 accepted/rejected records in total with completeness metadata.
Only regular ZIP children are candidates; child symlinks, nested paths, parent traversal and caller-selected paths are refused or excluded.
Detection requires a non-directory, non-symlink root `main_entry.txt` or root `bugreport*.txt` entry; `version.txt` alone is insufficient.
This uses ZIP central-directory metadata and root report names, not the filename extension alone; it neither decompresses markers nor proves report payload integrity.
A Unix directory session retains an opened directory descriptor, opens an independent descriptor-relative `.` directory cursor and enumerates through `fdopendir`/`readdir`, and opens each candidate with descriptor-relative `openat` and `O_NOFOLLOW | O_NONBLOCK`.
The opened file must be regular; its handle is passed to `Archive::from_file`, keeping later independent readers and hashes on the admitted file rather than reopening a replaceable pathname.
Unsupported platforms reject directory-mode startup explicitly; single-file combined mode remains available.
Before a selected candidate is used, it must still satisfy scope, regular-file and archive-admission checks.

| Tool | Arguments | Result and work |
| --- | --- | --- |
| `list_bugreports` | empty object | Bounded candidate IDs and rejection diagnostics; ZIP metadata only. |
| `list_insights` | `bugreport`, optional `confirmation` | Seven artifact names/metadata from confirmed in-memory analysis; no output directory required. |
| `read_insight` | `bugreport`, `artifact`, optional `byte_start`, `max_bytes`, `source_sha256`, `snapshot_id`, `confirmation` | A UTF-8 artifact page with byte offsets and explicit continuation/completeness. |

The artifact allowlist is README.md, system.json, apps.json, anr.json, crashes.json, memory.json and cpu.json.
`max_bytes` defaults to 16 KiB and is at most 64 KiB; offsets use generated UTF-8 bytes and pages preserve character boundaries.
The cached complete artifact set is at most 64 MiB in aggregate; exceeding that bound fails without presenting a prefix as complete.
Both artifact methods require request-specific confirmation before analysis or text access, including cache hits; `--force` is not confirmation.
An offset splitting a UTF-8 character, an out-of-range offset or a page too small for the next character returns `invalid_insight_range`; invalid page budgets are refused.
At most one report's generated artifact set is cached, limiting retained analysis output while allowing README-first exploration.
File combined mode retains archive tools/resources and adds these three tools; directory mode exposes these three tools only.
All source/evidence semantics come from the existing insight generator; the provider does not infer new findings from filenames.

Every successful `read_insight` page includes `data.source_sha256` and `data.snapshot_id`.
Requests with `byte_start > 0` must supply the preceding page's `source_sha256` and `snapshot_id`; the provider compares the digest with the pinned source and the snapshot ID with the retained generation before delivery.
Missing continuation identity is refused; a mismatched digest returns `insight_source_changed` instead of returning a page from another archive revision.

Each cache generation receives a new process-local snapshot ID; after cache eviction or regeneration, `insight_snapshot_expired` requires restarting at byte zero.
The provider never regenerates an evicted artifact set to satisfy a continuation, because host-run metadata may differ even when source bytes are unchanged.
