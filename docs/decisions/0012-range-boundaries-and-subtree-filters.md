# 0012 — Partial EOF reads and subtree boundaries

Status: Accepted

## Context

REQ-MCP-ARC-04 promised available content for past-end requests, while strict core spans returned errors.
Reversed MCP byte endpoints silently became zero-length reads through saturating subtraction.
REQ-MCP-ARC-02 promised subtree scope, while a raw prefix ANR also matched ANR2.

## Decision

MCP read_entry uses partial core range methods, preserving available content when requests extend past EOF.
Return data.past_end on every successful read; true uses Ok, truncated true, stopped_reason end_of_entry and omitted_count zero.
A wholly past-end request returns an empty payload with the same flags; unavailable nonexistent content is not counted as omitted.
Requested range fields retain the original endpoints; returned line locations or bytes_returned describe the actual content.
Exact final lines and byte ranges ending exactly at EOF are not past-end.
Equal byte endpoints are valid empty ranges; they are past-end only when the offset itself exceeds EOF.
Line endpoints are inclusive and require 1 <= start <= end; byte endpoints are half-open and require start <= end.
Invalid endpoints yield invalid_range parameter errors before reader creation.
EOF takes envelope stop-reason precedence over excerpt and long-line clipping; per-excerpt flags still expose clipping.
Decoder, CRC and safety failures remain errors; partial EOF handling never converts them to successful data.
Strict core APIs remain available to callers requiring the whole span.
Byte intersections use admitted declared size, preserving lazy-validation limits; EOF flags do not promise complete payload integrity verification.

list_entries removes trailing slashes from path_prefix, then matches an exact path or a slash segment boundary.
ANR and ANR/ select the same subtree and exclude ANR2, including the subtree directory entry when present.
Empty and slash-only prefixes select all entries; matching remains case-sensitive without dot-segment resolution.
Entry IDs remain the only addressed-entry keys.

## Consequences and evidence

Clients can request a useful range without first knowing the exact line count or byte size.
Clients must inspect past_end separately from excerpt truncation and handle malformed ranges as parameter errors.
Clients needing arbitrary filename-prefix matching use find_entries patterns rather than path_prefix.
[Canonical MCP contract](../swe.3-detailed-design/03-mcp-server.md), [core contract](../swe.3-detailed-design/01-core-archive-contract.md) and [executed boundary checks](../../verification/results/range-boundaries.md) define implementation and evidence.
IO/cost guidance, transport exit behavior and full qualification remain separate.
