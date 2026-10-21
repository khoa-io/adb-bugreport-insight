# UTF-8 line excerpt correction — 2026-10-18

## Scope and tested tree

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing working changes and this focused decoding correction.
Environment: macOS, Rust/Cargo 1.99.0, cached dependencies and offline Cargo commands.
The user requested fixing UTF-8 corruption before deciding TASK019 subtask 1.4's excerpt-byte policy.
REQ-MCP-09 requires accurate source-content evidence; ARCH-ARCHIVE owns the conversion, consumed by MCP read_entry.

## Reproduction and correction

Before the fix, LineSlice::to_excerpt converted each byte to a character, turning café into cafÃ© while incorrectly labeling it Utf8.
Both new core regressions failed on the prior implementation, including multibyte text adjacent to a NUL byte.
The converter now decodes UTF-8 byte sequences together, preserving valid characters and replacing malformed sequences/NUL with U+FFFD.
Existing Binary/Utf8Lossy classifications for NUL/invalid input are retained.
Source byte counts, source offsets, line retention limits, clipping/completeness semantics and marker handling are unchanged.
No acceptance of the proposed emitted-byte cap is inferred from this bug-fix authorization.

## Targeted checks

| Check | Assertion | Result |
| --- | --- | --- |
| text::tests::line_excerpt_preserves_multibyte_utf8 | Exact empty/ASCII/accented/Vietnamese/Japanese/emoji/combining text and Utf8 classification | Passed after failing before correction |
| text::tests::line_excerpt_replaces_invalid_utf8_and_nul_without_corrupting_valid_text | Replacement of invalid/truncated sequences and NUL; neighboring valid multibyte text preserved | Passed after failing before correction |
| line_read_preserves_utf8_text_over_stdio | Real CLI/MCP line read preserves exact multilingual text, Utf8 classification, byte count, full source length and offset | Passed |

Commands: `cargo test -p bugreport-insight-core --lib --offline line_excerpt` and `cargo test -p bugreport-insight-cli --test protocol --offline line_read_preserves_utf8_text_over_stdio`.
[Core tests](../../crates/core/src/text.rs), [wire test](../../crates/cli/tests/protocol.rs), [design](../../docs/swe.3-detailed-design/04-archive-safety.md#memory-decoding-and-addressing) and [traceability](../../docs/traceability.md#req-mcp-09) locate the evidence.

## Workspace validation

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | Passed |
| cargo build --workspace --offline | Passed |
| cargo test --workspace --offline | Passed: 157 tests, including reference suites |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Affected-area whitespace check | Passed |

## Limits

This fixes decoding accuracy, not the 512-byte cap or source-versus-emitted byte-count policy.
Lossy-input completeness metadata, marker normalization/clipping, scan coverage and full REQ-MCP-06/09 qualification remain open.
TASK019 stays 3 of 11 complete until subtask 1.4's remaining policy and implementation work is resolved.
