# ADB Bugreport Insight

`bugreport-insight` opens an Android `adb bugreport` ZIP in place and exposes seven archive-inspection tools to an AI assistant through an MCP server over stdio.
It lists, filters, samples, reads and searches archive entries without extracting files.
`--insight BUGREPORT.zip` generates seven evidence-linked triage artifacts.
Combine `--mcp --insight FILE_OR_DIRECTORY` to discover reports and read generated insights through MCP tools without writing output.

## Build and run

Use a Rust 2024 toolchain from the repository root:

```bash
cargo build --workspace --offline
cargo run -p bugreport-insight-cli --offline -- BUGREPORT.zip --mcp
```

Offline builds require the dependencies to be cached.
See [development](docs/guides/02-development.md#development-workflow) for normal setup and verification commands.
`--help` displays options; `--verbose` enables stderr diagnostics.
When a report exceeds a default limit, retry with `--force` to select finite raised resource limits; the process warns on stderr.
stdout carries MCP JSON-RPC frames and is intended for a connected MCP client.
Running with an archive but without `--mcp` reports that no mode was selected.

## Tools

| Tool | Purpose |
| --- | --- |
| `archive_info` | Archive identity, optional stored-file hash and marker metadata |
| `list_entries` | Entry listing with optional path prefix |
| `find_entries` | Literal or glob path filtering |
| `entry_info` | Entry metadata and sampled content detection |
| `find_by_type` | Filter entries by sampled content type |
| `read_entry` | Read a selected line or byte range by entry ID |
| `grep_archive` | Content search with client response/scan budgets, locator pages, continuation cursor and explicit coverage |
| `list_bugreports` | Combined mode: discover bugreport ZIPs within the selected file/directory scope |
| `list_insights` | Combined mode: confirm analysis and list the seven in-memory artifacts |
| `read_insight` | Combined mode: confirm and read a bounded UTF-8 artifact page |

Directory discovery visits only immediate regular ZIP children, excludes child symlinks and checks bugreport marker/report metadata.
It never searches parents or nested directories.

Follow [MCP usage](docs/guides/01-usage.md#mcp-usage) for call order and result interpretation.
[Current design decisions](./docs/decisions/index.md) explain the contracts; the [qualification assessment](docs/verification/assessment.md) records current verdicts and remaining evidence limits.

## Workspace and documentation

`crates/core` is the reusable ZIP library, `crates/mcp` implements handlers and transport, and `crates/cli` launches the server.
`crates/test-fixtures` supplies test-only synthetic ZIPs.
The [documentation index](./docs/index.md) routes requirements, architecture, detailed design, verification, traceability, decisions, proposals and historical records.
