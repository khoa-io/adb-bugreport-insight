# 0008 — Validate marker identifiers instead of clipping them

Status: Accepted

## Decision

Version/main-entry markers accept at most 256 actual source bytes including a terminal line ending, with strict UTF-8 and no NUL.
Remove exactly one terminal LF/CRLF and preserve every other space/character; no version-number or filename grammar is added.
Declared oversized markers are withheld without reading; a bounded stream with one overflow probe checks actual sizes when metadata claims a small entry.
Unavailable marker values are null, accompanied by explicit status, source EntryId when present, declared size and a stable error code.
Successful complete reads also report original source byte length and line-ending removal.
Missing markers have explicit missing state; only absent version.txt retains format-0 handling.

## Consequences

archive_info returns other available metadata when marker validation or reading fails, rather than returning clipped or lossy guessed identifiers.
Oversized marker withholding sets envelope truncated true, byte_budget and omitted_count zero; invalid/unreadable cases use explicit status and notes without pretending text was clipped.
Agents can inspect source entries through bounded read_entry; a corrupt source may still return a domain failure while the session remains available.
Validated main-entry names use exact ZIP metadata lookup, preserving spaces; lack of a matching entry leaves the size null.
SHA hashing failures retain their existing independent failure path.
Insight source fields remain nullable; missing alone can derive format 0, while invalid versions retain an evidence gap and marker-status finding attribute without inventing a version value.
The 256-byte marker acceptance limit is separate from the 512-byte excerpt rule and archive expansion limits.

[Requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-06) · [Wire design](../swe.3-detailed-design/03-mcp-server.md#success-and-domain-error-shapes) · [Verification](../../verification/results/marker-validation.md)
