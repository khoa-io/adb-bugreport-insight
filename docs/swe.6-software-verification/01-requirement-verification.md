# Requirement verification measures

Candidate measures for 30 [committed requirements](../swe.1-requirements/02-software-requirements.md).
This measure catalog supplies check specifications; the [qualification assessment](../verification/assessment.md) supplies requirement verdicts.
The [insight execution](../../verification/results/insight-generation.md) records fresh targeted verdicts for the five new requirements and workspace regression results.
The current 30-requirement walkthrough is complete with a qualified result; native non-Unix exit evidence remains under [Open transport obligations](../verification/03-open-qualification.md#transport-exits).
Unrelated proposed requirements are excluded; the superseded artifact-schema source is in scope through `REQ-INSIGHT-*`.

[Traceability](../traceability.md) owns requirement → architecture allocation and links to design/source.
[Unit measures](../swe.4-unit-verification/01-verification-measures.md), [component measures](../swe.5-component-and-integration-verification/01-component-verification.md), and [integration measures](../swe.5-component-and-integration-verification/02-integration-and-verification.md) identify exact executable symbols and assertion scope.
Supporting executions are linked from the [verification index](../verification/index.md).

## Committed measure map

Rows select tests/inspection without claiming full clause coverage; abbreviated names resolve through the measure pages.

| Requirement | Selected measure | Review boundary / remaining evidence |
| --- | --- | --- |
| <a id="req-gen-01"></a>[`REQ-GEN-01`](../swe.1-requirements/02-software-requirements.md#req-gen-01) | Review core filesystem access; execute `reading_tools_do_not_mutate_the_fixture` and `a_full_tool_sweep_leaves_the_archive_unchanged` | Wire sweep compares bytes; direct handler sweep compares length only. Inspect extraction/output side effects separately. |
| <a id="req-gen-02"></a>[`REQ-GEN-02`](../swe.1-requirements/02-software-requirements.md#req-gen-02) | Core hostile and [lazy-validation tests](../../crates/core/tests/lazy_validation.rs) plus review `caps`, `Archive::open_with`, and reader guards | Metadata admission and actual produced-byte enforcement are distinct; [targeted evidence](../../verification/results/lazy-archive-validation.md) covers the accepted lazy policy, without full software sign-off. |
| <a id="req-lib-01"></a>[`REQ-LIB-01`](../swe.1-requirements/02-software-requirements.md#req-lib-01) | Inspect core package manifest and exported `Archive` API; compile/use public core tests | Review library distribution and absence of workspace runtime dependencies. |
| <a id="req-mcp-01"></a>[`REQ-MCP-01`](../swe.1-requirements/02-software-requirements.md#req-mcp-01) | Inspect resolved production dependencies, crate targets and CLI/server entry points | [Targeted verification](../../verification/results/cli-mcp-dependencies.md) confirms CLI→MCP/core and MCP→core, with forbidden reverse/transitive edges absent; dev-only fixtures excluded. |
| <a id="req-mcp-02"></a>[`REQ-MCP-02`](../swe.1-requirements/02-software-requirements.md#req-mcp-02) | Inspect server bound `Arc<Archive>` and registry construction; execute bound identity and all-seven wire tests | Verify lifetime binding and absence of an archive-switch API by inspection. |
| <a id="req-mcp-03"></a>[`REQ-MCP-03`](../swe.1-requirements/02-software-requirements.md#req-mcp-03) | Apply GEN-01/02 checks through real CLI tool sweep | Synthetic sweep does not itself establish every limit or absence of extraction. |
| <a id="req-mcp-04"></a>[`REQ-MCP-04`](../swe.1-requirements/02-software-requirements.md#req-mcp-04) | Inspect all handlers and registry; run direct/wire immutability sweeps | Review filesystem effects separately from archive-byte comparison. |
| <a id="req-mcp-05"></a>[`REQ-MCP-05`](../swe.1-requirements/02-software-requirements.md#req-mcp-05) | Run registry-name, closed-schema, unknown-parameter, and wire tool-list tests | Wire tests check exact unique membership independently of order, plus descriptions and closed input schemas; [ADR 0011 evidence](../../verification/results/unordered-tool-discovery.md) records the targeted result. |
| <a id="req-mcp-06"></a>[`REQ-MCP-06`](../swe.1-requirements/02-software-requirements.md#req-mcp-06) | Run common envelope, item-omission, grep-stop, and byte-read cap tests; inspect all result paths | [Response-scope evidence](../../verification/results/tool-response-scope.md) covers success versus error shapes and metadata applicability; [Excerpt-cap evidence](../../verification/results/emitted-excerpt-cap.md) covers emitted-byte accounting; [marker validation](../../verification/results/marker-validation.md) covers the accepted metadata exception; [Search pagination evidence](../../verification/results/search-pagination.md) covers scanner stop boundaries. |
| <a id="req-mcp-07"></a>[`REQ-MCP-07`](../swe.1-requirements/02-software-requirements.md#req-mcp-07) | Execute provider and real resource process checks; compare every resource to generated bytes; review bounded reads/URI matching | Complete-file, SHA-bound read-only access; see [fresh resource record](../../verification/results/mcp-insight-resources.md). |
| <a id="req-mcp-08"></a>[`REQ-MCP-08`](../swe.1-requirements/02-software-requirements.md#req-mcp-08) | Run representative four-status, corrupt/missing entry, and stalled-scan tests | [Outcome evidence](../../verification/results/empty-and-detection-outcomes.md) adds collection misses, capped/partial/all-failed classification, explicit bounded failures and direct domain-error wire checks; a universal 28-cell status matrix is not claimed. |
| <a id="req-mcp-09"></a>[`REQ-MCP-09`](../swe.1-requirements/02-software-requirements.md#req-mcp-09) | Run byte-offset, scoped grep, line-offset, and byte-excerpt tests; inspect locator payloads | [UTF-8 line evidence](../../verification/results/utf8-line-excerpts.md) verifies character preservation through MCP; [emitted-cap evidence](../../verification/results/emitted-excerpt-cap.md) covers line/byte/grep truncation and source follow-up; The qualification assessment reviews the remaining kind/range/coverage clauses. |
| <a id="req-mcp-10"></a>[`REQ-MCP-10`](../swe.1-requirements/02-software-requirements.md#req-mcp-10) | Run stdout capture, verbose stderr, initialize, EOF-before/after-init, broken-pipe and invalid-request recovery process tests; inspect transport selection | [Transport exit evidence](../../verification/results/transport-exits.md) checks clean EOF, broken stdout, session recovery and typed server-task exit mapping; [Actual SDK task-failure evidence](../../verification/results/server-task-failures.md) closes that propagation gap; native non-Unix pipe detection remains unverified. |
| <a id="req-mcp-11"></a>[`REQ-MCP-11`](../swe.1-requirements/02-software-requirements.md#req-mcp-11) | Run instructions_name_every_tool_and_stay_short; inspect INSTRUCTIONS and cost previews against actual IO paths | Verify all names/line count, actual work/output bounds, intended call sequence and confirmation retry fields; no universal cost ranking or archive content. |
| <a id="req-mcp-12"></a>[`REQ-MCP-12`](../swe.1-requirements/02-software-requirements.md#req-mcp-12) | Run gate/wrapper units and raw confirmation protocol cases | No content IO before approval; exact operation/argument binding, single-use/mismatch/expiry/capacity/argument-size limits, independent resource pool and per-page cursor approval. |
| <a id="req-mcp-arc-01"></a>[`REQ-MCP-ARC-01`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-01) | Run bound identity, digest-declined, and reference identity tests; inspect identity handler | Digest follows the specified optional first-choice cache contract; marker caps/validation have targeted and qualification evidence. |
| <a id="req-mcp-arc-02"></a>[`REQ-MCP-ARC-02`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-02) | Run prefix-list and omitted-item tests; wire all-seven sweep | Inspect ID/path/size payload and default item cap. |
| <a id="req-mcp-arc-03"></a>[`REQ-MCP-ARC-03`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-03) | Run path matching, regex refusal, invalid-glob, and segment-wise glob tests | Inspect literal/glob input behavior and index-only matching. |
| <a id="req-mcp-arc-04"></a>[`REQ-MCP-ARC-04`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-04) | Run both/neither range-pair, line-offset, byte-cap, late-line, and hostile-line tests | [ADR 0012 evidence](../../verification/results/range-boundaries.md) verifies partial EOF, exact boundaries, invalid endpoints and preserved safety errors; excerpt cap has separate ADR 0007 evidence. |
| <a id="req-mcp-arc-05"></a>[`REQ-MCP-ARC-05`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-05) | Run locator byte-offset, caller-scope, coverage-shape, and stalled-scan tests | Budgeted-page wire checks and core scan_pages exercise partial-entry continuation, output limits, complete-line matching and explicit gaps; see the search-pagination result. |
| <a id="req-mcp-arc-06"></a>[`REQ-MCP-ARC-06`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-06) | Run detection-basis and nested-archive tests; inspect entry-info result construction | Compare every size/method/time/type/nested metadata field with the requirement. |
| <a id="req-mcp-arc-07"></a>[`REQ-MCP-ARC-07`](../swe.1-requirements/02-software-requirements.md#req-mcp-arc-07) | Run type-filter and unknown-type tests; inspect detection basis | Inspect sampled-byte classification and unidentifiable cases. |
| <a id="req-cli-03"></a>[`REQ-CLI-03`](../swe.1-requirements/02-software-requirements.md#req-cli-03) | Run initialize, all-seven, bad-ZIP startup, missing-archive/mode, EOF-before/after-init, broken-pipe and invalid-request recovery tests | [Transport exit evidence](../../verification/results/transport-exits.md) records exact process exits and diagnostic stage plus typed server-task exit mapping; [Actual SDK service/send-task failures](../../verification/results/server-task-failures.md) now reach the production supervisor and CLI typed-error mapping; native non-Unix pipe behavior remains open. |
| <a id="req-cli-04"></a>[`REQ-CLI-04`](../swe.1-requirements/02-software-requirements.md#req-cli-04) | Run `help_goes_to_stdout_and_exits_zero` | Checks successful help and expected flags/name. |
| <a id="req-cli-05"></a>[`REQ-CLI-05`](../swe.1-requirements/02-software-requirements.md#req-cli-05) | Run `verbose_adds_stderr_detail_only` and stdout capture; inspect `diag` | Compare stderr behavior and inspect verbose flag handling. |
| <a id="req-insight-01"></a>[`REQ-INSIGHT-01`](../swe.1-requirements/02-software-requirements.md#req-insight-01) | Process-level tests for accepted syntax, mode conflicts, default output path, and exit codes | Inspect stderr diagnostics and confirm insight mode writes no stdout payload. |
| <a id="req-insight-02"></a>[`REQ-INSIGHT-02`](../swe.1-requirements/02-software-requirements.md#req-insight-02) | Hash and mtime the input archive before/after success and failure; pre-create output and compare its tree | Verify source bytes unchanged, no-overwrite behavior, atomic publication, and cleanup of incomplete output. |
| <a id="req-insight-03"></a>[`REQ-INSIGHT-03`](../swe.1-requirements/02-software-requirements.md#req-insight-03) | Generate from a deterministic synthetic ZIP; assert exact filenames and parse every JSON object | Validate the seven schema contracts and README required sections/line cap. |
| <a id="req-insight-04"></a>[`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04) | Domain fixtures for current/stale/undated evidence, absent sections, duplicate sources, and conflicting channels | Compare findings, provenance, counts, negative evidence, freshness, and uncertainty to the promoted schema clauses. |
| <a id="req-insight-05"></a>[`REQ-INSIGHT-05`](../swe.1-requirements/02-software-requirements.md#req-insight-05) | Boundary tests for README rows/lines, excerpt bytes, archive limits, malformed entries, and expansion limits | Assert hard bounds, explicit elision, and error rather than silent truncation. |

## Conformance completion criteria

### Acceptance review cases

These planned review cases clarify existing clauses; they are not new requirements, implemented tests or results.
The normative values and exceptions remain in the [requirement owner](../swe.1-requirements/02-software-requirements.md).
Assess every applicable clause; mark ambiguity unresolved instead of inventing a passing interpretation.

| Requirement | Case and expected observation to assess |
| --- | --- |
| REQ-GEN-01 | Open and exercise a ZIP in an isolated scratch directory; inspect file access and compare input bytes/output inventory to assess in-place access without extraction. |
| REQ-GEN-02 | Exercise metadata ceilings and produced-byte read ceilings at and above their boundaries, including lying sizes, skipped prefixes, stored/deflated chunk continuity and exact-ceiling EOF. Verify unread content remains unverified; aggregate admission uses declared sizes only. |
| REQ-LIB-01 | Inspect the library target/public API and normal dependency graph; use it from another crate without requiring CLI/MCP. Dev-only fixture dependencies do not imply runtime coupling. |
| REQ-MCP-01 | Use offline Cargo metadata to check separate library/executable targets and required production edges CLI→MCP, CLI→core and MCP→core. Reject direct/transitive MCP→CLI or core→workspace-sibling coupling; exclude dev-only fixtures and inspect the public serve launch path. |
| REQ-MCP-02 | Inspect startup binding and all registered parameters for a second-archive/switch path; compare repeated calls with the originally bound identity. |
| REQ-MCP-03 | Exercise read-only and safety-limit cases through the server path as well as core; a core-only pass does not establish adapter behavior. |
| REQ-MCP-04 | Exercise all seven tools and their failure paths; inspect writes and compare archive bytes and scratch-directory contents. Account explicitly for internal caches versus the wording about state. |
| REQ-MCP-05 | Inspect descriptions, input schemas and result contracts; compare the exact seven unique wire names and schemas independently of order under ADR 0011. |
| REQ-MCP-06 | For each successful tool path, assess the four mandatory fields, including partial success. Confirm domain and protocol/framework failures carry no successful envelope; distinguish structured metadata, bounded markers and complete resources. Exercise defaults, above-cap output, long lines and scan stops; verify sanitized UTF-8 bytes and source_bytes follow-up under ADR 0007; verify valid/missing/oversized/invalid/unreadable markers and source-ID recovery under ADR 0008. |
| REQ-MCP-08 | Compare completed no-hit, stopped no-hit, absent-entry, unreadable-entry and parameter-failure outcomes; cover listings/type filters as well as grep without treating skipped failures as complete misses. |
| REQ-MCP-09 | Apply locator checks to content-reading/search evidence, excluding structured discovery/archive metadata and error payloads. Follow each returned locator back to the known fixture position; compare kind, range, offset, excerpt and completeness with source bytes, including non-ASCII/NUL input. |
| REQ-MCP-10 | Inspect transport registration; capture stdout/stderr across success, malformed calls, startup failure and verbose mode. Assess the entire stdout stream, including blank output ignored by current tests. |
| REQ-MCP-11 | Inspect initialization for seven names, actual IO versus output bounds, intended order, confirmation/error/retry fields, line ceiling and absence of archive content; compare previews with source contracts, without treating estimates as CPU/time guarantees. |
| REQ-MCP-12 | Verify all five content tools and both resource methods challenge before reads; exercise matched/mismatched/replayed/expired tokens, independent pools, maximum pending/argument bounds and grep cursor/page independence; approved semantic/domain failures remain possible. |
| REQ-MCP-ARC-01 | Compare every returned identity/marker field with a known archive and independent digest; assess missing/long/non-UTF-8 markers and the optional-digest behavior against the obligation. |
| REQ-MCP-ARC-02 | Compare ID/path/size records with the fixture index; exercise absent/present prefixes, `ANR/` versus `ANR2/`, empty result and item omission. |
| REQ-MCP-ARC-03 | Use matching/nonmatching complete literal paths and segment-aware globs; verify no content reads by inspection and typed refusal of unsupported/malformed modes. |
| REQ-MCP-ARC-04 | Exercise each complete pair, neither/both/partial pairs, line 1, zero/invalid lines, byte zero, reversed/end-of-entry/past-end ranges and oversized excerpts. Check the accepted partial-return policy without relaxing read/CRC/safety errors. |
| REQ-MCP-ARC-05 | Search known hits/misses with explicit and whole-archive scope, match cap, long lines and budget stops within the final entry; compare entry counts with actual searched bytes and stop flags. |
| REQ-MCP-ARC-06 | Compare each required metadata field with the fixture; exercise detected text/binary/nested ZIP and an unreadable sample. Declared sizes are metadata, not proof of actual expansion. |
| REQ-MCP-ARC-07 | Exercise known types, unidentifiable bytes, no matches, invalid requested type and detection failures; assess whether the basis supports the classification and failures remain visible. |
| REQ-CLI-03 | Start a valid session; omit mode/archive; provide missing/bad ZIP; disconnect stdin and assess transport errors separately. Compare startup ordering and exact exit codes with the requirement. |
| REQ-CLI-04 | Invoke help and compare usage/options with supported arguments; expect successful termination and readable usage output. |
| REQ-CLI-05 | Compare equivalent ordinary/verbose sessions; assess additional stderr diagnostics and unchanged protocol-only stdout. |
| REQ-INSIGHT-01 | Run exact `--insight` syntax; test missing path, mode conflict, non-file path, help, and verbose diagnostics. |
| REQ-INSIGHT-02 | Compare source archive digest/mtime and existing output tree; inject a write failure and verify the final directory is absent. |
| REQ-INSIGHT-03 | Parse all six JSON files, compare exact output names, and check README sections, ten-row cap, and 400-line cap. |
| REQ-INSIGHT-04 | Use one fixture per analysis domain; assert required finding distinctions and evidence locators, plus explicit unavailable/partial states. |
| REQ-INSIGHT-05 | Exercise 512-byte excerpt and archive expansion/entry boundaries; assert flagged elision and no successful silent omission. |

### Verdict and evidence rules

Record clause outcomes before overall requirement verdicts.
**Passed** requires evidence for all applicable clauses; **failed** identifies an observed contradiction; **not demonstrated** identifies missing, skipped or inadequate evidence.
Distinguish demonstrated failures from unevaluated clauses within a requirement.
Use the [result-record fields](../verification/01-strategy.md#result-record-fields) to bind each measure to its execution or inspection.
Do not fill a verdict by inheriting a green suite count.

For every row/clause, SWE.6-T1 must compare assertions/results, record passed/failed/not demonstrated with revision/environment and explain gaps/conflicts.
The written report must separate archive immutability, bounded behavior, protocol correctness, and remaining discrepancies.
Counts, code links and historical demonstrations cannot replace the walkthrough.
[Shared strategy](../verification/01-strategy.md) defines result recording.

The [design decisions](../decisions/index.md) explain current contracts; the assessment separately identifies remaining verification limits.

## Search pagination acceptance evidence

[Search pagination results](../../verification/results/search-pagination.md) cover REQ-MCP-04/05/06/09 and REQ-MCP-ARC-05 through real wire budgets, read-only continuation, complete match delivery, bounded source consumption and honest skipped-content coverage.
The qualification assessment includes these targeted results in the completed requirement walk.

## Collection-outcome acceptance evidence

[Executed checks](../../verification/results/empty-and-detection-outcomes.md) map REQ-MCP-04/06/08, REQ-MCP-ARC-02/03/06 and ARCH-MCP-TOOLS-ARC/ARCH-MCP-SRV to clean misses, partial collections, bounded failure records, sample-only coverage and unchanged direct-entry error channels.
These targeted results support collection-status and explicit detection-failure clauses in the assessment.
[Unordered discovery evidence](../../verification/results/unordered-tool-discovery.md) separately closes subtask 1.7 under ADR 0011 with exact unique membership and schema checks.

[Range/prefix evidence](../../verification/results/range-boundaries.md) supports ADR 0012 range/subtree clauses in the assessment.

## Content IO confirmation acceptance evidence

[Executed confirmation record](../../verification/results/content-io-confirmation.md) maps REQ-MCP-04/05/06/07/11/12 to request-bound previews, accurate IO guidance, no pre-approval execution, bounded state and explicit retry/error channels.
This supports ADR 0013 confirmation clauses in the assessment; transport-failure evidence limits remain in the open obligations.

## Raised limits and scoped insight acceptance

| Requirement | Acceptance measure |
| --- | --- |
| REQ-CLI-12 / REQ-GEN-02 | A synthetic ZIP above the default entry count is refused with the measured limit and `--force` guidance; forced MCP and static insight succeed with a stderr warning and unchanged source bytes/mtime; selected produced-byte ceilings and integrity checks remain enforced. |
| REQ-MCP-13 | Exact combined CLI syntax launches successfully for file and directory; only immediate qualifying ZIP basenames are discovered, caller paths cannot escape, insight tools require confirmation before generation and return fixed, bounded artifact pages without filesystem writes. |
| <a id="req-mcp-14"></a>REQ-MCP-14 | Directory discovery exposes ten unique tool names; all seven archive input schemas accept an optional basename. Exercise omitted selection for empty, unique, ambiguous and truncated discovery; reject foreign/path selectors; verify selected-report metadata, entry-local addressing, report-local grep continuation, confirmation binding to the shared 64-token pool and context generation, the eight-context/300-second policy, read-only source preservation and bound-file selector matching. |
| REQ-MCP-02/04/05/06/10/11/12 / REQ-INSIGHT-01/05 | Existing archive-only discovery/protocol tests remain applicable; combined-mode tests verify new capability discovery, confirmation and paging without relaxing stdout purity or read-only behavior. |

The [targeted result](../../verification/results/force-and-scoped-insight-mcp.md) supplements the historical walkthrough for these changed clauses.
Unchanged requirements reuse existing fixture evidence with a fresh workspace regression run; native non-Unix transport-failure verification remains outside this change.
REQ-MCP-14 verification has not yet been executed; no verdict may be inferred from the updated tool-list assertion alone.
