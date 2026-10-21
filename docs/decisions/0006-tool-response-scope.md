# 0006 — Successful results, errors and metadata

Status: Accepted

## Context

Successful bounded evidence, domain failures and protocol rejection communicate different outcomes.
Applying success completeness fields or content locators to every category would let a failure appear as complete empty evidence.

## Decision

All successful archive-tool results, including empty/partial success, include truncated, stopped_reason, omitted_count and notes.
Domain failures use MCP isError true and structured error code/message/data, retaining addressed-entry context when available; they omit successful-envelope completeness fields.
Protocol/framework failures use their rejection/error channels, with no successful envelope; SDK parameter failures may use tool-error content instead of JSON-RPC error.
Content-reading/search evidence uses entry locators and bounded excerpts; structured discovery/archive metadata needs no artificial content locator.
Separately bounded format/main-entry marker text is an explicit metadata exception to content-excerpt formatting.
Complete insight resources retain REQ-MCP-07 rather than inheriting archive-tool excerpt limits or success envelopes.

## Consequences

Clients distinguish error channels before interpreting completeness fields and do not infer archive health from a failed or partial operation.
A protocol rejection need not share the domain-error JSON body.
[ADR 0007](0007-emitted-utf8-excerpt-cap.md), [ADR 0008](0008-validated-marker-metadata.md) and [ADR 0009](0009-client-search-budgets-and-pagination.md) separately define sanitized excerpt accounting, strict marker validation and scan coverage.

[Requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-06), [design](../swe.3-detailed-design/03-mcp-server.md#success-and-domain-error-shapes) and [wire evidence](../../verification/results/tool-response-scope.md) own the exact contracts and demonstrated scope.
