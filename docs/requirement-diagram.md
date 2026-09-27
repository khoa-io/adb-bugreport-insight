# Requirement Diagram

<!-- These diagrams trace each requirement in [requirements.md](./requirements.md) to the system element that satisfies it. Within each diagram the layout is top-down: elements sit at the top, the requirements they satisfy below, and the `risk` / `verifymethod` attributes of each requirement belong to it. -->

<!-- The set is split into four diagrams by concern. A single diagram carrying all 33 nodes is unreadable at
normal zoom — a 17-node version already rendered 3250 px wide — so partitioning by concern keeps each
canvas narrow while preserving the full 24-requirement / 24-relationship set. The Analysis Artifacts
canvas is now the widest, at nine requirement nodes; if it becomes hard to read, split the six
per-artifact requirements into their own concern diagram. -->

## Core Library

```mermaid
requirementDiagram

    direction TB

    element zip_reader {
        type: "module"
        docRef: "Read-only, size-limited ZIP access"
    }

    element scanner_framework {
        type: "module"
        docRef: "Evidence scanners over logcat/dumpstate"
    }

    element core_library {
        type: "module"
        docRef: "Reusable core analysis library"
    }

    requirement read_only_zip {
        id: "REQ-GEN-01"
        text: "Open bugreport ZIP archives without extracting files to disk"
        risk: high
        verifymethod: test
    }

    performanceRequirement safety_limits {
        id: "REQ-GEN-02"
        text: "Enforce size and entry count limits to prevent resource exhaustion"
        risk: high
        verifymethod: test
    }

    requirement evidence_extraction {
        id: "REQ-GEN-03"
        text: "Scan logcat/dumpstate for ANR, crashes, memory pressure, and CPU contention"
        risk: high
        verifymethod: test
    }

    designConstraint library_distribution {
        id: "REQ-LIB-01"
        text: "Distribute the core analysis engine as a library so other Rust crates can depend on it"
        risk: medium
        verifymethod: inspection
    }

    zip_reader - satisfies -> read_only_zip
    zip_reader - satisfies -> safety_limits
    scanner_framework - satisfies -> evidence_extraction
    core_library - satisfies -> library_distribution
```

## Command-Line Interface

```mermaid
requirementDiagram

    direction TB

    element cli {
        type: "interface"
        docRef: "Command-line interface entry point"
    }

    element mcp_server {
        type: "module"
        docRef: "Live-inspection MCP server"
    }

    interfaceRequirement analyze_command {
        id: "REQ-CLI-01"
        text: "bugreport-insight BUGREPORT generates static analysis artifacts"
        risk: high
        verifymethod: test
    }

    interfaceRequirement mcp_mode {
        id: "REQ-CLI-03"
        text: "--mcp starts an MCP server for live inspection of the bugreport"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement help_flag {
        id: "REQ-CLI-04"
        text: "--help displays usage information and available options"
        risk: low
        verifymethod: inspection
    }

    interfaceRequirement verbose_flag {
        id: "REQ-CLI-05"
        text: "--verbose enables verbose logging for debugging"
        risk: low
        verifymethod: inspection
    }

    cli - satisfies -> analyze_command
    cli - satisfies -> help_flag
    cli - satisfies -> verbose_flag
    mcp_server - satisfies -> mcp_mode
```

## Analysis Toggles

```mermaid
requirementDiagram

    direction TB

    element cli {
        type: "interface"
        docRef: "Command-line interface entry point"
    }

    interfaceRequirement sys_toggle {
        id: "REQ-CLI-02"
        text: "--sys includes system analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement apps_toggle {
        id: "REQ-CLI-06"
        text: "--apps includes apps analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement anr_toggle {
        id: "REQ-CLI-07"
        text: "--anr includes ANR analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement crash_toggle {
        id: "REQ-CLI-08"
        text: "--crash includes crash analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement memory_toggle {
        id: "REQ-CLI-09"
        text: "--memory includes memory analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement cpu_toggle {
        id: "REQ-CLI-10"
        text: "--cpu includes CPU analysis in the generated artifacts, default true"
        risk: medium
        verifymethod: test
    }

    interfaceRequirement no_prefix_toggle {
        id: "REQ-CLI-11"
        text: "--no-* disables a specific analysis type, e.g. --no-anr skips ANR analysis"
        risk: medium
        verifymethod: test
    }

    cli - satisfies -> sys_toggle
    cli - satisfies -> apps_toggle
    cli - satisfies -> anr_toggle
    cli - satisfies -> crash_toggle
    cli - satisfies -> memory_toggle
    cli - satisfies -> cpu_toggle
    cli - satisfies -> no_prefix_toggle
```

