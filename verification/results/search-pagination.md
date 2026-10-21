# Search pagination verification — 2026-10-18

Scope: TASK019 subtask 1.5, accepted [ADR 0009](../../docs/decisions/0009-client-search-budgets-and-pagination.md).
This record concerns the uncommitted worktree on HEAD b31987d, including prior marker/excerpt fixes and existing insight/resource work; it does not claim full SWE.6-T1 qualification.
Environment: Darwin arm64, rustc 1.99.0 (b940084d7 2026-09-28); deterministic synthetic stored/deflated ZIPs plus the existing reference suites.

## Selected checks

| Obligations | Checks and expected result | Verdict |
| --- | --- | --- |
| REQ-MCP-06/09, REQ-MCP-ARC-05; ARCH-ARCHIVE | Core scan_pages: one-byte budgets retain UTF-8 and final unterminated lines, no invented LF, no prefix replay, deduplicated scope; anchored globs never match partial lines | Pass |
| REQ-GEN-02, REQ-MCP-06; ARCH-ARCHIVE | Deflated decoder survives pages and cumulative entry ceiling still rejects overflow; long lines become explicit gaps while later lines remain searchable | Pass |
| REQ-MCP-04/06; ARCH-MCP-TOOLS-ARC | Handler lifecycle tests: maximum eight live sessions, expiry/replay refusal, too-small locator budget preserves existing continuation for larger retry | Pass |
| REQ-MCP-04/05/06/09, REQ-MCP-ARC-05; ARCH-MCP-SRV | Real CLI wire tests: every successful result <=2,048 serialized bytes in old/new protocol variants; every page consumes <=600 new source bytes and <=3 matches; all 75 lines delivered exactly once; summed source bytes equal input; archive stays byte-identical | Pass |
| REQ-MCP-06/08/09; ARCH-MCP-SRV | Empty partial pages carry cursor, invalid budgets/query changes preserve cursor, final page establishes completion, overlong/encrypted gaps never claim complete/empty search | Pass |

Source/check locations: [core search](../../crates/core/src/search.rs), [decoder](../../crates/core/src/reader.rs), [core tests](../../crates/core/tests/scan_pages.rs), [pagination and lifecycle tests](../../crates/mcp/src/tools/pagination.rs), [stdio tests](../../crates/cli/tests/protocol.rs).

## Execution

Initial workspace regression exposed a changed pattern_mode_unavailable error code; restored the existing code and reran successfully before adding the final gap checks.
Focused core/lifecycle checks passed during iteration.
The encrypted-only gap test caught an incorrect line_too_long reason; corrected it to skipped_content.
Clippy identified three test loops better expressed with while-let; corrected them and reran the affected core checks and Clippy.

| Command | Actual result |
| --- | --- |
| `cargo test --workspace --offline` | Pass: 177 tests, zero failures, including reference suites and three added wire flows |
| `cargo test -p bugreport-insight-core --test scan_pages --offline` | Pass: four tests after lint-only cleanup |
| `cargo fmt --all -- --check` | Pass |
| `cargo build --workspace --offline` | Pass |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Pass |
| `python3 scripts/check_docs.py` | Pass: 174 Markdown pages, zero link/presentation errors |
| `python3 scripts/diagrams.py check` | Pass: nine fresh SVGs and valid diagram notation/links; no diagram sources changed in this subtask |
| Affected-area `git diff --check` | Pass; unrelated protected instruction-file whitespace excluded |

Completion requires all selected checks above to pass; no staged content, commits or external messages were produced.

## Boundaries

Budgets bound successful tool-result JSON and newly observed decompressed source bytes, not tokens, outer JSON-RPC frames, decoder internal work or CPU time.
One indivisible locator can require a larger response budget; no total page count is known without completing the search.
External archive mutation remains outside the snapshot guarantee; cursors are local to one handler process, expire after 300 idle seconds, and abandoned state is reclaimed on a subsequent grep call or shutdown.
Lines over 1 MiB are explicitly skipped and keep coverage incomplete; pagination does not claim to search their content.
General empty/detection status policy and remaining discrepancy items stay open.
