# 0004 — Lazy archive validation

Status: Accepted

## Context

Actual expansion cannot be established without decompressing content.
Metadata-only startup and bounded selective access require a clear separation between admission and content validation.

## Decision

Opening checks stored file size, entry count and declared uncompressed entry/aggregate sizes without decompressing payloads.
Selected reads enforce the actual per-entry ceiling, counting produced bytes from the entry start, including skipped prefixes and discarded tails.
Hard overflow fails the read; chunk-stream EOF must not conceal it.
The 1 GiB aggregate is declared-only admission, not an actual expansion or session-work budget.
Unopened entries and unread suffixes remain unverified; neither open success nor a partial read certifies their CRC, size or readability.
CLI defaults remain the baseline; explicit `--force` selects the finite raised policy in REQ-CLI-12, and library callers retain coherent custom limits.
Readers enforce the selected per-entry and compression-ratio policy.

## Consequences

Startup stays proportional to archive metadata rather than decompression of every entry.
Corrupt or oversized actual payloads may fail later when inspected.
Partial reads may stop before discovering a corrupt suffix; this is an evidence boundary, not proof of archive health.
EntryChunks borrows EntryReader, retaining one decoder and shared Cursor guards; callers must retain the reader while iterating.
The borrowed lifetime preserves decoder continuity, and exceeding the ceiling remains a reported error.
Internal decoder work, cumulative session reads and actual whole-archive expansion are not bounded by the declared aggregate limit.

[Requirement](../swe.1-requirements/02-software-requirements.md#req-gen-02) · [Architecture](../swe.2-architecture/02-components-and-dependencies.md#lazy-archive-validation) · [Design](../swe.3-detailed-design/04-archive-safety.md) · [Verification](../../verification/results/lazy-archive-validation.md)
