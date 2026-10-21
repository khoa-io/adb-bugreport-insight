# Deferred requirement view — MCP tools

> **Proposal only.**
> Definitions: [deferred requirements](../02-requirements.md).
> Notation: [conventions](../../conventions.md).

## Draft — the deferred tool surface

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [DRAFT-REQ-MCP-ARC-08](../02-requirements.md#draft-req-mcp-arc-08) | interfaceRequirement | `core_archive` | medium | test |
| [DRAFT-REQ-MCP-ARC-09](../02-requirements.md#draft-req-mcp-arc-09) | interfaceRequirement | `core_archive` | medium | test |
| [DRAFT-REQ-MCP-ANR-01](../02-requirements.md#draft-req-mcp-anr-01) | interfaceRequirement | `mcp_server`, `analyzer_crates` | high | test |
| [DRAFT-REQ-MCP-APP-01](../02-requirements.md#draft-req-mcp-app-01) | interfaceRequirement | `mcp_server`, `analyzer_crates` | medium | test |
| [DRAFT-REQ-MCP-CRS-01](../02-requirements.md#draft-req-mcp-crs-01) | interfaceRequirement | `mcp_server`, `analyzer_crates` | high | test |
| [DRAFT-REQ-MCP-MEM-01](../02-requirements.md#draft-req-mcp-mem-01) | interfaceRequirement | `mcp_server`, `analyzer_crates` | high | test |
| [DRAFT-REQ-MCP-CPU-01](../02-requirements.md#draft-req-mcp-cpu-01) | interfaceRequirement | `mcp_server`, `analyzer_crates` | medium | test |
| [DRAFT-REQ-MCP-TOOLS-ANALYSIS](../02-requirements.md#draft-req-mcp-tools-analysis) | designConstraint | `analyzer_crates` | high | test |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `mcp_server` | module | Live-inspection MCP server over stdio |
| `core_archive` | module | Read-only ZIP list, read, find, and grep |
| `analyzer_crates` | module | Analyzer crates anr, apps, stacktrace, memory, cpu |

`DRAFT-REQ-MCP-TOOLS-ANALYSIS` is modelled as a `designConstraint` on `analyzer_crates` rather than as a tool.
It constrains analyzer wording; archive tools make no causal claims, so `core_archive` has no allocation edge.

Artifact resource allocation belongs to [REQ-MCP-07](../../swe.1-requirements/allocations/02-mcp-server.md); the superseded proposal ID remains in the requirement owner.

Generic in-memory insight discovery/list/read tools are committed under [REQ-MCP-13](../../swe.1-requirements/02-software-requirements.md#req-mcp-13), independently of these deferred domain-specific interfaces.
