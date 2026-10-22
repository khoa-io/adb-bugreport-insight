# 0011 — Unordered tool discovery

Status: Accepted

## Context

The handler vector and SDK-derived discovery are separate: the adapter selects handlers by name, and vector order does not control the discovery response.
Clients address tools by name rather than array position.

## Decision

tools/list must return exactly the enabled mode's tool names, each once, with descriptions and typed input schemas.
Ordinary archive mode has seven archive tools; combined file and directory modes expose those seven plus three insight tools.
Array order is unspecified; clients must not infer execution order or dependencies from discovery positions.
Initialization instructions retain recommended call sequencing under REQ-MCP-11.
Neither the documentation table nor the internal registry vector defines wire order.
Existing set-based wire assertions and separate schema checks directly exercise this contract.

## Consequences and evidence

Array order remains unspecified; clients must select tools by name.
The directory tool set and report-selector parameter are specified by [REQ-MCP-14](../swe.1-requirements/02-software-requirements.md#req-mcp-14) and [ADR 0015](0015-directory-archive-tool-selection.md).
[REQ-MCP-05](../swe.1-requirements/02-software-requirements.md#req-mcp-05) and the [MCP design](../swe.3-detailed-design/03-mcp-server.md) own the wire list contract.
The existing [unordered-discovery result](../../verification/results/unordered-tool-discovery.md) covers its recorded revision; REQ-MCP-14 requires fresh directory-mode verification.
Full SWE.6 qualification remains separate.
