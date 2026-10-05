# bugreport-insight

A read-only [MCP](https://modelcontextprotocol.io) server over an Android
`adb bugreport` ZIP archive. One process opens one archive in place and serves
it to an AI agent over stdio. Nothing is extracted, written, or modified.

```console
bugreport-insight BUGREPORT.zip --mcp
```

The archive is opened **before** the transport starts, so a server that cannot
see its archive exits non-zero with a diagnostic on stderr rather than answering
as though the archive were empty. stdout carries JSON-RPC frames and nothing
else.

## The seven tools

Entries are addressed by `entry_id`, never by path — paths are output only.

| Tool | Answers | Cost |
| --- | --- | --- |
| `archive_info` | Am I looking at the archive I think I am? | index only; SHA-256 memoised |
| `list_entries` | What is here, with sizes? | index only |
| `find_entries` | Which paths match a pattern? | index only |
| `entry_info` | Everything about one entry | index + 8 KiB sample |
| `find_by_type` | Which paths are of this content type? | 8 KiB sample per entry |
| `read_entry` | A bounded slice of one entry | reads bytes |
| `grep_archive` | Content search, returning locators | scans content |

Every result carries `status`, `truncated`, `stopped_reason`, `omitted_count`,
and `notes`. `status` is one of `ok`, `empty`, `not_found`, `unreadable`: a
failure is never shaped like a successful empty result, so `empty` always means
the call worked and there was nothing there.

Bugreports often contain sensitive information. Keep archives and any generated
output local, and review them before sharing.

## Workspace

| Crate | Package | Role |
| --- | --- | --- |
| `crates/core` | `bugreport-insight-core` | ZIP index, bounded reader and search, content detection, archive identity |
| `crates/mcp` | `bugreport-insight-mcp` | The seven tools behind the `ToolHandler` seam, plus the `rmcp` adapter |
| `crates/cli` | `bugreport-insight` (binary) | argv parsing and mode selection |

`crates/core` never depends on a workspace sibling. `crates/mcp` is the only
crate that names an `rmcp` symbol, and it does so in one adapter module — which
is what lets the whole tool layer be unit-tested with no server running.

## Building

```console
cargo build --offline
cargo test  --offline
```

`cargo`, `rustc`, and `rust-analyzer` all live in `~/.cargo/bin`, already on
`PATH` on this machine. Verify with `~/.cargo/bin/rustc --print sysroot` →
`/Users/p2p/.rustup/toolchains/stable-aarch64-apple-darwin`. Call the binaries
by absolute path anyway when a command has to be unambiguous about which
toolchain it used.

Dependencies resolve from the ordinary cargo registry cache; there is no
vendored directory and no machine-local path in the build. See
[`architecture.md` §8](./docs/architecture.md#8-dependency-availability--verified-2026-10-05)
for the dependency table and for what an offline machine needs instead.

## Documentation

[`docs/index.md`](./docs/index.md) is the entry point for the whole doc set: it
routes between documents, says which one is the source of truth for what, and
names a starting point per kind of question.

| Document | Source of truth for |
| --- | --- |
| [`docs/requirements.md`](./docs/requirements.md) | What the tool must do. 23 requirements, all implemented. |
| [`docs/architecture.md`](./docs/architecture.md) | What it is made of — crates, elements, IDs, status, structural constraints. |
| [`docs/diagrams/index.md`](./docs/diagrams/index.md) | Which element satisfies which requirement, with risk and verification method. |
| [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) | The join: requirements ↔ elements ↔ use cases. |
| [`docs/use-cases.md`](./docs/use-cases.md) | Use cases and their actors. |
| [`docs/draft/`](./docs/draft/index.md) | Deferred work: the `DRAFT-` requirements and the artifact schemas. |
| [`docs/design/`](./docs/design/index.md) | Decision records: what was decided, what was rejected, what is still open. |

## Design decisions

[`docs/design/`](./docs/design/index.md) holds the accepted design for the
redesign: the rule-1 conflict resolution, the final ID list, the core archive
API, the offline dependency rulings, the build plan, and the open items. It is
a **historical record** of decisions made before implementation, not a plan for
work still to do.

Start at [`design/index.md`](./docs/design/index.md). Engineers should read
[`design/03-id-scheme.md`](./docs/design/03-id-scheme.md),
[`design/04a-core-archive-api.md`](./docs/design/04a-core-archive-api.md), and
[`design/04b-core-archive-types.md`](./docs/design/04b-core-archive-types.md)
first.

> The design set's crate naming (`crates/archive`) is **superseded**: the crate is
> `crates/core`, package `bugreport-insight-core`. See
> [`architecture.md` §3.1](./docs/architecture.md#31-committed). `docs/` wins for scope
> and `docs/design/` for reasoning.
