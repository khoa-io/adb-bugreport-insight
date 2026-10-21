# 0001 — Committed archive scope

Status: Accepted

## Context and alternatives

Archive exploration, static insight generation and generated resources have distinct responsibilities.
A component that also serves deferred analysis tools would blur the boundary between committed and proposed scope.
Separate allocation allows archive access to remain independently reusable.

## Decision and consequences

Separate ARCH-ARCHIVE, ARCH-MCP-SRV and ARCH-MCP-TOOLS-ARC from deferred analyzer-tool and batch elements.
Static insight and generated-resource components have their own committed allocations.
Keep DRAFT prefixes stable and retired IDs reserved under the [ID rules](../conventions.md#ids-and-links).
Archive inspection does not require running analyzers; generation and resource reads retain their separate contracts.
Promotion of a proposal requires explicit scope approval and coordinated requirement, architecture and traceability changes.

Current owners: [conventions](../conventions.md), [components](../swe.2-architecture/02-components-and-dependencies.md) and [proposals](../proposals/index.md).
