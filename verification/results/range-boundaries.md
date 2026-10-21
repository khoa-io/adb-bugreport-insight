# Range boundaries and subtree filters — 2026-10-18

## Scope and environment

Tested tree: b31987d plus existing working changes and this accepted range/subtree implementation.
Environment: macOS arm64, Rust/Cargo 1.99.0, cached dependencies offline, synthetic ZIPs and existing reference suites.
[TASK019 subtask 1.8](../../docs/decisions/index.md) and [ADR 0012](../../docs/decisions/0012-range-boundaries-and-subtree-filters.md) record the accepted decision.

## Executed measures

| Measure | Observed result |
| --- | --- |
| [Core partial range check](../../crates/core/tests/partial_ranges.rs) | Available line/byte prefixes survive EOF; u64::MAX endpoints avoid arithmetic overflow; strict APIs still reject unavailable spans; partial APIs preserve produced-byte failures during skipped prefixes and output |
| range_boundaries_preserve_available_content_and_reject_invalid_endpoints in [component tools](../../crates/mcp/tests/tools.rs) | Partial and wholly past-end lines/bytes, exact final line/byte, equal byte endpoints, zero/reversed endpoints, empty and unterminated files, source immutability |
| subtree_filters_match_path_segments_and_keep_exact_entry_ids in component tools | ANR and ANR/ include directory/nested entries, exclude ANR2 and ANR-file, preserve IDs; nonmatching root is Empty and empty prefix selects all |
| eof_ranges_and_subtree_boundaries_work_over_stdio in [protocol tests](../../crates/cli/tests/protocol.rs) | Real CLI initialization, subtree discovery, partial/empty EOF envelopes, parameter errors, successful follow-up and unchanged archive bytes |
| Existing hostile/lazy/corrupt-entry and reference suites | Safety/decode failures and late random-access behavior retain their contracts |

The first component run correctly failed the old out_of_bounds_range_reports_unreadable expectation, which contradicted the newly accepted EOF policy.
Replaced that assertion with explicit empty-partial EOF fields; updated the wire domain-error case to retain missing-entry error/context verification rather than treating normal EOF as unreadable.
All final checks pass.

| Final gate | Result |
| --- | --- |
| cargo test --workspace --offline | Passed: 187 tests including reference suites; 42 component tool tests and 34 real protocol tests |
| cargo build --workspace --offline | Passed |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| cargo fmt --all -- --check | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Affected-file whitespace | Passed |

## Limits

Normal EOF uses Ok even with an empty payload; past_end/end_of_entry express the unavailable portion independently from known record omissions.
EOF takes envelope reason precedence over excerpt clipping; excerpt flags remain authoritative for represented content.
Core strict APIs remain available and MCP uses the new partial methods.
Byte intersections use admitted declared size and the guarded strict read; malformed premature EOF inside the expected intersection is an error.
As before, bounded byte reads need not validate the final CRC or unread suffix under lazy validation; flags do not certify whole-entry integrity.
This closes range/subtree discrepancies only; IO/cost claims, transport failures and full SWE.6-T1 qualification remain open.
