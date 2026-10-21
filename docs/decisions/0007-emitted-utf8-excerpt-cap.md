# 0007 — Cap emitted UTF-8 excerpts and retain source progress

Status: Accepted

## Decision

Each line, byte-range and grep-match excerpt is at most 512 emitted UTF-8 bytes after decoding and sanitization, before JSON escaping.
Keep the longest whole-character prefix; preserve valid UTF-8 and replace malformed bytes/NUL explicitly.
Excerpt bytes counts emitted text; source_bytes counts the represented source prefix.
Source line lengths, offsets and bytes_returned stay in original source-byte coordinates.
Clipping sets truncated true and complete false, reaching the successful tool envelope as byte_budget unless a scanner/item/retention reason already applies.
Omitted_count continues to count withheld items/matches, not clipped bytes.
Lossy replacement makes complete false even without clipping.
The cap is per excerpt, not per response, and is independent of scan/decompression budgets and complete-artifact resources.

## Follow-up and consequences

Given `café` repeated 120 times (600 bytes), the first excerpt is 102 repetitions plus `ca` (512 bytes), followed by `fé` plus 17 repetitions (88 bytes) in an explicit source-byte request.
Repeated read_entry or grep_archive requests return the same result rather than advancing a hidden cursor.
Use source start plus source_bytes for continuation; emitted bytes can differ after replacement expansion.
Grep still returns one record per matching line, with a line-start offset; this decision does not add occurrence pagination or change matching/search budgets.
Adding source_bytes is an additive wire field; changing bytes to emitted length is a deliberate contract correction, and callers using it as a source count must switch to source_bytes.
Markers retain their separate 256-source-byte metadata contract pending a distinct representation/clipping decision.

[Requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-06) · [Design](../swe.3-detailed-design/03-mcp-server.md#success-and-domain-error-shapes) · [Verification](../../verification/results/emitted-excerpt-cap.md)
