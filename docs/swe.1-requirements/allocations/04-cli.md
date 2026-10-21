# Cli requirement view

Allocation of [committed requirements](../02-software-requirements.md).
[Traceability](../../traceability.md) owns allocation; [conventions](../../conventions.md) define notation.
Records allocation metadata without proving verification; canonical requirements and traceability own obligations.

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [REQ-CLI-03](../02-software-requirements.md#req-cli-03) | interfaceRequirement | `mcp_server` | medium | test |
| [REQ-CLI-04](../02-software-requirements.md#req-cli-04) | interfaceRequirement | `cli` | low | inspection |
| [REQ-CLI-05](../02-software-requirements.md#req-cli-05) | interfaceRequirement | `cli` | low | inspection |
| [REQ-CLI-12](../02-software-requirements.md#req-cli-12) | interfaceRequirement | `ARCH-CLI-FLAGS, ARCH-ARCHIVE, ARCH-INSIGHT-ANALYZERS` | high | boundary and process tests |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `cli` | interface | bugreport-insight binary |
| `mcp_server` | module | Live-inspection MCP server over stdio |
