# Software qualification assessment — 2026-10-19

The current committed set contains **30 requirements**, assessed individually below; the earlier 23- and 29-requirement counts are historical baselines.
The assessment finds **29 passed for the reviewed macOS implementation and selected acceptance evidence, one not demonstrated, and no observed failing requirement**.
REQ-CLI-03 lacks native non-Unix broken-response-pipe execution evidence; actual SDK service/send-task fault propagation is now demonstrated.
This is a completed requirement walkthrough and report, not an unconditional whole-product or cross-platform conformance sign-off.
[TASK021](../../docs/verification/03-open-qualification.md#transport-exits) owns that remaining verification work.
Deferred requirements remain outside the verdict set; the promoted insight schema contracts are in scope through REQ-INSIGHT-03/04.
No ASPICE assessment or certification is claimed.

## Tree, environment and methods

Base revision is `b31987d7d3e3e77452ac322d4a35b6f93ff36d64`, plus the existing uncommitted insight/resource implementation, TASK018/TASK019 changes and this session's documentation reconciliation and server lifecycle correction.
The assessment concerns that working tree, not the base commit alone.
Platform is Darwin arm64, with `rustc 1.99.0 (b940084d7 2026-09-28)` and `cargo 1.99.0 (5f94df478 2026-08-27)`, Rust 2024 and offline dependencies.
Fixtures are the shared synthetic ZIP builders, hostile metadata/payload fixtures and the hash-identified private reference archive described in the linked results.
Private report bytes and generated output remain untracked; this report contains no real bugreport excerpts.

The review joined the normative clauses and shared tool constraints to their implementing units, test assertions, and executed records rather than assigning verdicts from suite totals.
The review also identified the SDK nested send-task termination bug, now corrected and verified by [actual task-failure evidence](server-task-failures.md).
Source review covered archive open/read guards, identity/markers, tool handlers/parameters/envelopes, search continuation, confirmation wrappers, resource access, server lifecycle, CLI dispatch and insight construction/validation/publication.
Historical evidence is reused only where the contract and implementing path remain applicable: the October 17 insight/resource records are supplemented by the current marker, lazy-read and confirmation changes and current regression execution.
A reused result remains identified by its original date; this does not recast an old real-reference trial as a new manual session.

| Method | Expected result | Observed result |
| --- | --- | --- |
| Extract explicit requirement anchors from requirements, traceability and SWE.6 measures | Same unique committed ID set in all three | 30 anchors, 30 unique, no missing or extra anchors in each file. |
| `cargo metadata --offline --no-deps --format-version 1`, target/dependency inspection | Distinct core/MCP libraries and CLI executable; approved production edges | Core library has no workspace production dependency; MCP depends on core; CLI binary `bugreport-insight` depends on both; fixture edges are development-only. |
| Clause/assertion inspection below | Each verdict grounded in actual behavior, assertion and result | Completed for all 30 IDs; the transport failure execution gap is retained. |
| Fresh synthetic CLI probe with NUL in first report line | Explicit incomplete excerpt and NUL note, no stdout analysis | Exit 0, zero stdout bytes, `archive_identity` excerpt contains NUL, `excerpt_complete:false`, `NUL_BYTES_IN_EVIDENCE` note present. |
| Workspace gates | Formatting, offline build/test and warnings-denied Clippy pass | [Final lifecycle execution](server-task-failures.md), executed October 19: 200 tests passed, zero failed/ignored; formatting, offline build and Clippy passed. |

The ID comparison is a set/uniqueness check, not proof of assertion quality or every allocation edge.
[Traceability](../../docs/traceability.md) remains the canonical allocation; this report supplies verdicts rather than a competing requirements catalog.

## Requirement walkthrough

**Passed** means the applicable clauses were supported by the named executable assertions and reviewed implementation in the stated scope.
**Not demonstrated** retains missing execution evidence; it is not a demonstrated failure.
Detailed fixture/assertion specifications resolve through the [SWE.6 measure map](../../docs/swe.6-software-verification/01-requirement-verification.md) and its linked SWE.4/SWE.5 pages.

| Requirement | Clauses and demonstrating checks/review | Verdict |
| --- | --- | --- |
| REQ-GEN-01 | No extraction/write in core open/read paths; `a_full_tool_sweep_leaves_the_archive_unchanged` compares source bytes, and insight process checks compare bytes/mtime while publishing only the specified sibling artifacts. | Passed. |
| REQ-GEN-02 | Metadata count/stored size/declared entry and aggregate admission, coherent library limits, produced bytes including skipped prefixes, exact ceilings, lying sizes, stored/deflated continuity and unread-tail scope: `hostile`, `lazy_validation` and `partial_ranges` assertions plus all reader guards. [Lazy-policy result](lazy-archive-validation.md). | Passed; no certification of unread payload or cumulative session expansion. |
| REQ-LIB-01 | Offline Cargo targets/dependencies and public-core consumers demonstrate independently reusable core library with external-only production dependencies. | Passed. |
| REQ-MCP-01 | Distinct library/executable targets, CLI mode ownership, CLI→core/MCP and MCP→core edges; manifest/source inspection excludes reverse production coupling. [Dependency result](cli-mcp-dependencies.md). | Passed. |
| REQ-MCP-02 | `serve` constructs one `Arc<Archive>` and registry/resource provider from it; no switching parameter or archive-open tool; bound identity and all-seven process tests. | Passed. |
| REQ-MCP-03 | Server tools call the same guarded read-only core APIs; wire immutability/error checks plus hostile/lazy tests show no adapter bypass or extraction. | Passed. |
| REQ-MCP-04 | All seven handlers and provider have no publication/mutation paths; direct/wire sweeps preserve archive; search/confirmation mutations remain bounded process-local state explicitly permitted by the contract. | Passed. |
| REQ-MCP-05 | Exact seven unique wire names, descriptions, schemars object schemas, closed inputs and documented result shapes; unknown/misspelled parameters rejected. Discovery order is unspecified. [Discovery result](unordered-tool-discovery.md). | Passed. |
| REQ-MCP-06 | All successful tools/empty/partial results have four bounds fields; domain/protocol/challenge errors remain distinct; item/match/scan/response bounds, UTF-8/source byte accounting, continuation and coverage, bounded validated markers and resource exception reviewed against handler/wire assertions. [Response scope](tool-response-scope.md), [excerpt cap](emitted-excerpt-cap.md), [markers](marker-validation.md), [pagination](search-pagination.md). | Passed. |
| REQ-MCP-07 | Fixed seven names, complete exact UTF-8 text/MIME/size, SHA-bound URIs and identity verification, explicit missing/incomplete/symlink/invalid/oversize errors; bounded actual reads and no generation/write path. Eight provider tests and real CLI resource comparisons. [Resource result](mcp-insight-resources.md), with current confirmation checks. | Passed. |
| REQ-MCP-08 | Clean misses are Empty with normal shape; capped/unfinished/all-failed queries partial Ok; separate bounded match/failure omissions and detection-sample coverage; missing/unreadable direct entry and invalid params stay errors. [Outcome result](empty-and-detection-outcomes.md). | Passed; no artificial universal four-status-by-seven-tools matrix is required. |
| REQ-MCP-09 | Read/grep locator entry ID, kind, source range and byte offset; 512-byte UTF-8 preservation, completeness, source_bytes continuation and lossy replacement; source offsets checked against fixtures. Metadata/error/resource exceptions reviewed. [UTF-8](utf8-line-excerpts.md) and [excerpt follow-up](emitted-excerpt-cap.md) results. | Passed. |
| REQ-MCP-10 | Only stdio selected; stdout frame capture, initialize and verbose tests plus inspection of every diagnostic path show stderr logging and no alternate transport registration. Capture ignores blank lines, supplemented by source inspection finding no stdout logging path. | Passed for the reviewed implementation; cross-platform failure-exit evidence belongs to CLI-03 below. |
| REQ-MCP-11 | Initialization instructions name all tools, truthful work/output bounds, sequence and per-call/page/resource confirmation retries within 40 lines and without archive content. Cost descriptions checked against actual hashing/markers/samples/range/scan paths. | Passed. |
| REQ-MCP-12 | Gate and wrapper assertions cover pre-approval refusal, all five content tools/both resources, operation/normalized-argument binding, token single use/mismatch/expiry/foreign/replay, pool capacity, argument limit, independent resource pool and per-page approval. Semantic failures remain possible after consumption. | Passed; estimates do not guarantee time, CPU or physical IO. |
| REQ-MCP-ARC-01 | Bound basename/size/count/digest match independent reference identity; explicit digest opt-out/cache choice inspected; validated marker values/status/source IDs and named-entry size asserted, including absent/invalid/oversize/unreadable. [Marker result](marker-validation.md) and reference identity tests. | Passed against the clarified optional first-choice digest contract. |
| REQ-MCP-ARC-02 | ID/path/size payload reviewed; item omission/default caps, empty/all subtree selection, exact path/slash boundaries and ANR versus ANR2 checked through handlers and wire. [Range/prefix result](range-boundaries.md). | Passed. |
| REQ-MCP-ARC-03 | Index-only literal and segment-wise glob paths, no content reads; invalid glob and typed regex refusal; shared ZIP-only/entry-ID scope inspected. | Passed. |
| REQ-MCP-ARC-04 | Exactly one complete range pair, inclusive 1-based lines, half-open bytes, equal-byte empty request, invalid/reversed bounds and available/wholly past-end EOF; preserved CRC/decoder/safety errors and excerpt/source-range accounting. [Range result](range-boundaries.md), `partial_ranges` and wire follow-up. | Passed. |
| REQ-MCP-ARC-05 | Scoped/archive-wide content search returns located matches; all budgets and complete-line matching, UTF-8/deflated page continuity, pending delivery, independent page approval, cursor binding/lifecycle and skipped-content reporting reviewed and executed. [Pagination result](search-pagination.md). | Passed; skipped overlong/unreadable content remains explicit incomplete coverage. |
| REQ-MCP-ARC-06 | Handler returns compressed/uncompressed sizes, compression code/name, optional raw DOS modification fields, detection/basis and nested ZIP status; direct detection and nested-hostile assertions; unreadable sample remains error. | Passed; declared size and packed modification fields are metadata, not measured expansion/timezone conversion. |
| REQ-MCP-ARC-07 | Sample-backed text/binary/specific types and Unknown basis; supported requested names, no-match, invalid type, partial/all failed and bounded omission cases; directories excluded and sample-only completeness explicit. [Outcome result](empty-and-detection-outcomes.md). | Passed. |
| REQ-CLI-03 | Valid bound startup, pre-transport archive errors, bad usage, normal EOF before/after initialization, broken stdout on Unix, analysis/publication failures and request recovery pass; actual rmcp service/send-task panics propagate through the production supervisor into typed server-task errors and existing CLI exit mapping. The nested send-task defect failed before correction and passes afterwards. The non-Unix monitor returns false without equivalent detection evidence. [Transport result](transport-exits.md) and [task-failure result](server-task-failures.md). | Not demonstrated for native non-Unix broken stdout; all listed executed cases passed after correction. |
| REQ-CLI-04 | `help_goes_to_stdout_and_exits_zero` asserts successful help, binary name and supported flags; actual usage inspected. | Passed. |
| REQ-CLI-05 | `verbose_adds_stderr_detail_only`, captured stdout frames and CLI diagnostic dispatch demonstrate verbose stderr detail with protocol-only stdout. | Passed. |
| REQ-INSIGHT-01 | Exact single-report syntax, exclusive modes/positionals, invalid/missing input, six-domain run and sibling destination; process tests assert no stdout payload and stderr diagnostics. [Insight result](insight-generation.md). | Passed. |
| REQ-INSIGHT-02 | Source bytes/mtime preserved; fixed sibling writes, existing output hashes and racing empty/nonempty destination preservation; injected write failure removes staging and publishes no partial final set. Source review confirms exclusive final rename. | Passed on macOS; power-loss durability is not promised. |
| REQ-INSIGHT-03 | Exact seven files, JSON-object roots, required common/domain structures, capture reference/source/generator consistency, unique finding IDs and validator invariants; all-domain synthetic values and measured reference outputs. Reviewed promoted schema/generation rules against constructor/validator and reused October 17 clause review. | Passed for specified supported evidence layouts; no independent machine-JSON-Schema certification is claimed or required. |
| REQ-INSIGHT-04 | Shared provenance/freshness/severity caps, raw-versus-validated counts, domain joins/deduplication and identity, null/gap/negative evidence, event-only reported confidence, stale/current separation, memory units, CPU windows and sampled contention are inspected and asserted by domain fixtures. Reference locators were independently checked in October 17 evidence; fresh NUL probe confirms explicit incompleteness/note. | Passed for supported layouts; unavailable/OEM-unrecognized evidence remains explicit, not a health verdict or inferred cause. |
| REQ-INSIGHT-05 | Recursive validator enforces 512-byte excerpts/counts; UTF-8/NUL tests, README ten-row/400-line guards, section/retention budgets and inherited archive-expansion guards; oversized/failed parse aborts before publication. [Insight result](insight-generation.md) plus current lazy-validation checks. | Passed. |

## Documentation validation

`python3 scripts/check_docs.py` passed after the assessment and related evidence updates: 190 Markdown pages, 2,227 local links and zero errors.
Affected tracked Markdown whitespace checks passed.

## Disposition and remaining work

Archive immutability is demonstrated through source review plus synthetic bytes/mtime checks and the identified historical reference trials.
Bounded behavior is demonstrated against metadata/produced-byte guards, emitted excerpts, serialized search-page budgets, continuation state, confirmation pools and complete resource ceilings; these are distinct limits rather than a universal peak-memory/time bound.
Protocol correctness is demonstrated through the real executable's initialization, discovery, tools, resources, confirmation challenges, error/recovery paths and clean disconnect.
The separate [TASK020 real-case trial](real-case-user-validation.md) is now complete against the final rebuilt binary: all three user roles passed, with independent search-locator checks and input/output SHA/mtime preservation.
That workflow evidence supplements this walkthrough; it does not substitute for hostile-input checks or native non-Unix qualification.

TASK021 must implement/execute equivalent broken-response-pipe behavior on a supported native non-Unix runner or explicitly decide supported platform scope through SWE.1–SWE.6.
Actual SDK receive/send-task panics now exercise the production supervisor, composed with the existing CLI typed-error exit assertion; no production fault flag or released-CLI crash injection was introduced.
The composed task-propagation/exit evidence closes the SDK task-failure clause, while neither those checks nor Unix process checks establish native non-Unix response-pipe behavior.
The remaining gap is verification scope and possible platform behavior, not a reason to retain resolved contract discrepancies in requirements.
Any behavior correction follows the complete SWE change process and updates this assessment with actual results.

The current source-review/fixture assertions support the promoted schema clauses for supported layouts; no generic successor task demanding proof of every possible Android/OEM report is created.
Additional uncovered layouts should become concrete requirements/design/test increments when encountered in the authorized real-case exercise.
