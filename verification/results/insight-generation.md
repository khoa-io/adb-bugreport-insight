# Insight generation verification — 2026-10-17

`UC-03` now generates the seven promoted artifacts through six evidence analyzers.
The selected SWE.1–SWE.6 feature work and executable checks are complete.
This record supersedes the [partial scaffold milestone](../../docs/swe.3-detailed-design/05-insight-generation.md), retaining that milestone as history.
It is targeted evidence for `REQ-INSIGHT-01`–`05`, not a qualification of the earlier 23 requirements or an ASPICE assessment.

## Tested tree and environment

- Base revision: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64`, plus the uncommitted insight source, fixture, requirements, architecture, schemas, diagrams and verification changes in this worktree.
- Platform: macOS, Apple silicon; Rust 2024 workspace.
- Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`.
- Offline dependency resolution succeeded; added core dependencies reuse cached `chrono`, `regex-automata` and `libc`.
- Synthetic source: shared `insight_archive()` fixture, with Android-15-shaped package/process records, current Java/ANR events, stale ANR/native evidence, CPU windows, procstats and swap snapshots.
- Real fixture: untracked `tmp/reference-archive.zip`, 14,710,126 bytes, 398 ZIP entries, SHA-256 `10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92`.
- The reference acceptance test ran against that matching fixture; it did not take its optional absent-fixture skip.
- Real report contents and generated artifacts stay under ignored `tmp/`; this record retains only identity and aggregate measurements.

The [design](../../docs/swe.3-detailed-design/05-insight-generation.md) and [generation rules](../../docs/proposals/artifact-schemas/12-generation-rules.md) resolve promoted-proposal ambiguities before the final checks.
Those decisions cover capture-reference shape, null clocks, closed severity rules, stronger deduplication, freshness-separated groups, explicit thresholds and archive-wide raw-search scope.
The unrelated pre-existing MCP usage-guide edits were preserved.
No project files were staged or committed.

## Selected checks and actual execution

