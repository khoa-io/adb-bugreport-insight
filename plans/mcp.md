# Plan: `list_archive` MCP tool for bugreport-insight

> First feature for the `bugreport-insight mcp` server: list the contents of a bugreport ZIP
> archive **without extracting** any entry. Delivered in two layers — a reusable, pure-Rust core
> function, and an MCP tool that exposes it over stdio.

## Context

- A `bugreport` is a ZIP archive. Before any analysis, users want to see what's inside it.
- `bugreport-insight-core` already provides read-only ZIP access
  (`open_archive` / `validate_archive` in `crates/core/src/archive.rs`) using
  `zip::ZipArchive<BufReader<File>>` (zip 2.4.2, deflate-only). Entries are never extracted to
  disk.
- The MCP server is currently a roadmap item (`PLAN.md` §MCP server); there are no `rmcp`,
  `tokio`, or `async-trait` dependencies yet. This plan adds the first tool.
- Safety limits already enforced in `validate_archive`: `MAX_ENTRIES` = 10 000,
  `MAX_ENTRY_BYTES` = 256 MiB, `MAX_TOTAL_BYTES` = 1 GiB. Listing reuses these, so untrusted
  entry names/sizes are rejected before any payload is read.

## Design

### Layer 1 — reusable core listing (no new dependencies)

Add `ArchiveEntry` (metadata-only, `Serialize`) to `crates/core/src/model.rs`, and a
`list_archive(path) -> Result<Vec<ArchiveEntry>>` function to `crates/core/src/archive.rs`
that walks the ZIP central directory (no decompression) and returns name, uncompressed size,
compressed size, is-dir flag, and compression method.

### Layer 2 — MCP server (`crates/mcp`)

A new thin crate following the project's git/cargo-style plugin pattern:
`bugreport-insight mcp` execs the sibling `bugreport-insight-mcp` binary, which starts an
`rmcp` stdio MCP server exposing the `list_archive` tool. The tool returns the entry list as
pretty JSON.

---

## Detailed steps

### 1. `crates/core/src/model.rs` — add `ArchiveEntry`

Append (before the `Manifest` struct; `Serialize` is already imported):

```rust
/// A single entry in a bugreport ZIP archive, described by central-directory metadata only
/// (no payload is read). Emitted by `core::archive::list_archive`.
#[derive(Clone, Serialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
    pub compression: String,
}
```

### 2. `crates/core/src/archive.rs` — add `list_archive`

- Extend the `zip` import to `use zip::{CompressionMethod, ZipArchive};`.
- Add a `compression_name` helper mapping `CompressionMethod::Stored`/`::Deflated` to stable
  strings (`"stored"`, `"deflated"`) with a `format!("{method:?}")` fallback. Only `Stored` and
  `Deflated` consts are referenced because the workspace enables only the `deflate` zip feature
  (other method consts may be feature-gated).
- Add `list_archive`:

```rust
pub fn list_archive(path: &Path) -> Result<Vec<crate::model::ArchiveEntry>, Error> {
    let mut archive = open_archive(path)?;
    validate_archive(&mut archive)?;
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        entries.push(crate::model::ArchiveEntry {
            name: entry.name().to_owned(),
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            is_dir: entry.is_dir(),
            compression: compression_name(entry.compression()),
        });
    }
    Ok(entries)
}
```

### 3. `crates/core/src/lib.rs` — re-export

Add `pub use archive::list_archive;` alongside the existing `analyze` re-exports.

### 4. `crates/core/src/archive.rs` — unit test

Add a `#[cfg(test)]` test that writes a small in-memory ZIP to a temp file (one file, one
directory entry, one nested file), calls `list_archive`, and asserts names + `is_dir` + `size`.

### 5. New crate `crates/mcp` — `Cargo.toml`

```toml
[package]
name = "bugreport-insight-mcp"
version.workspace = true
edition.workspace = true
description = "bugreport-insight subcommand: Model Context Protocol (MCP) server over stdio"

[[bin]]
name = "bugreport-insight-mcp"
path = "src/main.rs"

[dependencies]
bugreport-insight-core.workspace = true
rmcp = "0.8"
schemars = "0.8"
serde.workspace = true
serde_json.workspace = true
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### 6. New crate `crates/mcp` — `src/lib.rs`

Handler struct `BugreportInsightMcp` with a `#[tool_router]` impl exposing one tool
`list_archive` (params `{ bugreport_zip: String }`), returning a `CallToolResult` whose single
`TextContent` is the pretty-printed JSON array of `ArchiveEntry`. A `#[tool_handler]` impl of
`rmcp::server::ServerHandler` wires the router. (`schemars::JsonSchema` derive on the params
struct; `serde::Deserialize`.)

### 7. New crate `crates/mcp` — `src/main.rs`

`#[tokio::main]` that builds `rmcp::transport::StdioTransport`, constructs
`Server::builder().with_handler(...).build(transport)`, and runs it until Ctrl-C, mapping errors
to `ExitCode`.

### 8. Root `Cargo.toml` — register the crate

- Add `"crates/mcp",` to `members`.
- Add `bugreport-insight-mcp = { path = "crates/mcp" }` to `[workspace.dependencies]`.

### 9. `crates/cli/src/plugin.rs` — register the `mcp` verb

Add `"mcp"` to `BUILTIN_SUBCOMMANDS` (dispatch is exec-only; no new `cli` dependency needed).

### 10. Docs

- `README.md`: list `mcp` under subcommands and describe `list_archive`.
- `docs/project-summary.md`: mark `mcp`/`list_archive` implemented in the roadmap and add
  `list_archive` to the core-module map.

---

## Verification

```bash
cargo build --workspace
cargo test -p bugreport-insight-core list_archive
cargo clippy --workspace --all-targets
cargo fmt --check
```

Smoke-test the MCP server over stdio:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}\n' \
  | ./target/debug/bugreport-insight-mcp
```

## Risks / notes

- **`rmcp` API drift:** the crate is written against `rmcp` 0.8 (`#[tool_router]`,
  `#[tool_handler]`, `Server::builder()`, `StdioTransport`, `ToolRouter`). If the resolved
  version differs, imports may need adjustment (e.g. `rmcp::ServiceExt` +
  `transport::stdio::stdio()`). Pin/verify via `cargo build`.
- **Zip feature gating:** `CompressionMethod` consts for bzip2/zstd may be unavailable under
  `default-features = false, features = ["deflate"]`; the helper only references `Stored` and
  `Deflated`.
- **No extraction:** listing reads central-directory metadata only; consistent with the
  project's no-extraction guarantee.

## Out of scope (future tools)

`analyze_bugreport`, `analyze_dumpstate`, `read_insight` — to be added after `list_archive`
establishes the MCP plumbing.
