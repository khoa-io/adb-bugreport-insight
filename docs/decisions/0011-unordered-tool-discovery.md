# 0011 — Unordered tool discovery

Status: Accepted

## Context

The handler vector and SDK-derived discovery are separate: the adapter selects handlers by name, and vector order does not control the discovery response.
Clients address tools by name rather than array position.

## Decision

tools/list must return exactly the enabled mode's tool names, each once, with descriptions and typed input schemas.
Ordinary archive mode has seven archive tools; combined file mode adds three insight tools; confined directory mode exposes only the three insight tools under REQ-MCP-13.
Array order is unspecified; clients must not infer execution order or dependencies from discovery positions.
Initialization instructions retain recommended call sequencing under REQ-MCP-11.
Neither the documentation table nor the internal registry vector defines wire order.
Existing set-based wire assertions and separate schema checks directly exercise this contract.

## Consequences and evidence

No runtime routing, tool names, parameters or response shapes change.
Clients relying on a particular discovery order must instead select tools by name.
[Requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-05), [MCP design](../swe.3-detailed-design/03-mcp-server.md) and [verification](../../verification/results/unordered-tool-discovery.md) own the contract and targeted verdict.
Full SWE.6 qualification remains separate.
