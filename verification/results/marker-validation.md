# Marker validation — 2026-10-18

## Scope and environment

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing working changes and the accepted marker policy implementation.
Environment: macOS, Rust/Cargo 1.99.0, offline cached dependencies.
The user accepted [ADR 0008](../../docs/decisions/0008-validated-marker-metadata.md) during the top-to-bottom archive_info walkthrough.
REQ-MCP-06/REQ-MCP-ARC-01 and ARCH-ARCHIVE/ARCH-MCP-TOOLS-ARC own the bounded marker and wire contracts.

## Targeted checks

[Core marker tests](../../crates/core/tests/markers.rs) pass four cases:

- Exact LF/CRLF removal, preservation of surrounding spaces/Unicode, exact main-entry lookup and distinct empty valid text.
- Exact 256-byte acceptance versus 257/300-byte withholding, without returning a prefix or incorrectly deriving format 0.
- Missing, invalid UTF-8, NUL and CRC-unreadable markers remain distinct; identity metadata survives.
- Understated 4-byte metadata over a 300-byte payload is rejected by the actual 256-byte stream guard; a smaller custom library reader ceiling yields unreadable, without falsely claiming the marker exceeded 256 bytes.

[Real-stdio protocol test](../../crates/cli/tests/protocol.rs) archive_info_reports_marker_states_and_allows_source_follow_up verifies structured states, null bad values, marker IDs, explanatory notes, oversize envelope bounds and continued session use.
Byte-range follow-up read_entry succeeds for readable invalid/oversized marker sources; a line-based follow-up over the corrupt marker fixture reaches CRC validation and returns domain failure while later listing still works.
An initial byte-range follow-up assertion incorrectly assumed an exact-length bounded read must validate CRC; it failed, and the regression now selects the line path explicitly. Partial byte reads retain the accepted lazy-validation boundary and do not certify whole-entry CRC.
[Insight regression](../../crates/core/src/insight/tests.rs) invalid_version_marker_does_not_become_format_zero proves absent version alone derives 0, while invalid version omits the version finding attribute and adds marker status/evidence gap.
That regression initially exposed the closed finding-attribute rule forbidding null attribute values; the correction omits the unknown attribute instead of inventing a scalar version.

Commands: cargo test -p bugreport-insight-core --test markers --offline; cargo test -p bugreport-insight-cli --test protocol --offline archive_info_reports_marker_states_and_allows_source_follow_up; cargo test -p bugreport-insight-core --lib --offline invalid_version_marker_does_not_become_format_zero.

## Workspace validation

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | Passed |
| cargo build --workspace --offline | Passed |
| cargo test --workspace --offline | Passed: 168 tests including private reference suites |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Affected-area whitespace check | Passed |

## Boundaries and compatibility

The markers object is additive to archive identity; legacy format_version/main_entry fields now become null for invalid/oversized/unreadable markers instead of returning guessed text or failing the entire metadata call.
Marker source_bytes is an actual full length only after a successful bounded read; declared_size_bytes remains untrusted metadata, and unavailable lengths stay null.
Normal source acceptance is 256 bytes; the guarded decoder may observe one extra probe byte to reject understated size claims.
Status valid establishes this byte/encoding policy, not numeric format semantics or existence of a named target entry.
Core ArchiveFormat0 notes remain tied only to absence; generated artifact source shapes remain unchanged, while the system finding adds a scalar marker-status attribute and uses an evidence gap for unknown version.
Hash behavior, scanner budgets/coverage and complete resource contracts remain unchanged; full qualification remains open.
