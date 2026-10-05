# Requirements

<!-- The committed specification. Every ID in this file is a binding requirement; **23 of them**, and all
23 are implemented. Requirements that describe output the tool does not produce live in
[`draft/requirements.md`](./draft/requirements.md) and are deliberately not here. -->

Each requirement carries a stable ID and an anchor, and the visible badge next to it links to its row
in [`diagrams/index.md`](./diagrams/index.md), the requirement-to-element index. The link runs both
ways: [`traceability-matrix.md`](./traceability-matrix.md) is the third join, mapping each
requirement to the design elements that serve it.

## Contents

- [General](#general) — 2
- [Core library](#core-library) — 1
- [MCP server](#mcp-server) — 10
- [MCP tools](#mcp-tools) — 7
- [Command-line interface](#command-line-interface) — 3

## General

- **Read-only ZIP access.** Open bugreport ZIP archives without extracting files to disk.
  <a id="req-gen-01"></a> ([`REQ-GEN-01`](./diagrams/index.md#req-gen-01))
- **Safety limits.** `bugreport-insight` refuses any archive that exceeds, at open time: **10,000**
  entries in the central directory; **256 MiB** uncompressed size of any single entry, enforced
  against the decompressed stream and not only the declared header; **1 GiB** total uncompressed
  across all entries; **2 GiB** archive file size. A violation is an **error**, never a silent
  truncation. The limits are constants of `crates/core` and are not configurable per run.
  Measurement point: archive open time, before any entry is read.
  <a id="req-gen-02"></a> ([`REQ-GEN-02`](./diagrams/index.md#req-gen-02))

## Core library

The archive half is a library, not only a mode of the binary: `bugreport-insight` is one consumer of
`bugreport-insight-core`, and other Rust projects are expected to be others.

- **Library distribution.** Distribute the archive engine as a library so other Rust crates can depend
  on it. `crates/core` depends only on external crates and never on a workspace sibling.
  <a id="req-lib-01"></a> ([`REQ-LIB-01`](./diagrams/index.md#req-lib-01))

## MCP server

`REQ-CLI-03` states that the `--mcp` flag exists. This section states what the server behind that flag
must do. The flag is the interface contract; this is the implementation contract. A reader who has
read only `REQ-CLI-03` knows the server can be started, and nothing about what it exposes, what it
refuses to do, or what an agent is allowed to conclude from a tool result.

- **Independent crate.** The server is its own crate, a peer of `cli`, not a mode compiled into the
  CLI binary. It depends on `bugreport-insight-core`; `cli` and `mcp` never depend on each other.
  <a id="req-mcp-01"></a> ([`REQ-MCP-01`](./diagrams/index.md#req-mcp-01))
- **Single-archive scope.** One server instance is bound to exactly one `BUGREPORT` archive for its
  lifetime. There is no multi-archive query, no archive list, and no way to open a second archive
  mid-session. <a id="req-mcp-02"></a> ([`REQ-MCP-02`](./diagrams/index.md#req-mcp-02))
- **Read-only over the archive.** The server reads the archive in place and never extracts it, under
  the same size and entry-count limits as `REQ-GEN-01` and `REQ-GEN-02`. Exposing an archive to an
  agent does not relax the read-only guarantee. <a id="req-mcp-03"></a> ([`REQ-MCP-03`](./diagrams/index.md#req-mcp-03))
- **Read-only tool surface.** No tool mutates the archive, any output directory, or any other state.
  The server's job is to answer questions about one already-opened archive.
  <a id="req-mcp-04"></a> ([`REQ-MCP-04`](./diagrams/index.md#req-mcp-04))
- **Tool naming and typing.** Every tool is named, described, and declared with a typed input schema
  (JSON Schema, derived via `schemars`) and a documented result shape, so an agent can call it
  without reading the implementation. A regression test asserts the `tools/list` response carries
  exactly the seven committed tool names, in order, with their schemas.
  <a id="req-mcp-05"></a> ([`REQ-MCP-05`](./diagrams/index.md#req-mcp-05))
- **Bounded responses.** Every tool result carries `truncated: bool`, `stopped_reason: enum | null`,
  `omitted_count: int ≥ 0`, and `notes: string[]` — **all four always present, never omitted**.
  `stopped_reason` ∈ `{item_cap, match_cap, scan_budget, line_too_long, byte_budget}`. Defaults:
  **200** items per list result (`list_entries`, `find_entries`, `find_by_type`), **50** grep matches
  (`grep_archive`), **32 MiB** scanned per grep call. Excerpt cap **512 bytes**, hard. A capped
  result set reports what it omitted. A tool never returns an archive section verbatim. Measurement
  point: the result envelope returned to the agent over stdio.
  <a id="req-mcp-06"></a> ([`REQ-MCP-06`](./diagrams/index.md#req-mcp-06))
- **Empty is not error.** A search that matched nothing returns a successful result stating that
  nothing matched, distinguishable from a tool that failed, an entry that was absent, and an entry
  that could not be read. `status` is one of `ok`, `empty`, `not_found`, `unreadable`, and a failure
  is never shaped like a successful empty result.
  <a id="req-mcp-08"></a> ([`REQ-MCP-08`](./diagrams/index.md#req-mcp-08))
- **Evidence discipline.** A tool result states evidence as a locator — archive `entry_id`, kind
  (`text` | `binary` | `nested_zip`), line or byte range with `byte_offset` included for
  `grep_archive`, and a bounded excerpt capped at **512 bytes** with truncation and completeness
  flagged — never as a copy of the source. The locator is always an `entry_id`, never a raw path
  string. <a id="req-mcp-09"></a> ([`REQ-MCP-09`](./diagrams/index.md#req-mcp-09))
- **stdio is the required transport.** The server speaks MCP over stdio. SSE and HTTP transports are
  not part of this requirement set and are not implemented. stdout carries JSON-RPC frames and
  nothing else; every diagnostic goes to stderr, and `--verbose` sets the stderr level only.
  <a id="req-mcp-10"></a> ([`REQ-MCP-10`](./diagrams/index.md#req-mcp-10))
- **`initialize` carries `instructions`.** The `initialize` response carries `instructions`: at most
  **40 lines** naming each of the seven tools, its cost class, its bound, and the intended call order.
  It publishes no archive content, and it is the one place a new agent learns the intended order
  without spending a tool call to discover it.
  <a id="req-mcp-11"></a> ([`REQ-MCP-11`](./diagrams/index.md#req-mcp-11))

## MCP tools

The tool surface is the server's actual product, so each tool carries its own ID. All seven map
one-to-one onto capabilities `crates/core` already owns — listing, reading, finding, grepping, and
per-entry metadata — and **none of them runs an analyzer**. They are specified individually in
[`diagrams/req-mcp-tools.md`](./diagrams/req-mcp-tools.md).

| ID | Tool | Returns |
| --- | --- | --- |
| <a id="req-mcp-arc-01"></a>[`REQ-MCP-ARC-01`](./diagrams/index.md#req-mcp-arc-01) | `archive_info` | Archive identity and shape: basename, size, SHA-256 over the stored bytes, entry count, `version.txt` verbatim, and the main entry's name and size |
| <a id="req-mcp-arc-02"></a>[`REQ-MCP-ARC-02`](./diagrams/index.md#req-mcp-arc-02) | `list_entries` | Entry paths and sizes, optionally scoped to one subtree prefix |
| <a id="req-mcp-arc-03"></a>[`REQ-MCP-ARC-03`](./diagrams/index.md#req-mcp-arc-03) | `find_entries` | Paths matching a glob or literal, with no content |
| <a id="req-mcp-arc-04"></a>[`REQ-MCP-ARC-04`](./diagrams/index.md#req-mcp-arc-04) | `read_entry` | A bounded line or byte range of one entry |
| <a id="req-mcp-arc-05"></a>[`REQ-MCP-ARC-05`](./diagrams/index.md#req-mcp-arc-05) | `grep_archive` | Content search, returning matches as locators |
| <a id="req-mcp-arc-06"></a>[`REQ-MCP-ARC-06`](./diagrams/index.md#req-mcp-arc-06) | `entry_info` | One entry's compressed and uncompressed size, compression method, modification time, detected content type, and nested-archive status |
| <a id="req-mcp-arc-07"></a>[`REQ-MCP-ARC-07`](./diagrams/index.md#req-mcp-arc-07) | `find_by_type` | Paths whose entries are of a given detected content type, with the basis for each detection |

The constraints that shape these seven are stated once rather than seven times:

- **Addressing is by `entry_id`, always.** Every tool takes an `entry_id: u32` from `list_entries` or
  `find_entries`. A path is never an input. This removes path traversal, encoding mismatch, duplicate
  entry names, and glob-versus-literal ambiguity in one move. `list_entries` accepts an optional
  `path_prefix`, which is a display convenience scoped to one subtree and does not change how results
  are addressed.
- **Format scope is ZIP only.** `crates/core` implements no TAR. Multi-format support is
  [`DRAFT-REQ-MCP-ARC-09`](./draft/requirements.md#draft-req-mcp-arc-09).
- **Pattern modes are literal and glob.** `regex` is a typed refusal, never a silent fallback to
  literal: [`DRAFT-REQ-MCP-ARC-08`](./draft/requirements.md#draft-req-mcp-arc-08).
- **`read_entry` takes flat range fields.** Exactly one of (`line_start`, `line_end`) or
  (`byte_start`, `byte_end`) must be specified. The mutual exclusion is a validation error rather
  than an enum, because an externally-tagged enum produces an `inputSchema` with no root `"type"`,
  which is not a schema the MCP specification permits. A range past end-of-entry returns what exists
  plus the fact that it did.
- **Content detection reports its basis.** An entry is called text or binary from its bytes; a more
  specific type is claimed only when archive metadata or byte content supports it. An unidentifiable
  entry is reported as unidentifiable, never guessed at.

## Command-line interface

- **`--mcp`.** `bugreport-insight BUGREPORT --mcp` starts a server bound to exactly one `BUGREPORT`
  archive for its lifetime (`REQ-MCP-02`), speaking MCP over stdio (`REQ-MCP-10`). The flag is the
  interface contract; [the MCP server section](#mcp-server) is the implementation contract. The
  archive is opened **before** the transport starts, so a server that cannot see its archive exits
  non-zero with a diagnostic on stderr rather than answering as though the archive were empty.
  <a id="req-cli-03"></a> ([`REQ-CLI-03`](./diagrams/index.md#req-cli-03))
- **`--help`.** Display usage information and available options.
  <a id="req-cli-04"></a> ([`REQ-CLI-04`](./diagrams/index.md#req-cli-04))
- **`--verbose`.** Enable verbose logging for debugging. It sets the log level on **stderr** via
  `eprintln!`; stdout carries JSON-RPC under `--mcp` and is never a logging channel.
  <a id="req-cli-05"></a> ([`REQ-CLI-05`](./diagrams/index.md#req-cli-05))

Exit codes are 0 for success, 2 for bad usage, and 1 for an archive or transport failure.

The artifact-generation command and its seven analysis toggles are
[draft](./draft/requirements.md#command-line-interface-draft) and are not implemented.

## Verification

Each requirement's risk and verification method are recorded in
[`diagrams/index.md`](./diagrams/index.md). The property that the committed scope is *read-only* —
nothing extracted, written, or modified — is enforced mechanically by a grep assertion in the test
suite and by stdout carrying JSON-RPC frames and nothing else.
