# Mcp server requirement view

Allocation of [committed requirements](../02-software-requirements.md).
[Traceability](../../traceability.md) owns allocation; [conventions](../../conventions.md) define notation.
Records allocation metadata without proving verification; canonical requirements and traceability own obligations.

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [REQ-MCP-01](../02-software-requirements.md#req-mcp-01) | designConstraint | `mcp_server` | medium | resolved production graph and composition-entry inspection |
| [REQ-MCP-02](../02-software-requirements.md#req-mcp-02) | designConstraint | `mcp_server` | medium | inspection |
| [REQ-MCP-03](../02-software-requirements.md#req-mcp-03) | requirement | `mcp_server` | high | test |
| [REQ-MCP-04](../02-software-requirements.md#req-mcp-04) | designConstraint | `mcp_server` | high | inspection |
| [REQ-MCP-05](../02-software-requirements.md#req-mcp-05) | interfaceRequirement | `mcp_server` | high | test |
| [REQ-MCP-06](../02-software-requirements.md#req-mcp-06) | performanceRequirement | `mcp_server` | high | test |
| [REQ-MCP-07](../02-software-requirements.md#req-mcp-07) | interfaceRequirement | `mcp_server`, `mcp_resources` | medium | test |
| [REQ-MCP-08](../02-software-requirements.md#req-mcp-08) | requirement | `mcp_server` | high | test |
| [REQ-MCP-09](../02-software-requirements.md#req-mcp-09) | requirement | `mcp_server` | high | test |
| [REQ-MCP-10](../02-software-requirements.md#req-mcp-10) | interfaceRequirement | `mcp_server` | medium | test |
| [REQ-MCP-11](../02-software-requirements.md#req-mcp-11) | interfaceRequirement | `mcp_server` | medium | test |
| [REQ-MCP-12](../02-software-requirements.md#req-mcp-12) | interfaceRequirement | `mcp_server`, `mcp_tools`, `mcp_resources` | high | request-bound/no-IO, lifecycle and real-stdio tests |
| [REQ-MCP-13](../02-software-requirements.md#req-mcp-13) | interfaceRequirement | `ARCH-MCP-INSIGHT, ARCH-MCP-SRV` | high | boundary and process tests |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `mcp_server` | module | Live-inspection MCP server over stdio |

`mcp_resources` is `ARCH-MCP-RES`: exact, bounded reads of generated artifacts for the server’s bound archive.

`mcp_tools` is `ARCH-MCP-TOOLS-ARC`: registry confirmation wrappers precede content handlers; `ARCH-MCP-SRV` gates resource calls before provider work.
