# 0005 — CLI composes MCP and core

Status: Accepted

## Context

The CLI starts the MCP server and invokes core insight generation.
Crate reuse depends on production dependency direction and public responsibilities, while the executable may link those libraries.

## Decision

CLI owns arguments and mode selection and depends on both MCP and core.
MCP is a reusable library owning protocol handling; it depends on core and must not depend directly or transitively on CLI.
Core has no production dependency on another workspace sibling.
Production dependency review includes normal/build edges and excludes development-only fixture dependencies.
A separate library crate may be linked into the executable; crate independence does not require a separate server process or binary.

## Consequences

The launcher and public server entry point follow this composition graph.
Core remains reusable by other consumers, while protocol adaptation remains in MCP.
Future dependency changes must preserve these directions; compilation alone does not prove this boundary.

[Requirement](../swe.1-requirements/02-software-requirements.md#req-mcp-01) · [Architecture](../swe.2-architecture/02-components-and-dependencies.md#dependency-rules-and-observed-graph) · [Design](../swe.3-detailed-design/03-mcp-server.md#ownership-and-parameters) · [Verification](../../verification/results/cli-mcp-dependencies.md)