[Unit measures](../../docs/swe.4-unit-verification/01-verification-measures.md#insight-units), [component measures](../../docs/swe.5-component-and-integration-verification/01-component-verification.md#insight-component-boundary), and [process measures](../../docs/swe.5-component-and-integration-verification/02-integration-and-verification.md#insight-process) name the assertions and owners.

| Command or method | Actual result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo build --workspace --offline` | Passed. |
| `cargo test --workspace --offline` | 137 passed; 0 failed/ignored: 39 core unit, 20 hostile, 19 original reference, 1 insight reference, 35 MCP handler, 22 protocol process, 1 insight domain process. |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Passed without warnings. |
| `python3 scripts/check_docs.py` | Passed; 160 Markdown pages and 1,725 local links, with zero errors. |
| `python3 scripts/diagrams.py render` | Passed; regenerated the use-case and two architecture views through the pinned renderer. |
| `python3 scripts/diagrams.py check` | Passed; nine SVGs fresh with valid links/notation. |
| SVG visual review through raster previews | New component/runtime layouts readable, no clipped labels or overlapping message text. |
| `git diff --check` | Passed. |
| `cargo run --offline --bin bugreport-insight -- --insight tmp/reference-archive.zip` | Exit 0; seven artifacts published; no stdout analysis payload. |
| Repeat exact binary invocation against that existing output | Exit 1; stderr explains existing output; every output-file SHA-256 unchanged. |
| Compare real input before/after generation | SHA-256, 14,710,126-byte length and nanosecond mtime unchanged. |
| Inspect publication tree | Exactly seven final files; no `.insight-stage-*` leftovers. |
| Independently follow finding locators with Python ZIP reads | All 42 nonempty finding excerpts match original bytes and their 1-based line numbers; top table has exactly ten rows. |

An intermediate cohort regression assertion initially required only `in_window`; the capture-instant fixture correctly produced `now`.
The assertion now accepts both current freshness classes and passes while still requiring separate stale/current groups.
The first updated runtime diagram label began with `--mcp`, which PlantUML interpreted as an illegal arrow; the label was corrected and render/check rerun successfully.
These intermediate failures are resolved by the final executions above.

## Requirement clauses and verdicts

| Requirement | Demonstrating checks/review | Targeted verdict |
| --- | --- | --- |
| `REQ-INSIGHT-01` | Real-process accepted syntax, missing/invalid input, exclusive mode/positional arguments, sibling destination, stderr/exit codes, and six-domain assertions in `insight_cli_analyzes_all_domains_and_preserves_input_bytes_and_metadata`. | Passed. |
| `REQ-INSIGHT-02` | Synthetic bytes/mtime comparison, real SHA/mtime comparison, output collision hashes, injected write failure, racing empty/nonempty destination preservation; source review shows read-only core readers, fixed output names and one exclusive final rename. | Passed for publication and immutability; existing archive-limit timing caveat below remains. |
| `REQ-INSIGHT-03` | Exact file-set assertions, parsed JSON objects, required envelope/domain-field and cross-field runtime validation, nine README sections, synthetic domain values and independently measured reference counts. | Passed for the promoted contracts with the documented generation decisions. |
| `REQ-INSIGHT-04` | Domain unit fixtures verify current/stale/undated distinctions, cross-boot traces, Java/native separation, exact deduplication, freshness cohorts, inferred ANR joins, window-local process identities, counts/coverage, nulls/gaps, sampled contention and no collector-causation inference. | Passed for supported evidence layouts; missing/unrecognized evidence stays explicit. |
| `REQ-INSIGHT-05` | UTF-8/NUL excerpt boundary, section-budget failure, source review of 128 MiB/2,000,000-line/20,000-section/50,000-region ceilings, README ten-row/90-locator caps and final 400-line guard; hostile archive boundary/streaming regressions passed. | Passed for insight bounds and inherited streaming guards; original GEN-02 timing is not newly qualified. |

Review additionally followed all new findings through `Artifact::finish`, their analyzer locators, the closed rule table and the validator.
Severity/freshness summaries derive from emitted findings and close exactly; count partitions reject inconsistencies.
`system.json` owns capture, the five others reference it, and all six share source/generator context.
Core code never uses archive paths as output paths and never opens nested ZIP contents for analysis.
The sole platform-specific unsafe operation passes owned NUL-terminated paths to exclusive rename; racing-target tests demonstrate its macOS behavior.

## Real-report trial observations

| Measurement | Observed |
| --- | --- |
| README | 122 lines; ten top-finding rows; all nine sections. |
| Main report newline count | 682,951. |
| Current retained log window | 64.662 seconds; cold-boot context. |
| Inventory | 282 packages, 83 process records, 564 per-user install-state blocks, two users and two displays. |
| ANR | Zero current incidents; 65 retained trace files; last-ANR section has 31 processes and stale provenance. |
| Java crashes | 271 deduplicated incidents from 274 raw fatal lines; one current crash-loop group. |
| Native crashes | 50 historical incidents, grouped as 47 and 3 by normalized stack signatures. |
| Memory | Total RAM 3,047,508 KiB; total PSS 2,007,345 KiB; three procstats blocks; four swap snapshots. |
| CPU | Four cumulative core records; 1,772 ms pre-dump sample; 168 sampled lock rows; 30 distinct Java-monitor events, 17 tagged system_server. |

The raw monitor channel has 33 lines; none of the 17 system_server waits reaches one second.
This corrects the earlier schema example's broad zero-system_server claim without guessing an ART debug-library gate.
The native records and retained ANR files remain historical and cannot inflate current incident counts.
Raw archive-wide keyword counts include outer side text, so they intentionally exceed earlier main-only example searches.

The final local review artifact is `tmp/.reference-archive/insight/README.md`, with its six adjacent JSON files.
An earlier trial directory was renamed to `insight-first-trial` before the final invocation; the application's no-overwrite policy was exercised rather than bypassed.

## Limits and exclusions

- The runtime validator checks required field structures and invariants; no standalone machine JSON Schema was supplied by the proposal, and this run does not claim independent JSON Schema certification.
- OEM layouts that do not determine a value produce null/gaps; absent channels do not establish device health.
- Nested ZIP contents and binary protobuf payloads remain excluded under the archive-reader contract, with coverage/gaps recorded.
- macOS publication was executed; Linux/Windows implementations were reviewed but not executed on those platforms.
- Existing `REQ-GEN-02` open-time-versus-streaming enforcement and other earlier [requirement discrepancies](../../docs/decisions/index.md) remain separately tracked.
  Insight reuses those guards; passing regressions do not resolve the baseline wording conflict or qualify the earlier 23 obligations.
- Batch/NDJSON, undefined manifest/session segmentation, per-domain CLI switches, analyzer MCP tools and MCP resources remain deferred by the agreed promotion boundary.
- Hosted CI, independent external review and power-loss durability testing were not run; local build/test/lint/doc/diagram checks provide the targeted evidence recorded here.

No required selected feature check remains unexecuted.
Full product conformance stays under SWE.6-T1.

## Successor decision — 2026-10-18

The archive-limit timing caveat above records the requirement in force for this historical execution.
The user subsequently selected [lazy archive validation](../../docs/decisions/0004-lazy-archive-validation.md); [targeted verification](lazy-archive-validation.md) records the revised policy and reader correction without rewriting this earlier result.
