# 0003 — Immutable index and independent readers

Status: Accepted

## Context and alternatives

ZIP entry access borrows a mutable parser.
A shared ZIP parser cursor would couple request lifetimes and decoder state.
Reopening paths for every reader would allow later symlink/path replacement to redirect authorized access.
Separate metadata permits cheap discovery and isolates reader lifetimes from server state.

## Decision and consequences

Index the central directory immutably at startup; create owned readers for content operations.
Share the archive/index and digest cache through Arc; keep each ZIP parser and logical position independent.
The source is one pinned file behind a short-held mutex; each read atomically seeks and reads at its logical position.
This serializes descriptor IO without sharing ZIP parser/decoder positions.
The cost is reparsing and replaying decompression for later offsets.
Detection also reads samples and identity can hash/read markers; only listing and path filtering are index-only.
Archive-tool handler dispatch is synchronous; resource-provider reads use blocking workers under their separate contract.
This proves neither executor responsiveness nor protection against external archive mutation after indexing.

Current owners: [runtime](../swe.2-architecture/03-runtime-and-interactions.md), [archive contract](../swe.3-detailed-design/01-core-archive-contract.md).

[ADR 0009](0009-client-search-budgets-and-pagination.md) adds a scoped exception: grep retains bounded independent decoders across cursor pages behind a handler session mutex, avoiding prefix replay without a shared ZIP cursor.
