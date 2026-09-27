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

## Getting Started

```bash
cargo build --workspace
cargo run -- <BUGREPORT_ZIP>
```

For detailed information, see the linked documentation files.