## Analysis Artifacts

```mermaid
requirementDiagram

    direction TB

    element artifact_writer {
        type: "module"
        docRef: "JSON/NDJSON/Markdown artifact generation"
    }

    requirement artifact_generation {
        id: "REQ-GEN-04"
        text: "Produce JSON/NDJSON findings, a Markdown README, a manifest, and session segmentation"
        risk: medium
        verifymethod: test
    }

    requirement readme_artifact {
        id: "REQ-ART-01"
        text: "README.md summarizes findings and further investigation steps"
        risk: medium
        verifymethod: inspection
    }

    requirement json_artifacts {
        id: "REQ-ART-02"
        text: "Generate system.json, apps.json, anr.json, crashes.json, memory.json, and cpu.json"
        risk: medium
        verifymethod: test
    }

    requirement system_artifact {
        id: "REQ-ART-03"
        text: "system.json establishes the canonical capture context, section status, and build-type gating every other artifact references"
        risk: high
        verifymethod: test
    }

    requirement apps_artifact {
        id: "REQ-ART-04"
        text: "apps.json separates installed packages from running processes, keyed on process_key rather than pid, with per-channel coverage"
        risk: medium
        verifymethod: test
    }

    requirement anr_artifact {
        id: "REQ-ART-05"
        text: "anr.json keys ANRs by incident, never promotes retained trace files to incidents, and labels every evidence item with its age"
        risk: high
        verifymethod: test
    }

    requirement crashes_artifact {
        id: "REQ-ART-06"
        text: "crashes.json names one authoritative channel for the headline count, keeps Java and native evidence separate, and reports the de-duplication split"
        risk: high
        verifymethod: test
    }

    requirement memory_artifact {
        id: "REQ-ART-07"
        text: "memory.json records the collector's own page-scanning cost and attributes swap growth to it rather than to the device"
        risk: high
        verifymethod: test
    }

    requirement cpu_artifact {
        id: "REQ-ART-08"
        text: "cpu.json separates windowed utilization from cumulative core counters and from contention, and carries each measurement's window and kind"
        risk: medium
        verifymethod: test
    }

    artifact_writer - satisfies -> artifact_generation
    artifact_writer - satisfies -> readme_artifact
    artifact_writer - satisfies -> json_artifacts
    artifact_writer - satisfies -> system_artifact
    artifact_writer - satisfies -> apps_artifact
    artifact_writer - satisfies -> anr_artifact
    artifact_writer - satisfies -> crashes_artifact
    artifact_writer - satisfies -> memory_artifact
    artifact_writer - satisfies -> cpu_artifact
```

## Requirement Index

Each ID below is an anchor target: the ID printed next to every requirement in
[requirements.md](./requirements.md) links back to its row here, and the row's own ID links forward to
the requirement text it visualizes. The `Diagram` column links to the section above that renders it.

