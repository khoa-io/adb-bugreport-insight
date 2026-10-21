# CLI and MCP dependency boundary — 2026-10-18

## Scope and environment

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing working changes and this accepted REQ-MCP-01 clarification.
Environment: macOS, Cargo 1.99.0, locally cached dependency resolution, offline operation.
[TASK019 subtask 1.2](../../docs/decisions/index.md) records the user's acceptance; [ADR 0005](../../docs/decisions/0005-cli-composes-mcp-and-core.md) defines the dependency rule.

## Method and evidence

Executed `cargo metadata --offline --format-version 1` against the current workspace.
Inspected resolved package targets and normal/build dependency edges, excluding development-only edges.
Asserted CLI directly depends on MCP and core, MCP directly depends on core, MCP cannot reach CLI transitively, and core cannot reach any workspace sibling through production edges.
The resolved graph is:

| Crate | Direct workspace production dependencies | Verdict |
| --- | --- | --- |
| bugreport-insight-cli | bugreport-insight-core, bugreport-insight-mcp | Passed |
| bugreport-insight-mcp | bugreport-insight-core | Passed |
| bugreport-insight-core | None | Passed |

[CLI manifest](../../crates/cli/Cargo.toml), [MCP manifest](../../crates/mcp/Cargo.toml) and [core manifest](../../crates/core/Cargo.toml) agree with that graph.
Source inspection confirms [main](../../crates/cli/src/main.rs) parses arguments, calls MCP serve for MCP mode and calls core generation for insight mode.
[MCP library](../../crates/mcp/src/lib.rs) exports serve; [server](../../crates/mcp/src/server.rs) owns stdio lifecycle and archive opening.
These observations demonstrate separate reusable crates with permitted linking, rather than a prohibition on executable composition.

## Validation and limits

| Method | Result |
| --- | --- |
| Offline Cargo metadata, resolved graph assertions and source inspection | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Whitespace check for affected documents and manifest comment | Passed |

Only contract documentation and a manifest comment changed for this resolution.
No Cargo build/test/Clippy rerun was selected: dependency declarations, Rust code and runtime behavior are unchanged; current metadata/source inspection directly verifies the accepted design constraint.
The preceding lazy-validation record's 153-test run remains historical evidence for that change, not a newly executed test run here.
This closes the dependency conflict, not the remaining discrepancy groups or full SWE.6 qualification.
