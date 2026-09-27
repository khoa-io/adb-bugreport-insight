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
- **Artifact Schemas** – `docs/artifact-schemas.md`
- **Use Cases** – `docs/use-cases.md`
- **Architecture Diagram** – `docs/architecture.md`
- **Class Diagram** – `docs/classes.md`
- **Sequence Diagrams** – `docs/sequences.md`
- **Activity Diagrams** – `docs/activities.md`

The `docs/index.md` file aggregates these references for easy navigation.

## Key Features

- Read‑only ZIP access; no temporary extraction.
- Modular scanner plugins (ANR, stacktrace, memory, CPU, etc.).
- Generates bounded findings with evidence excerpts.
- Subcommand dispatch for focused analysis (`bugreport‑insight <verb>`).

## Decided Artifact Schemas

The seven analysis artifacts — `README.md`, `system.json`, `apps.json`, `anr.json`,
`crashes.json`, `memory.json`, `cpu.json` — are specified in
[`docs/artifact-schemas.md`](./artifact-schemas.md), and constrained by the requirements
`REQ-ART-01` through `REQ-ART-08` in [`docs/requirements.md`](./requirements.md).

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
