# 0009 — Client search budgets and cursor pagination

Status: Accepted

## Context

A bounded search must expose partial coverage and allow the client to continue without replaying decompression or dropping a line across page boundaries.
Separate response and source-byte budgets let a client control delivered output and incremental scanning while keeping completeness claims explicit.

## Decision

New grep calls supply a pattern, optional mode/scope, and optional response_budget, scan_budget and max_matches.
Continuation calls supply cursor and optional budgets/max_matches, omitting query fields; the saved query and archive binding cannot change.
Response budget defaults to 65,536 bytes and accepts 2,048 through 262,144 bytes.
The bound covers successful serialized MCP CallToolResult JSON, including escaped envelope text and cursor, excluding the outer JSON-RPC frame, request ID and transport newline.
It does not promise bounded error messages, tokens or client rendering size.
A conservative SDK-wrapper allowance is included before accepting each locator; a locator that cannot fit is rejected with an instruction to increase the budget, preserving an existing continuation for retry.
Scan budget defaults/maxes at 33,554,432 bytes and accepts a minimum of one byte; max_matches defaults/maxes at 50 and accepts a minimum of one.
Out-of-range budgets are rejected rather than silently clamped.

Each page reads at most scan_budget new decompressed source bytes, including line terminators and discarded overlong-line data.
Retained carry bytes were charged when first read; processing them on a later page does not recharge or replay decompression.
Internal decoder work, compressed IO, CPU and wall time are not bounded by that source-byte budget.
The cumulative per-entry safety ceiling and ratio checks survive page boundaries.
Reaching a budget without observing EOF remains conservatively incomplete, including an exact-size boundary; a later page can observe EOF without consuming source bytes.
An empty page can carry next_cursor; it does not prove absence of matches elsewhere.
No page count or future match count is promised before searching the remainder.

Matching runs only on complete logical lines, with one first-occurrence record per matching line.
A partial line survives scan-budget boundaries, preserving UTF-8 sequences and anchored glob semantics.
Lines longer than 1 MiB are skipped as whole matching units; their tails are consumed under subsequent page budgets, and later lines remain searchable.
Each such line increments skipped_lines once and prevents complete coverage for the search, even after exhaustion.
This explicit gap avoids returning a prefix match as if it represented a complete anchored line.
Directory entries require no content scan and count as completed; encrypted files count as skipped entries and prevent complete coverage.
Other decode/CRC/safety errors fail the call and invalidate that continuation, rather than becoming an empty success.

Coverage counters are cumulative except scanned_bytes, which counts new source bytes for the current page; total_scanned_bytes is cumulative.
scanned_entries counts started files; completed_entries counts EOF-completed files and directories; entries_not_searched counts entries not yet visited; partial_entry_id identifies the active decoder when present.
coverage_complete requires exhausted scope, no pending match delivery, and no skipped entries/lines.
Pending observed matches are retained for the next page; omitted_count remains zero because they are deferred, not discarded.
scan_budget, match_cap and byte_budget identify page boundaries; line_too_long or skipped_content explain terminal coverage gaps.
Excerpt-only clipping remains independent of scan coverage.

## Ownership and consequences

Core ScanSession owns query, counters, carry/partial-line buffers and a demand-driven decoder worker for the active entry.
The parked worker keeps a borrowed ZIP decoder alive safely without self-referential structs, unsafe code, whole-entry buffering or prefix replay.
Dropping a session disconnects and joins its worker; the worker reads only when requested and validates produced bytes before yielding them.
MCP retains at most eight live continuations per handler behind a mutex; handler dispatch remains synchronous.
Tokens are opaque, process-local and single-use, rotate after each successful continuation, and expire after 300 seconds without use.
Expired state is reclaimed on the next validly budgeted grep call or handler destruction, so expiry does not imply a background timer.
Concurrent searches use independent entry decoders; this is a scoped exception to ADR 0003's stateless-call description, not a shared ZIP cursor.
The archive must remain externally unchanged during a session, as with existing readers; no snapshot is created.

## Owners and evidence

- Contract: [requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-06), [MCP design](../swe.3-detailed-design/03-mcp-server.md#search-pages).
- Implementation: [core search](../../crates/core/src/search.rs), [reader worker](../../crates/core/src/reader.rs), [pagination](../../crates/mcp/src/tools/pagination.rs).
- Verification: [executed results](../../verification/results/search-pagination.md).
