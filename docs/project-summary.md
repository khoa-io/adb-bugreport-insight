---
title: bugreport-insight — Project Summary
date: 2026-09-28
version: 1.0
audience: Engineering Team, Architects, Stakeholders
---

# bugreport-insight — Project Summary

`bugreport-insight` is a Rust CLI tool that analyzes Android `adb bugreport` ZIP archives **without extracting them**. It reads the archive in‑place, runs a suite of pattern‑based scanners over the logcat and dumpstate content, and writes structured insight artifacts (JSON/NDJSON) plus a concise Markdown report into a sibling `.BUGREPORT/insight/` directory.

## Documentation Overview

- **Requirements** – `docs/requirements.md`
- **Requirement Diagram** – `docs/requirement-diagram.md`
- **Traceability Matrix** – `docs/traceability-matrix.md`
- **Artifact Schemas (DRAFT)** – `docs/artifact-schemas.md`
- **Use Cases** – `docs/use-cases.md`
- **Architecture Diagram** – `docs/architecture.md`

`docs/classes.md`, `docs/sequences.md`, and `docs/activities.md` were removed during the redesign and
will be redrawn against the committed requirements once `core` and `mcp` are implemented.

Every specification document assigns IDs to its own elements — `ARCH-*` in the architecture diagram,
`UC-*` in the use cases — and links each one into [`docs/traceability-matrix.md`](./traceability-matrix.md)
alongside the requirements it serves. That matrix is the join between "what the tool must do" and
"what the design says will do it".

The `docs/index.md` file aggregates these references for easy navigation.

## Key Features

**Committed now ✅**

- Read-only ZIP access with size and entry-count limits; no temporary extraction (`REQ-GEN-01`, `REQ-GEN-02`).
- An MCP server for AI agents over stdio (`REQ-MCP-01` … `10`).
- Seven archive tools: `archive_info`, `list_entries`, `find_entries`, `read_entry`, `grep_archive`, `entry_info`, `find_by_type` (`REQ-MCP-ARC-01` … `07`).
- `--mcp`, `--help`, and `--verbose` (`REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05`).

**Draft 🚧 — analyzer work not started**

- Modular scanner plugins (ANR, stacktrace, memory, CPU, etc.) (`DRAFT-REQ-GEN-03`).
- Generates bounded findings with evidence excerpts, as artifacts (`DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08`).
- Subcommand dispatch for focused analysis (`bugreport-insight <verb>`) (`DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11`).

## Artifact Schemas (DRAFT)

The seven analysis artifacts — `README.md`, `system.json`, `apps.json`, `anr.json`,
`crashes.json`, `memory.json`, `cpu.json` — are specified in
[`docs/artifact-schemas.md`](./artifact-schemas.md), and constrained by the requirements
`DRAFT-REQ-ART-01` through `DRAFT-REQ-ART-08` in
[`docs/requirements.md`](./requirements.md). Those eight are draft, as is the section containing them
and everything else the analyzers produce: `DRAFT-REQ-GEN-03` and `DRAFT-REQ-GEN-04`, plus
`DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, and `DRAFT-REQ-CLI-06` through `DRAFT-REQ-CLI-11`. What is not
draft is what the tool needs first: reading and grepping the ZIP (`REQ-GEN-01`, `REQ-GEN-02`), the MCP
server (`REQ-MCP-01` through `REQ-MCP-06` and `REQ-MCP-08` through `REQ-MCP-10`), the seven
`REQ-MCP-ARC-*` archive tools, and the `--mcp`, `--help`, and `--verbose` options (`REQ-CLI-03`,
`REQ-CLI-04`, `REQ-CLI-05`). `DRAFT-REQ-MCP-07`, exposing the artifacts as MCP resources, is not being
implemented now.

Of 46 requirements, 22 are committed and 24 are draft.

The schemas were derived by reading the AOSP source that produces a bugreport, so every field
is traceable to evidence the archive actually contains. Two constraints shape most of them:

- **A bugreport's sections can be arbitrarily stale, and most carry no timestamp of their
  own.** Every finding therefore carries a `provenance` block and a four-state `freshness`
  (`now`, `in_window`, `stale`, `undated`). This is not hypothetical: the reference archive
  holds a complete ANR thread dump from four months before capture, sitting next to live
  logcat with nothing marking the difference.
- **A raw pattern-hit count is a trap.** In that same archive, `tombstone` matches 749 times
  with zero real tombstones among them. No artifact reports a bare count; every count carries
  a match block separating confirmed evidence from suppressed substring noise, together with
  the window coverage of the channel it came from.

## Getting Started

```bash
cargo build --workspace
cargo run -- <BUGREPORT_ZIP>
```

For detailed information, see the linked documentation files.
