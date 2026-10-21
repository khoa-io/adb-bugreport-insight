# Empty and detection outcomes — 2026-10-18

Scope: TASK019 subtask 1.6; accepted [ADR 0010](../../docs/decisions/0010-empty-results-and-detection-failures.md).
Revision: HEAD b31987d plus the uncommitted worktree, including prior pagination/marker fixes and existing insight/resource work.
Environment: Darwin arm64, rustc 1.99.0 (b940084d7 2026-09-28); synthetic stored ZIPs with encrypted flags and existing workspace/reference fixtures.
No real bugreport content is added to this record.

## Measures and expected results

| Obligations | Check specification and expected result | Verdict |
| --- | --- | --- |
| REQ-MCP-06/08, REQ-MCP-ARC-02/03 | collection_misses_are_empty_but_zero_output_limits_are_partial_ok: clean listing/path/type misses are Empty, while zero output limits withholding known matches are capped Ok | Pass |
| REQ-MCP-08, REQ-MCP-ARC-06 | type_queries_preserve_failures_and_distinguish_unknown_from_unreadable: readable matches and Unknown survive alongside explicit encrypted-entry failures; file sample coverage is false; direct entry_info retains its domain error | Pass |
| REQ-MCP-06/08 | type_failures_and_matches_have_separate_bounded_omission_counts: two matching records plus 200 failure details retained; 298 matches and five failures omitted; every classification outcome counted | Pass |
| REQ-MCP-08 | all_failed_and_directory_only_type_queries_have_honest_outcomes: all-failed is partial Ok, while directory-only and empty ZIP scopes are clean Empty | Pass |
| REQ-MCP-ARC-06 | type_coverage_describes_samples_rather_than_full_file_validation: successful classification reports 8 KiB sample/basis despite an uninspected NUL-containing tail | Pass |
| REQ-MCP-04/06/08, REQ-MCP-ARC-06 | empty_collections_and_partial_detection_failures_are_distinct_over_stdio: real CLI/MCP returns clean Empty misses, partial failure records, successful Unknown classification and direct unreadable errors; session survives and archive stays byte-identical | Pass |

Implementation/check locations: [handlers](../../crates/mcp/src/tools/handlers.rs), [adapter](../../crates/mcp/src/adapter.rs), [component tests](../../crates/mcp/tests/tools.rs), [stdio checks](../../crates/cli/tests/protocol.rs), [shared encrypted-flag fixture helper](../../crates/test-fixtures/src/lib.rs).
Core detection/reader behavior is unchanged; existing core suites supply regression coverage.

## Final gates

| Command | Actual result |
| --- | --- |
| `cargo test -p bugreport-insight-mcp --test tools --offline` | Pass: 40 component tests, including five added outcome cases |
| `cargo test -p bugreport-insight-cli --test protocol --offline` | Pass: 33 real stdio tests, including the added collection-outcome flow |
| `cargo test --workspace --offline` | Pass: 183 tests, zero failures, including reference suites |
| `cargo fmt --all -- --check` | Pass |
| `cargo build --workspace --offline` | Pass |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Pass |
| `python3 scripts/check_docs.py` | Pass: 176 Markdown pages, zero link/presentation errors |
| `python3 scripts/diagrams.py check` | Pass: nine fresh SVGs; no diagram sources changed in this subtask |
| Affected-area `git diff --check` | Pass; unrelated protected instruction-file whitespace excluded |

No check failures occurred during this change.
Existing staged/unstaged work was preserved; no commits or external messages were produced.

## Limits

Classification coverage concerns promised samples and their existing basis, never full-file/CRC validation under lazy reads.
At most 200 matching records and 200 failure records are returned independently; omitted detail is counted explicitly, without pretending unknown matches in failed files were observed.
All-failed collections remain successful partial queries; direct-entry failures retain separate domain errors.
Grep cursor/coverage behavior and other existing success/error boundaries are unchanged.
This targeted evidence does not close tool ordering, ranges, IO/exit decisions or full SWE.6-T1 qualification.
