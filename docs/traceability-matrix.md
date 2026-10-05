# Traceability Matrix

Every element in every specification document carries an ID, and every ID appears here beside the
requirements it serves. This is the join: [`requirements.md`](./requirements.md) states what the tool
must do, [`requirement-diagram.md`](./requirement-diagram.md) visualizes requirement-to-element, and
this file records which design elements are in scope for which requirements — so a reader can go from
an implementation decision up to the requirement that justifies it, and back down again.

## How to read the status column

| Mark | Meaning |
| --- | --- |
| ✅ | Committed. The requirement is binding now and the element exists or is being built. |
| 🚧 | Draft. Every requirement served here is prefixed with the word DRAFT followed by a hyphen. **Not traceability evidence** — see [`requirements.md`](./requirements.md#analysis-artifacts-draft-draft-req-art-02) for what the prefix means. Do not implement against a draft ID. |

A requirement with the word DRAFT followed by a hyphen may change or be withdrawn without that being a breaking change. A row with no
✅ anywhere in its requirement list is not evidence that anything is committed.

## Requirement Coverage

All 48 requirements, grouped by the document that realizes them. The `Elements` column links to the
rows further down this file.

| Requirements | Count | Elements | Status |
| --- | --- | --- | --- |
| `REQ-GEN-01`, `REQ-GEN-02` | 2 | `ARCH-ARCHIVE`, `ARCH-MCP-SRV` | ✅ |
| `REQ-LIB-01` | 1 | `ARCH-ARCHIVE`, `ARCH-MCP-SRV`, `ARCH-CLI-FLAGS` | ✅ |
| `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` | 3 | `ARCH-CLI-FLAGS`, `UC-01` | ✅ |
| `REQ-MCP-01`, `REQ-MCP-02`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11` | 10 | `ARCH-MCP-SRV`, `ARCH-MCP-TOOLS-ARC` | ✅ |
| `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07` | 7 | `ARCH-ARCHIVE`, `ARCH-MCP-TOOLS-ARC`, `UC-01`, `UC-02` | ✅ |
| `DRAFT-REQ-GEN-03` | 1 | `ARCH-ARCHIVE`, `ARCH-MCP-SRV`, `ARCH-CLI-FLAGS`, `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-MCP-RES`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-ARCH-ANR`, `DRAFT-ARCH-APPS`, `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MEMORY`, `DRAFT-ARCH-CPU`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-GEN-04` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-ART-01` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-02` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-03` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-04` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-05` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-06` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-07` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-ART-08` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-MCP-07` | 1 | `DRAFT-ARCH-MCP-RES` | 🚧 |
| `DRAFT-REQ-MCP-ARC-08` | 1 | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` | 🚧 |
| `DRAFT-REQ-MCP-ANR-01` | 1 | `DRAFT-ARCH-ANR`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-MCP-APP-01` | 1 | `DRAFT-ARCH-APPS`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-MCP-CRS-01` | 1 | `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-MCP-MEM-01` | 1 | `DRAFT-ARCH-MEMORY`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-MCP-CPU-01` | 1 | `DRAFT-ARCH-CPU`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-CLI-01` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-02` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-05` | 🚧 |
| `DRAFT-REQ-CLI-06` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-07` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-08` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-09` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-10` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |
| `DRAFT-REQ-CLI-11` | 1 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT`, `DRAFT-UC-03`, `DRAFT-UC-04`, `DRAFT-UC-05`, `DRAFT-UC-06` | 🚧 |

## Architecture Elements

IDs originate in [`architecture.md`](./architecture.md).

| ID | Element | Requirements | Status |
| --- | --- | --- | --- |
| <a id="arch-archive"></a>[`ARCH-ARCHIVE`](./architecture.md#arch-archive) | `crates/archive` — ZIP list / read / find / grep | `REQ-GEN-01`, `REQ-GEN-02`, `REQ-MCP-03`, `REQ-MCP-09`, `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07` | ✅ |
| <a id="arch-mcp-srv"></a>[`ARCH-MCP-SRV`](./architecture.md#arch-mcp-srv) | `crates/mcp` server core — lifecycle, transport, error mapping | `REQ-MCP-01`, `REQ-MCP-02`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11` | ✅ |
| <a id="arch-mcp-tools-arc"></a>[`ARCH-MCP-TOOLS-ARC`](./architecture.md#arch-mcp-tools-arc) | The seven archive tools | `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08` | ✅ |
| <a id="arch-cli-flags"></a>[`ARCH-CLI-FLAGS`](./architecture.md#arch-cli-flags) | `crates/cli` flag parsing — `--mcp`, `--help`, `--verbose` | `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` | ✅ |
| <a id="draft-arch-core-art"></a>[`DRAFT-ARCH-CORE-ART`](./architecture.md#draft-arch-core-art) | `crates/core` artifact model and rendering | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01`, `DRAFT-REQ-ART-02`, `DRAFT-REQ-ART-03`, `DRAFT-REQ-ART-04`, `DRAFT-REQ-ART-05`, `DRAFT-REQ-ART-06`, `DRAFT-REQ-ART-07`, `DRAFT-REQ-ART-08` | 🚧 |
| <a id="draft-arch-cli-cmd"></a>[`DRAFT-ARCH-CLI-CMD`](./architecture.md#draft-arch-cli-cmd) | `cli` analyze command and seven toggles | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-CLI-11` | 🚧 |
| <a id="draft-arch-mcp-res"></a>[`DRAFT-ARCH-MCP-RES`](./architecture.md#draft-arch-mcp-res) | MCP resource surface | `DRAFT-REQ-MCP-07` | 🚧 |
| <a id="draft-arch-mcp-tools-analysis"></a>[`DRAFT-ARCH-MCP-TOOLS-ANALYSIS`](./architecture.md#draft-arch-mcp-tools-analysis) | Five analyzer tools | `DRAFT-REQ-MCP-ANR-01`, `DRAFT-REQ-MCP-APP-01`, `DRAFT-REQ-MCP-CRS-01`, `DRAFT-REQ-MCP-MEM-01`, `DRAFT-REQ-MCP-CPU-01` | 🚧 |
| <a id="draft-arch-anr"></a>[`DRAFT-ARCH-ANR`](./architecture.md#draft-arch-anr) | `anr` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-MCP-ANR-01` | 🚧 |
| <a id="draft-arch-apps"></a>[`DRAFT-ARCH-APPS`](./architecture.md#draft-arch-apps) | `apps` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-MCP-APP-01` | 🚧 |
| <a id="draft-arch-stacktrace"></a>[`DRAFT-ARCH-STACKTRACE`](./architecture.md#draft-arch-stacktrace) | `stacktrace` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-MCP-CRS-01` | 🚧 |
| <a id="draft-arch-memory"></a>[`DRAFT-ARCH-MEMORY`](./architecture.md#draft-arch-memory) | `memory` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-MCP-MEM-01` | 🚧 |
| <a id="draft-arch-cpu"></a>[`DRAFT-ARCH-CPU`](./architecture.md#draft-arch-cpu) | `cpu` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-MCP-CPU-01` | 🚧 |
| <a id="draft-arch-support"></a>[`DRAFT-ARCH-SUPPORT`](./architecture.md#draft-arch-support) | `support` shared CLI/JSON plumbing | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-CLI-11` | 🚧 |

## Use Cases

IDs originate in [`use-cases.md`](./use-cases.md).

| ID | Use case | Requirements | Status |
| --- | --- | --- | --- |
| <a id="uc-01"></a>[`UC-01`](./use-cases.md#uc-01) | Engineer inspects a bugreport through the MCP server | `REQ-MCP-01`, `REQ-MCP-02`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11`, `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07`, `REQ-GEN-01`, `REQ-GEN-02`, `REQ-CLI-03` | ✅ |
| <a id="uc-02"></a>[`UC-02`](./use-cases.md#uc-02) | Agent searches without exhausting context | `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09` | ✅ |
| <a id="draft-uc-03"></a>[`DRAFT-UC-03`](./use-cases.md#draft-uc-03) | Engineer triages from a static report | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01`, `DRAFT-REQ-ART-02`, `DRAFT-REQ-ART-03`, `DRAFT-REQ-ART-04`, `DRAFT-REQ-ART-05`, `DRAFT-REQ-ART-06`, `DRAFT-REQ-ART-07`, `DRAFT-REQ-ART-08` | 🚧 |
| <a id="draft-uc-04"></a>[`DRAFT-UC-04`](./use-cases.md#draft-uc-04) | CI pipeline regression artifacts | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` | 🚧 |
| <a id="draft-uc-05"></a>[`DRAFT-UC-05`](./use-cases.md#draft-uc-05) | Third-party scanner via subcommand | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-CLI-11` | 🚧 |
| <a id="draft-uc-06"></a>[`DRAFT-UC-06`](./use-cases.md#draft-uc-06) | Research batch-processing of NDJSON | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` | 🚧 |

## Diagram Indices

IDs originate in [`docs/diagram-<ID>.md`](./docs/diagram-<ID>.md) (planned).

| ID | Diagram | Elements depicted | Status |
| --- | --- | --- | --- |
| <a id="dgm-archive"></a>[`DGM-ARCHIVE`](./docs/diagram-DGM-ARCHIVE.md) | `docs/diagram-DGM-ARCHIVE.md` | `ARCH-ARCHIVE` | ✅ |
| <a id="dgm-server"></a>[`DGM-SERVER`](./docs/diagram-DGM-SERVER.md) | `docs/diagram-DGM-SERVER.md` | `ARCH-MCP-SRV` | ✅ |
| <a id="dgm-res"></a>[`DGM-RES`](./docs/diagram-DGM-RES.md) | `docs/diagram-DGM-RES.md` | `DRAFT-ARCH-MCP-RES` | 🚧 |
| <a id="dgm-cli"></a>[`DGM-CLI`](./docs/diagram-DGM-CLI.md) | `docs/diagram-DGM-CLI.md` | `ARCH-CLI-FLAGS` | ✅ |
| <a id="dgm-analysis"></a>[`DGM-ANALYSIS`](./docs/diagram-DGM-ANALYSIS.md) | `docs/diagram-DGM-ANALYSIS.md` | `DRAFT-ARCH-ANR`, `DRAFT-ARCH-APPS`, `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MEMORY`, `DRAFT-ARCH-CPU` | 🚧 |
| <a id="dgm-artifacts"></a>[`DGM-ARTIFACTS`](./docs/diagram-DGM-ARTIFACTS.md) | `docs/diagram-DGM-ARTIFACTS.md` | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` | 🚧 |

## Unimplemented Specifications

`classes.md`, `sequences.md`, and `activities.md` were removed during the redesign. They described
the previous implementation's function names (`scan_anrs()`, `open_archive()`, `run_subcommand()`) and
crate layout (`subcommand-anr`, `subcommand-support`), none of which matches the committed design. They
carried no element IDs and no requirement mapping, which is part of why they drifted.

They will be redrawn against the committed requirements once `core` and `mcp` are implemented, and
their elements will get `CLS-*`, `SEQ-*`, and `ACT-*` IDs with rows here. Until then a requirement can
be committed with no implementing element below this line — `REQ-GEN-01` and `REQ-GEN-02` are
committed and have no class or sequence diagram yet.

## Convention

| Prefix | Document | Kind |
| --- | --- | --- |
| `REQ-*`, `DRAFT-REQ-*` | [`requirements.md`](./requirements.md) / [`requirement-diagram.md`](./requirement-diagram.md) | Requirement |
| `ARCH-*`, `DRAFT-ARCH-*` | [`architecture.md`](./architecture.md) | Crate or component |
| `UC-*`, `DRAFT-UC-*` | [`use-cases.md`](./use-cases.md) | Use case |
| `DGM-*` | `docs/diagram-<ID>.md` *(planned)* | Diagram index |
| `CLS-*`, `SEQ-*`, `ACT-*` | not yet redrawn | Class, sequence step, activity step |

Every ID is also a link target in both directions: an element row here links to the document that
defines the element, and the document links back here. IDs are stable — an ID is never reused for a
different thing, and a withdrawn element keeps its ID with its row marked withdrawn.