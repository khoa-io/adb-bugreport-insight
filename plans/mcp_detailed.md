# `list_archive` MCP tool — complete implementation (ready to apply)

> **Status:** This file is a hand-off artifact. The repo path is read-only in the current
> sandbox, so these changes could not be applied or compiled by the agent. Apply them with
> write access enabled, then run `cargo build` + `cargo test` (and adjust the `rmcp` import
> surface if the resolved `rmcp` version's API differs — see the note at the bottom).

Feature: list the entries (files) inside a `bugreport` ZIP archive **without extracting**, exposed
both as a reusable core function and as the first MCP tool (`list_archive`).

---

## 1. `crates/core/src/model.rs` — add `ArchiveEntry`

Append after the `AppInfo` struct (before `Manifest`):

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

`Serialize` is already imported at the top of the file; no new imports needed.

---

## 2. `crates/core/src/archive.rs` — add `list_archive`

Add `CompressionMethod` to the `zip` import (top of file):

```rust
use zip::{CompressionMethod, ZipArchive};
```

Append these two items at the end of the file:

```rust
/// Maps a ZIP compression method to a short, stable string (`stored`, `deflated`, ...).
fn compression_name(method: CompressionMethod) -> String {
    if method == CompressionMethod::Stored {
        "stored".to_owned()
    } else if method == CompressionMethod::Deflated {
        "deflated".to_owned()
    } else {
        format!("{method:?}")
    }
}

/// Lists every entry in the bugreport ZIP by reading only the central directory — no payload is
/// extracted or decompressed. Entry names are treated as untrusted and are validated by
/// [`validate_archive`] before anything is returned.
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

---

## 3. `crates/core/src/lib.rs` — re-export

Change the existing re-export line from:

```rust
pub use analyze::{open_report, render_insight, unclaimed_findings};
```

to:

```rust
pub use analyze::{open_report, render_insight, unclaimed_findings};
pub use archive::list_archive;
```

---

## 4. `crates/core/src/archive.rs` (optional) — unit test

Append under an existing `#[cfg(test)]` block, or add a new one at the end:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_sample_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("main_entry.txt", options).unwrap();
        writer.write_all(b"bugreport main text").unwrap();
        writer.add_directory("subdir/", options).unwrap();
        writer.start_file("subdir/nested.txt", options).unwrap();
        writer.write_all(b"nested").unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn lists_entries_without_extracting() {
        let dir = std::env::temp_dir();
        let path = dir.join("bugreport-insight-list-test.zip");
        write_sample_zip(&path);
        let entries = list_archive(&path).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"main_entry.txt"));
        assert!(names.contains(&"subdir/"));
        assert!(names.contains(&"subdir/nested.txt"));
        let nested = entries.iter().find(|e| e.name == "subdir/nested.txt").unwrap();
        assert!(!nested.is_dir);
        assert_eq!(nested.size, 6);
        std::fs::remove_file(&path).ok();
    }
}
```

---

## 5. New crate `crates/mcp` — `Cargo.toml`

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

---

## 6. New crate `crates/mcp` — `src/lib.rs`

```rust
//! `bugreport-insight-mcp` — a local Model Context Protocol (MCP) server over stdio that
//! exposes tools for inspecting Android bugreport ZIP archives without extracting them.

use bugreport_insight_core::{archive::list_archive, model::ArchiveEntry};
use rmcp::{
    model::{CallToolResult, Parameters, TextContent},
    server::{ServerHandler, ToolRouter},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

/// Arguments for the `list_archive` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListArchiveParams {
    /// Path to the bugreport ZIP archive to inspect.
    pub bugreport_zip: String,
}

/// The MCP server handler. `Clone` so the transport can hold it cheaply.
#[derive(Clone)]
pub struct BugreportInsightMcp {
    tool_router: ToolRouter,
}

#[tool_router]
impl BugreportInsightMcp {
    /// Lists the entries inside a bugreport ZIP archive without extracting or decompressing
    /// any payload. Returns a JSON array of `{ name, size, compressed_size, is_dir, compression }`.
    #[tool(
        name = "list_archive",
        description = "List the entries (files and directories) inside a bugreport ZIP archive without extracting them."
    )]
    async fn list_archive(params: Parameters<ListArchiveParams>) -> Result<CallToolResult, String> {
        let path = PathBuf::from(&params.inner().bugreport_zip);
        let entries: Vec<ArchiveEntry> = list_archive(&path).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        Ok(CallToolResult {
            content: vec![TextContent::text(json)],
            is_error: Some(false),
        })
    }

    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl ServerHandler for BugreportInsightMcp {}
```

---

## 7. New crate `crates/mcp` — `src/main.rs`

```rust
use bugreport_insight_mcp::BugreportInsightMcp;
use rmcp::{transport::StdioTransport, Server};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let transport = StdioTransport::new();
    let server = match Server::builder()
        .with_handler(BugreportInsightMcp::new())
        .build(transport)
    {
        Ok(server) => server,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    match server.run(rmcp::signal::ctrl_c()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
```

---

## 8. Root `Cargo.toml` — register the crate

Add `"crates/mcp",` to the `members` list (after `"crates/subcommand-cpu",`), and add this
line to `[workspace.dependencies]` (after `bugreport-insight-cpu = ...`):

```toml
bugreport-insight-mcp = { path = "crates/mcp" }
```

---

## 9. `crates/cli/src/plugin.rs` — register the `mcp` verb

Change:

```rust
pub const BUILTIN_SUBCOMMANDS: &[&str] = &["anr", "apps", "stacktrace", "memory", "cpu"];
```

to:

```rust
pub const BUILTIN_SUBCOMMANDS: &[&str] = &["anr", "apps", "stacktrace", "memory", "cpu", "mcp"];
```

(The `cli` crate only execs the sibling binary; it needs no new dependency.)

---

## 10. Docs (optional)

- `README.md`: add a line under the subcommand list for `mcp` and `list_archive`.
- `docs/project-summary.md`: mark the `mcp` subcommand + `list_archive` tool as implemented in
  the roadmap/§11 table and add `list_archive` to the §4.2 core-module map.

---

## Verification steps (run after applying)

```bash
cargo build --workspace
cargo test -p bugreport-insight-core list_archive
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}\n' \
  | ./target/debug/bugreport-insight-mcp
```

---

## ⚠️ `rmcp` API note

The MCP crate follows the `rmcp` 0.8 API from the project's MCP guidance (`#[tool_router]`,
`#[tool_handler]`, `Server::builder()`, `StdioTransport`, `ToolRouter`). This was **not
compiled**. If `cargo build` reports different `rmcp` type/import names after resolution,
the likely fixes are:

- Replace `use rmcp::server::{ServerHandler, ToolRouter}` with the resolved path
  (e.g. `rmcp::handler::server::ServerHandler`).
- Use `rmcp::ServiceExt` + `rmcp::transport::stdio::stdio()` (`service.serve(stdio())`) if the
  resolved version dropped `Server::builder()`.
- Adjust `CallToolResult`/`TextContent` construction if the model API differs.

The core `list_archive` change (sections 1–4) is independent of `rmcp` and is expected to
compile as-is against the already-locked `zip` 2.4.2.
