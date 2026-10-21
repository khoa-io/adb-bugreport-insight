# Tool response scope — 2026-10-18

## Scope and tested tree

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing uncommitted feature/documentation changes and accepted response-scope contract/test updates.
Environment: macOS, Rust/Cargo 1.99.0, cached dependencies and offline Cargo commands.
[ADR 0006](../../docs/decisions/0006-tool-response-scope.md) records user acceptance for TASK019 subtask 1.3; [REQ-MCP-06](../../docs/swe.1-requirements/02-software-requirements.md#req-mcp-06) and [REQ-MCP-09](../../docs/swe.1-requirements/02-software-requirements.md#req-mcp-09) own the revised applicability.
Runtime behavior is unchanged; production source changes clarify comments about successful versus error results.

## Verification selection

Strengthened [real-process protocol checks](../../crates/cli/tests/protocol.rs) because the accepted rule concerns caller-visible wire shapes.

| Check | Expected assertion | Result |
| --- | --- | --- |
| all_seven_tools_answer_over_stdio | All successful tools have correctly typed bounding fields and no error body; MCP result is not isError | Passed |
| partial_success_retains_bounding_fields_without_becoming_an_error | Listing one of three entries remains successful with item_cap and two omissions | Passed |
| domain_failure_is_a_tool_error_not_a_protocol_error | Missing and unreadable-range failures carry code/message/entry/status, omit all successful-envelope fields, and permit subsequent successful calls | Passed |
| both_range_pairs_is_a_protocol_error | Local parameter conflict is a JSON-RPC error with no successful result/completeness fields | Passed |
| unknown_parameter_is_rejected_with_a_reason | SDK deserialization refusal uses isError without a top-level protocol error on the current SDK | Passed |

Source inspection of [ToolEnvelope](../../crates/mcp/src/tools/envelope.rs), [ToolError](../../crates/mcp/src/error.rs) and [adapter mapping](../../crates/mcp/src/adapter.rs) confirms the separate shapes.
[Identity](../../crates/core/src/identity.rs) builds structured archive metadata and reads marker input under its 256-source-byte bound, without a content locator.
Existing [resource wire checks](../../crates/cli/tests/protocol.rs) verify exact complete generated artifact reads under their separate contract.

## Validation

| Command | Result |
| --- | --- |
| cargo test -p bugreport-insight-cli --test protocol --offline | Passed: 25 tests |
| cargo fmt --all -- --check | Passed |
| cargo build --workspace --offline | Passed |
| cargo test --workspace --offline | Passed: 154 tests, including private reference checks |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Affected-area whitespace check | Passed |

## Limits

This is targeted evidence for response-shape applicability, not full REQ-MCP-06/09 conformance.
Excerpt caps, lossy UTF-8 byte accounting, marker verbatim/normalized wording and clipping signals remain open under subtask 1.4.
Status consistency, scanner completeness, SDK error wording and full software qualification are not established by these assertions.
Framework failures may use different channels; only the current wrapper path is asserted here.
No production response schema or runtime mapping changed.
