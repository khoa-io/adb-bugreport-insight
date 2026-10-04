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
| 🚧 | Draft. Every requirement served here is `DRAFT-`-prefixed. **Not traceability evidence** — see [`requirements.md`](./requirements.md#analysis-artifacts-draft-draft-req-art-02) for what the prefix means. Do not implement against a draft ID. |
| mixed | The element serves both committed and draft requirements. The mixed rows are listed below. |

A `DRAFT-` requirement may change or be withdrawn without that being a breaking change. A row with no
✅ anywhere in its requirement list is not evidence that anything is committed.

## Requirement Coverage

All 46 requirements, grouped by the document that realizes them. The `Elements` column links to the
rows further down this file.

| Requirements | Count | Elements | Status |
| --- | --- | --- | --- |
| `REQ-GEN-01`, `REQ-GEN-02` | 2 | `ARCH-CORE-ARC`, `ARCH-MCP` | ✅ |
| `REQ-LIB-01` | 1 | `ARCH-CORE`, `ARCH-MCP`, `ARCH-CLI` | ✅ |
| `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` | 3 | `ARCH-CLI`, `UC-01` | ✅ |
| `REQ-MCP-01` … `REQ-MCP-06`, `REQ-MCP-08` … `REQ-MCP-10` | 9 | `ARCH-MCP`, `ARCH-MCP-TOOLS` | ✅ |
| `REQ-MCP-ARC-01` … `REQ-MCP-ARC-07` | 7 | `ARCH-CORE-ARC`, `ARCH-MCP-TOOLS`, `UC-01`, `UC-02` | ✅ |
| `DRAFT-REQ-GEN-03` | 1 | `ARCH-ANR`, `ARCH-APPS`, `ARCH-STACKTRACE`, `ARCH-MEMORY`, `ARCH-CPU` | 🚧 |
| `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` | 9 | `ARCH-CORE-ART`, `ARCH-SUPPORT` | 🚧 |
| `DRAFT-REQ-MCP-07` | 1 | `ARCH-MCP`, `ARCH-CORE-ART` | 🚧 |
| `DRAFT-REQ-MCP-ANR-01`, `APP-01`, `CRS-01`, `MEM-01`, `CPU-01` | 5 | `ARCH-MCP-TOOLS`, `ARCH-ANR` … `ARCH-CPU` | 🚧 |
| `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11` | 8 | `ARCH-CLI`, `ARCH-ANR` … `ARCH-CPU` | 🚧 |

**22 committed, 24 draft.** The three mixed rows (`ARCH-CORE`, `ARCH-CLI`, `ARCH-MCP-TOOLS`) are
called out below.

## Architecture Elements

IDs originate in [`architecture.md`](./architecture.md).

| ID | Element | Requirements | Status |
| --- | --- | --- | --- |
| <a id="arch-core-arc"></a>[`ARCH-CORE-ARC`](./architecture.md#arch-core) | `core` archive access — ZIP/TAR list, read, find, grep | `REQ-GEN-01`, `REQ-GEN-02`, `REQ-MCP-ARC-01` … `07`, `REQ-MCP-03`, `REQ-MCP-09` | ✅ |
| <a id="arch-core-art"></a>[`ARCH-CORE-ART`](./architecture.md#arch-core) | `core` artifact model and rendering | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` | 🚧 |
| <a id="arch-anr"></a>[`ARCH-ANR`](./architecture.md#arch-anr) | `anr` scanner crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-MCP-ANR-01` | 🚧 |
| <a id="arch-apps"></a>[`ARCH-APPS`](./architecture.md#arch-apps) | `apps` scanner crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-MCP-APP-01` | 🚧 |
| <a id="arch-stacktrace"></a>[`ARCH-STACKTRACE`](./architecture.md#arch-stacktrace) | `stacktrace` scanner crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-MCP-CRS-01` | 🚧 |
| <a id="arch-memory"></a>[`ARCH-MEMORY`](./architecture.md#arch-memory) | `memory` scanner crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-MCP-MEM-01` | 🚧 |
| <a id="arch-cpu"></a>[`ARCH-CPU`](./architecture.md#arch-cpu) | `cpu` scanner crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-MCP-CPU-01` | 🚧 |
| <a id="arch-support"></a>[`ARCH-SUPPORT`](./architecture.md#arch-support) | `support` shared CLI/JSON plumbing | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-06` … `11` | 🚧 |
| <a id="arch-cli"></a>[`ARCH-CLI`](./architecture.md#arch-cli) | `cli` binary | ✅ `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` · 🚧 `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11` | mixed |
| <a id="arch-mcp"></a>[`ARCH-MCP`](./architecture.md#arch-mcp) | `mcp` server crate | ✅ `REQ-MCP-01` … `06`, `08` … `10` · 🚧 `DRAFT-REQ-MCP-07` | mixed |
| <a id="arch-mcp-tools"></a>[`ARCH-MCP-TOOLS`](./architecture.md#arch-mcp-tools) | MCP tool surface | ✅ `REQ-MCP-ARC-01` … `07`, `REQ-MCP-05`, `REQ-MCP-06` · 🚧 `DRAFT-REQ-MCP-{ANR,APP,CRS,MEM,CPU}-01` | mixed |

## Use Cases

IDs originate in [`use-cases.md`](./use-cases.md).

| ID | Use case | Requirements | Status |
| --- | --- | --- | --- |
| <a id="uc-01"></a>[`UC-01`](./use-cases.md#uc-01) | Engineer inspects a bugreport through the MCP server | `REQ-MCP-01` … `10`, `REQ-MCP-ARC-01` … `07`, `REQ-GEN-01`, `REQ-GEN-02`, `REQ-CLI-03` | ✅ |
| <a id="uc-02"></a>[`UC-02`](./use-cases.md#uc-02) | Agent searches without exhausting context | `REQ-MCP-ARC-04`, `05`, `06`, `07`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09` | ✅ |
| <a id="uc-03"></a>[`UC-03`](./use-cases.md#uc-03) | Engineer triages from a static report | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` | 🚧 |
| <a id="uc-04"></a>[`UC-04`](./use-cases.md#uc-04) | CI pipeline regression artifacts | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` | 🚧 |
| <a id="uc-05"></a>[`UC-05`](./use-cases.md#uc-05) | Third-party scanner via subcommand | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11` | 🚧 |
| <a id="uc-06"></a>[`UC-06`](./use-cases.md#uc-06) | Research batch-processing of NDJSON | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` | 🚧 |

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
| `ARCH-*` | [`architecture.md`](./architecture.md) | Crate or component |
| `UC-*` | [`use-cases.md`](./use-cases.md) | Use case |
| `CLS-*`, `SEQ-*`, `ACT-*` | not yet redrawn | Class, sequence step, activity step |

Every ID is also a link target in both directions: an element row here links to the document that
defines the element, and the document links back here. IDs are stable — an ID is never reused for a
different thing, and a withdrawn element keeps its ID with its row marked withdrawn.