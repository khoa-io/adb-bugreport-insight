# Unordered tool discovery — 2026-10-18

## Scope and environment

Tested tree: b31987d plus existing working changes and this accepted REQ-MCP-05 clarification.
Environment: macOS arm64, Rust/Cargo 1.99.0, offline cached dependencies, synthetic ZIP fixtures.
[TASK019 subtask 1.7](../../docs/decisions/index.md) records acceptance; [ADR 0011](../../docs/decisions/0011-unordered-tool-discovery.md) defines unordered discovery.

## Measures and results

Executed cargo test -p bugreport-insight-cli --test protocol --offline.
The existing tools_list_returns_exactly_the_seven_committed_tools test launches the CLI, initializes MCP and compares sorted discovered names with the seven committed names.
List equality preserves multiplicity: missing, duplicate and extra names fail while permutations pass.
The separate every_tool_declares_a_closed_schema test checks every discovered tool has a nonempty description and an object input schema with additionalProperties false.
The instructions_name_every_tool_and_stay_short test checks discovery guidance still names all tools and meets its line ceiling.
Inspection of adapter instructions confirms the recommended call sequence remains present; this does not certify the separate pending IO/cost claims.
Inspection of the registry and adapter confirms handler lookup uses names and SDK-generated discovery is independent of registry vector order.

| Check | Result |
| --- | --- |
| Real CLI/MCP protocol suite | Passed: 33 tests |
| cargo fmt --all -- --check | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Whitespace for affected files | Passed |

## Limits

Only documentation and Rust documentation comments changed; no runtime implementation or test logic changed.
The existing executable wire suite directly verifies the accepted contract; no new order assertion or duplicate test was added.
No full workspace build/test/Clippy rerun was selected for this wording-only resolution; prior 183-test evidence remains historical, not a new execution here.
This closes discovery ordering only; ranges, IO, transport exits and full SWE.6 qualification remain separate.
