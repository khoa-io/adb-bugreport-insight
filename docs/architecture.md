# Architecture

## Architecture Overview

`bugreport-insight` is a read-only analysis tool organized as a Cargo workspace around a single shared foundation crate (`core`), thin analyzer crates, and two independent frontends: a CLI for engineers and an MCP server for AI agents. It has no network access, no database, and no persistent server — it turns one ZIP archive into a folder of findings.

```mermaid
flowchart TD
    subgraph input["Input / environment"]
        DEV[Android device] -->|"adb bugreport"| ZIP[("BUGREPORT.zip")]
    end

    subgraph core["core — bugreport-insight-core"]
        ARCHIVE["ZIP + TAR access<br/>list · read · find · grep"]
        ARTIFACT["analysis artifact<br/>model + render"]
        ARCHIVE --> ARTIFACT
    end

    subgraph analyzers["analyzer crates"]
        ANR[anr]
        APPS[apps]
        STACK[stacktrace]
        MEM[memory]
        CPU[cpu]
    end

    subgraph frontends["frontends"]
        CLI["cli<br/>bugreport-insight binary"]
        MCP["mcp<br/>MCP server"]
    end

    OUT["(.BUGREPORT/insight/<br/>README.md · manifest.json · findings.json)"]
    ENGINEER["Engineer"]
    AGENT["AI agent"]

    ZIP -->|"reads in place"| ARCHIVE
    analyzers -->|"depend on"| core
    CLI -->|"depends on"| core
    CLI -->|"composes"| analyzers
    MCP -->|"depends on"| core
    MCP -->|"composes"| analyzers
    ARTIFACT -->|"writes artifacts"| OUT
    CLI -->|"serves"| ENGINEER
    MCP -->|"serves"| AGENT
```

- **`core`** owns all archive access — ZIP and TAR listing, reading, finding, and grepping — plus the analysis artifact model and rendering. Every other crate depends on `core` to read bugreports and emit artifacts. `core` itself depends on `ripgrep`, `zip`, and `tar`.
- **Analyzer crates** (`anr`, `apps`, `stacktrace`, `memory`, `cpu`) each own one kind of scanner and depend on `core`.
- **`cli`** is the `bugreport-insight` binary for engineers. It depends on `clap` for argument parsing, plus `core` and the analyzer crates.
- **`mcp`** is the MCP server for AI agents. It depends on `rmcp`, plus `core` and the analyzer crates.

## Crate Responsibilities and Dependencies

| Crate | Role | Depends on |
| --- | --- | --- |
| `core` | Read-only ZIP + TAR access (list, read, find, grep); analysis artifact model and rendering | `ripgrep`, `zip`, `tar` |
| `anr` | Application Not Responding scanner | `core` |
| `apps` | Installed/running app inventory (PID, package, data/install paths) | `core` |
| `stacktrace` | Native crashes/tombstones and Java stack traces | `core` |
| `memory` | Memory pressure and low-memory-killer scanner | `core` |
| `cpu` | CPU/I-O pressure and lock-contention scanner | `core` |
| `support` | Shared clap args and JSON-emit helper for analyzer binaries | `clap`, `serde`, `serde_json` |
| `cli` | The `bugreport-insight` CLI binary | `clap`, `core`, analyzer crates |
| `mcp` | MCP server for AI-agent-driven inspection | `rmcp`, `core`, analyzer crates |

## Dependency Rules

- `core` is the sole foundation: it depends only on external crates (`ripgrep`, `zip`, `tar`) and never on any sibling workspace crate.
- Analyzer crates and frontends (`cli`, `mcp`) depend on `core`; `core` never depends back on them, so there is no dependency cycle.
- The two frontends are peers: `cli` serves engineers over a command line, `mcp` serves AI agents over the Model Context Protocol. Neither depends on the other.
