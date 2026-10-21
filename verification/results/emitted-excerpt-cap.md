# Emitted UTF-8 excerpt cap — 2026-10-18

## Scope

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing working changes and accepted excerpt-cap implementation.
Environment: macOS, Rust/Cargo 1.99.0, offline cached dependencies.
The user accepted the end-to-end line/byte/grep example and authorized implementation of [ADR 0007](../../docs/decisions/0007-emitted-utf8-excerpt-cap.md).
REQ-MCP-06/09, ARCH-ARCHIVE and ARCH-MCP-TOOLS-ARC own the changed cap/conversion/envelope contracts.
Marker policy is intentionally separate; TASK019 subtask 1.4 remains in progress until that part is decided.

## Checks

Four core regression checks failed before the emitted-cap implementation: line overflow, replacement expansion, small/JSON-escaping caps and lossy completeness.
The shared converter now caps emitted text, preserves whole characters and maps source-prefix progress; line/byte/grep paths share that representation.
[Core text checks](../../crates/core/src/text.rs) cover exact/above 512, multibyte boundary crossings, 512 NUL/invalid bytes expanding into replacement text, zero/small/custom caps, and JSON escaping outside the text cap.
[Wire checks](../../crates/cli/tests/protocol.rs) cover 600-byte cafe text: first 512 bytes, explicit remaining 88 bytes, unchanged repeat requests and repeatable one-record-per-line grep.
A second wire case covers 500 ASCII bytes plus eight NUL bytes: first output is 512 emitted bytes representing 504 source bytes, followed by the last four source NULs as 12 emitted bytes.
These assertions distinguish source positions, emitted text, excerpt completeness, envelope truncation, omitted item counts and scanner coverage.
Five core text checks and all 28 real-process protocol tests pass in focused runs.

## Validation

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | Passed |
| cargo build --workspace --offline | Passed |
| cargo test --workspace --offline | Passed: 162 tests, including private reference suites |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Affected-area whitespace check | Passed |

## Limits

The excerpt cap does not constrain complete MCP response size, retained reader buffers, decompression work or scanner budgets.
Grep matching still uses retained line bytes and existing first-match-per-line behavior; scanner coverage/budget discrepancies remain open under subtask 1.5.
Arbitrary requested byte endpoints can split source encoding; replacement is then explicit, rather than silently changing requested coordinates.
Source_bytes is additive and bytes now means emitted length; callers relying on the old source-byte meaning must use source_bytes.
Markers and complete insight resources are outside this implementation change.
Full software qualification remains open.
