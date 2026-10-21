# 0013 — Confirmation before content IO

Status: Accepted

## Context

archive_info was advertised as index-only despite full stored-file hashing and repeated bounded marker reads.
A 512-byte excerpt or 200-match output limit does not bound skipped-prefix decompression, all-file detection, hashing or complete artifact reads.
The user chose explicit client review before these operations, extending beyond correcting description text alone.

## Decision

Keep metadata admission, initialize, tools/list, list_entries and find_entries immediate.
Gate archive_info, entry_info, find_by_type, read_entry, each grep_archive page, resources/list and resources/read before entering any content handler/provider.
Return confirmation_required with the exact operation, opaque token, conservative cost plan and 300-second expiry; execute only an explicit matching retry.
Tools retry with confirmation; resources retry with _meta["bugreport-insight/confirmation"].
Tool challenges use the separate isError/error body; resources use a JSON-RPC -32000 error with challenge fields in error.data.
There are no new tool names, automatic execution, blanket session grants or inherited page approvals.

The five tool wrappers share 64 pending requests; resources use a separate 64-request gate.
Each pending argument object is limited to 65,536 normalized JSON bytes to bound retained state as well as token count.
Exact operation/normalized-argument matching consumes one token before execution; mismatches preserve the original request, while replay, foreign tokens and expiry are refused.
Expiry cleanup is lazy on valid gate checks or shutdown; a full pool refuses new previews without work.
Confirmation is distinct from semantic validation, and approved execution can fail after consuming approval.
A grep preview does not consume, reserve or refresh its independently expiring cursor.

Plans report potential hash input and decoded source consumption from cached metadata and request limits; they do not guarantee physical IO, CPU or elapsed time.
Identity retains its existing first-choice digest cache and repeated marker reads; resource identity may require reopening after an earlier hash decline.
No core read, classification, hashing or resource-provider semantics change after approval.

## Consequences and evidence

Clients need one additional explicit preview/retry exchange for every content operation or search page; index queries remain useful without that exchange.
Clients decide whether the cost warrants execution; a token is a client retry capability, not proof of human identity or an authentication boundary.
Resources can require approval even when the final result is absent or a URI invalid, because provider validation may read a manifest or hash input.
Behavior-test helpers explicitly accept previews to retain prior outcome coverage; raw gate and stdio measures verify the production refusal boundary separately.
[REQ-MCP-12](../swe.1-requirements/02-software-requirements.md#req-mcp-12), [detailed protocol](../swe.3-detailed-design/03-mcp-server.md#content-io-confirmation) and [executed checks](../../verification/results/content-io-confirmation.md) own the exact contract and evidence.
Transport exits and full qualification remain separate decisions.
