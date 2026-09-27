# Architecture Diagram

## Architecture Overview

`bugreport-insight` is a read‑only, single‑binary‑plus‑plugins architecture. The CLI binary links a shared core library (read‑only ZIP access, scanners, rendering) and a set of scanner libraries, and dispatches focused verbs to sibling binaries. It has no network access, no database, and no persistent server — it is a local analysis utility that turns one ZIP archive into a folder of findings.

```mermaid
flowchart LR
    subgraph external["Input / environment"]
        DEV[Android device] -->|"adb bugreport"| ZIP[("BUGREPORT.zip")]
    end
    subgraph tool["bugreport-insight"]
        CLI["bugreport-insight<br/>cli crate"]
        CORE["bugreport-insight-core<br/>read‑only ZIP + scanners + render"]
        PLUGINS["plugin subcommands<br/>anr / apps / stacktrace / memory / cpu"]
    end
    OUT["(.BUGREPORT/insight/<br/>README.md · manifest.json · findings.json)" ]
    USER["Engineer / automation"]

    ZIP -->|"reads in place"| CLI
    CLI -->|"calls"| CORE
    CLI -.->|"dispatches verb"| PLUGINS
    PLUGINS -.->|"link as libraries"| CORE
    CORE -->|"writes artifacts"| OUT
    OUT -->|"consumes"| USER
```