| ID | Requirement | Satisfied by | Diagram | Risk | Verification |
| --- | --- | --- | --- | --- | --- |
| <a id="req-gen-01"></a>[`REQ-GEN-01`](./requirements.md#req-gen-01) | Read-only ZIP access | `zip_reader` | [Core library](#core-library) | high | test |
| <a id="req-gen-02"></a>[`REQ-GEN-02`](./requirements.md#req-gen-02) | Safety limits | `zip_reader` | [Core library](#core-library) | high | test |
| <a id="req-gen-03"></a>[`REQ-GEN-03`](./requirements.md#req-gen-03) | Evidence extraction | `scanner_framework` | [Core library](#core-library) | high | test |
| <a id="req-lib-01"></a>[`REQ-LIB-01`](./requirements.md#req-lib-01) | Library distribution | `core_library` | [Core library](#core-library) | medium | inspection |
| <a id="req-cli-01"></a>[`REQ-CLI-01`](./requirements.md#req-cli-01) | `BUGREPORT` analysis command | `cli` | [CLI](#command-line-interface) | high | test |
| <a id="req-cli-03"></a>[`REQ-CLI-03`](./requirements.md#req-cli-03) | `--mcp` server mode | `mcp_server` | [CLI](#command-line-interface) | medium | test |
| <a id="req-cli-04"></a>[`REQ-CLI-04`](./requirements.md#req-cli-04) | `--help` usage output | `cli` | [CLI](#command-line-interface) | low | inspection |
| <a id="req-cli-05"></a>[`REQ-CLI-05`](./requirements.md#req-cli-05) | `--verbose` logging | `cli` | [CLI](#command-line-interface) | low | inspection |
| <a id="req-cli-02"></a>[`REQ-CLI-02`](./requirements.md#req-cli-02) | `--sys` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-06"></a>[`REQ-CLI-06`](./requirements.md#req-cli-06) | `--apps` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-07"></a>[`REQ-CLI-07`](./requirements.md#req-cli-07) | `--anr` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-08"></a>[`REQ-CLI-08`](./requirements.md#req-cli-08) | `--crash` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-09"></a>[`REQ-CLI-09`](./requirements.md#req-cli-09) | `--memory` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-10"></a>[`REQ-CLI-10`](./requirements.md#req-cli-10) | `--cpu` toggle | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-cli-11"></a>[`REQ-CLI-11`](./requirements.md#req-cli-11) | `--no-*` disable form | `cli` | [Toggles](#analysis-toggles) | medium | test |
| <a id="req-gen-04"></a>[`REQ-GEN-04`](./requirements.md#req-gen-04) | Artifact generation | `artifact_writer` | [Artifacts](#analysis-artifacts) | medium | test |
| <a id="req-art-01"></a>[`REQ-ART-01`](./requirements.md#req-art-01) | `README.md` artifact | `artifact_writer` | [Artifacts](#analysis-artifacts) | medium | inspection |
| <a id="req-art-02"></a>[`REQ-ART-02`](./requirements.md#req-art-02) | Per-analysis JSON artifacts | `artifact_writer` | [Artifacts](#analysis-artifacts) | medium | test |
| <a id="req-art-03"></a>[`REQ-ART-03`](./requirements.md#req-art-03) | `system.json` capture context, section status, build-type gating | `artifact_writer` | [Artifacts](#analysis-artifacts) | high | test |
| <a id="req-art-04"></a>[`REQ-ART-04`](./requirements.md#req-art-04) | `apps.json` packages vs. processes, `process_key` identity | `artifact_writer` | [Artifacts](#analysis-artifacts) | medium | test |
| <a id="req-art-05"></a>[`REQ-ART-05`](./requirements.md#req-art-05) | `anr.json` incident keying and age labelling | `artifact_writer` | [Artifacts](#analysis-artifacts) | high | test |
| <a id="req-art-06"></a>[`REQ-ART-06`](./requirements.md#req-art-06) | `crashes.json` authoritative channel and de-duplication split | `artifact_writer` | [Artifacts](#analysis-artifacts) | high | test |
| <a id="req-art-07"></a>[`REQ-ART-07`](./requirements.md#req-art-07) | `memory.json` collector-cost attribution | `artifact_writer` | [Artifacts](#analysis-artifacts) | high | test |
| <a id="req-art-08"></a>[`REQ-ART-08`](./requirements.md#req-art-08) | `cpu.json` utilization vs. core counters vs. contention | `artifact_writer` | [Artifacts](#analysis-artifacts) | medium | test |

## Notation

- `requirement` — functional behavior the tool must exhibit.
- `performanceRequirement` — a bound on resource usage, here the archive size and entry count limits.
- `designConstraint` — a rule that constrains how the system is built rather than what it does.
- `interfaceRequirement` — part of the user-facing command-line contract.
- `satisfies` — the element implements the requirement.
