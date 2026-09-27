# Requirement Diagram

This project provides a command-line tool for analyzing Android bugreport ZIP archives. The primary requirements are:

- **Read-only ZIP access**: Must open bugreport archives without extracting files to disk.
- **Safety limits**: Enforce size and entry count limits to prevent resource exhaustion.
- **Evidence extraction**: Scan logcat/dumpstate for ANR, crashes, memory pressure, CPU contention, and other signals.
- **Artifact generation**: Produce JSON/NDJSON findings, a Markdown README, a manifest, and session segmentation.
- **Extensibility**: Allow addition of new scanners and subcommand binaries without modifying the core.

These requirements drive the architecture and implementation decisions throughout the codebase.
