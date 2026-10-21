# Mcp tools requirement view

Allocation of [committed requirements](../02-software-requirements.md).
[Traceability](../../traceability.md) owns allocation; [conventions](../../conventions.md) define notation.
Records allocation metadata without proving verification; canonical requirements and traceability own obligations.

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [REQ-MCP-ARC-01](../02-software-requirements.md#req-mcp-arc-01) | interfaceRequirement | `mcp_server`, `core_archive` | medium | test |
| [REQ-MCP-ARC-02](../02-software-requirements.md#req-mcp-arc-02) | interfaceRequirement | `mcp_server`, `core_archive` | medium | test |
| [REQ-MCP-ARC-03](../02-software-requirements.md#req-mcp-arc-03) | interfaceRequirement | `mcp_server`, `core_archive` | medium | test |
| [REQ-MCP-ARC-04](../02-software-requirements.md#req-mcp-arc-04) | interfaceRequirement | `mcp_server`, `core_archive` | high | test |
| [REQ-MCP-ARC-05](../02-software-requirements.md#req-mcp-arc-05) | performanceRequirement | `mcp_server`, `core_archive` | high | test |
| [REQ-MCP-ARC-06](../02-software-requirements.md#req-mcp-arc-06) | interfaceRequirement | `mcp_server`, `core_archive` | medium | test |
| [REQ-MCP-ARC-07](../02-software-requirements.md#req-mcp-arc-07) | interfaceRequirement | `mcp_server`, `core_archive` | medium | test |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `mcp_server` | module | Live-inspection MCP server over stdio |
| `core_archive` | module | Read-only ZIP list, read, find, and grep |
