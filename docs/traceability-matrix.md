# Traceability matrix

The join. [`requirements.md`](./requirements.md) states what the tool must do;
[`architecture.md`](./architecture.md) says what it is made of;
[`diagrams/index.md`](./diagrams/index.md) says which element satisfies which requirement. This file
records which design elements are in scope for which requirements, so a reader can go from an
implementation decision up to the requirement that justifies it, and back down again.

**50 requirements: 23 committed, 27 draft.**

## How to read the status column

| Mark | Meaning |
| --- | --- |
| ✅ | Committed. The requirement is binding now and the element exists. |
| 🚧 | Draft. Every requirement served here is prefixed `DRAFT-`. **Not traceability evidence.** Do not implement against a draft ID. |

A `DRAFT-` requirement may change or be withdrawn without that being a breaking change. A row with no
✅ anywhere in its requirement list is not evidence that anything is committed. See
[`draft/requirements.md`](./draft/requirements.md) for what the prefix means.

## Requirement coverage

Committed requirements first. Every one is implemented; the element column is where it landed.

| Requirements | Count | Elements |
| --- | --- | --- |
| `REQ-GEN-01`, `REQ-GEN-02` | 2 | `ARCH-ARCHIVE` |
| `REQ-LIB-01` | 1 | `ARCH-ARCHIVE` |
| `REQ-MCP-01`, `REQ-MCP-02`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11` | 10 | `ARCH-MCP-SRV` |
| `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07` | 7 | `ARCH-ARCHIVE`, `ARCH-MCP-TOOLS-ARC` |
| `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` | 3 | `ARCH-CLI-FLAGS` |
| `DRAFT-REQ-GEN-03` | 1 | `DRAFT-ARCH-ANR`, `DRAFT-ARCH-APPS`, `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MEMORY`, `DRAFT-ARCH-CPU` |
| `DRAFT-REQ-GEN-04` | 1 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` |
| `DRAFT-REQ-MCP-07` | 1 | `DRAFT-ARCH-MCP-RES` |
| `DRAFT-REQ-MCP-ARC-08`, `DRAFT-REQ-MCP-ARC-09` | 2 | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-TOOLS-ANALYSIS` | 1 | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-ANR-01` | 1 | `DRAFT-ARCH-ANR`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-APP-01` | 1 | `DRAFT-ARCH-APPS`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-CRS-01` | 1 | `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-MEM-01` | 1 | `DRAFT-ARCH-MEMORY`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-CPU-01` | 1 | `DRAFT-ARCH-CPU`, `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-CLI-11` | 9 | `DRAFT-ARCH-CLI-CMD`, `DRAFT-ARCH-SUPPORT` |
| `DRAFT-REQ-ART-01` … `DRAFT-REQ-ART-08` | 8 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` |

## Architecture elements

IDs originate in [`architecture.md`](./architecture.md), which also records the four superseded IDs.

## Committed

| ID | Element | Crate | Requirements |
| --- | --- | --- | --- |
| <a id="arch-archive"></a>[`ARCH-ARCHIVE`](./architecture.md#31-committed) | ZIP index, bounded reader and search, content detection, archive identity | `crates/core` | `REQ-GEN-01`, `REQ-GEN-02`, `REQ-LIB-01`, `REQ-MCP-03`, `REQ-MCP-09`, `REQ-MCP-ARC-01` … `07` |
| <a id="arch-mcp-srv"></a>[`ARCH-MCP-SRV`](./architecture.md#31-committed) | Server lifecycle, stdio transport, error mapping | `crates/mcp` | `REQ-MCP-01` … `06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11`, `REQ-CLI-03` |
| <a id="arch-mcp-tools-arc"></a>[`ARCH-MCP-TOOLS-ARC`](./architecture.md#31-committed) | The seven archive tools behind the `ToolHandler` trait | `crates/mcp` | `REQ-MCP-ARC-01` … `07`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08` |
| <a id="arch-cli-flags"></a>[`ARCH-CLI-FLAGS`](./architecture.md#31-committed) | `--mcp`, `--help`, `--verbose` | `crates/cli` | `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` |

## Draft — recorded, not built

| ID | Element | Requirements |
| --- | --- | --- |
| <a id="draft-arch-core-art"></a>[`DRAFT-ARCH-CORE-ART`](./architecture.md#32-draft--recorded-not-built) | Artifact model and rendering | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` |
| <a id="draft-arch-cli-cmd"></a>[`DRAFT-ARCH-CLI-CMD`](./architecture.md#32-draft--recorded-not-built) | `analyze` command and seven toggles | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11` |
| <a id="draft-arch-mcp-res"></a>[`DRAFT-ARCH-MCP-RES`](./architecture.md#32-draft--recorded-not-built) | MCP resource surface | `DRAFT-REQ-MCP-07` |
| <a id="draft-arch-mcp-tools-analysis"></a>[`DRAFT-ARCH-MCP-TOOLS-ANALYSIS`](./architecture.md#32-draft--recorded-not-built) | Five analyzer tools, plus pattern and format extensions | `DRAFT-REQ-MCP-ARC-08`, `DRAFT-REQ-MCP-ARC-09`, `DRAFT-REQ-MCP-TOOLS-ANALYSIS`, `DRAFT-REQ-MCP-ANR-01`, `DRAFT-REQ-MCP-APP-01`, `DRAFT-REQ-MCP-CRS-01`, `DRAFT-REQ-MCP-MEM-01`, `DRAFT-REQ-MCP-CPU-01` |
| <a id="draft-arch-anr"></a>[`DRAFT-ARCH-ANR`](./architecture.md#32-draft--recorded-not-built) | ANR scanner | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-MCP-ANR-01` |
| <a id="draft-arch-apps"></a>[`DRAFT-ARCH-APPS`](./architecture.md#32-draft--recorded-not-built) | App inventory scanner | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-MCP-APP-01` |
| <a id="draft-arch-stacktrace"></a>[`DRAFT-ARCH-STACKTRACE`](./architecture.md#32-draft--recorded-not-built) | Tombstone and Java stack-trace scanner | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-MCP-CRS-01` |
| <a id="draft-arch-memory"></a>[`DRAFT-ARCH-MEMORY`](./architecture.md#32-draft--recorded-not-built) | Memory-pressure and LMK scanner | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-MCP-MEM-01` |
| <a id="draft-arch-cpu"></a>[`DRAFT-ARCH-CPU`](./architecture.md#32-draft--recorded-not-built) | CPU and I/O pressure scanner | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-MCP-CPU-01` |
| <a id="draft-arch-support"></a>[`DRAFT-ARCH-SUPPORT`](./architecture.md#32-draft--recorded-not-built) | Shared CLI and JSON plumbing | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` |

Fourteen live elements: four committed, ten draft. There is no mixed row, and there cannot be one —
an element serving any draft requirement is itself draft, so a crate is never half-committed.

## Use cases

IDs originate in [`use-cases.md`](./use-cases.md).

| ID | Use case | Requirements |
| --- | --- | --- |
| <a id="uc-01"></a>[`UC-01`](./use-cases.md#uc-01) | Engineer inspects a bugreport through the MCP server | `REQ-GEN-01`, `REQ-GEN-02`, `REQ-CLI-03`, `REQ-MCP-01`, `REQ-MCP-02`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-11`, `REQ-MCP-ARC-01`, `REQ-MCP-ARC-02`, `REQ-MCP-ARC-03`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07` |
| <a id="uc-02"></a>[`UC-02`](./use-cases.md#uc-02) | Agent searches a bugreport without exhausting its context | `REQ-MCP-06`, `REQ-MCP-08`, `REQ-MCP-09`, `REQ-MCP-ARC-04`, `REQ-MCP-ARC-05`, `REQ-MCP-ARC-06`, `REQ-MCP-ARC-07` |
| <a id="draft-uc-03"></a>[`DRAFT-UC-03`](./use-cases.md#draft-uc-03) | Engineer triages from a static report | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` |
| <a id="draft-uc-04"></a>[`DRAFT-UC-04`](./use-cases.md#draft-uc-04) | CI pipeline regression artifacts | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` |
| <a id="draft-uc-05"></a>[`DRAFT-UC-05`](./use-cases.md#draft-uc-05) | Third-party scanner via subcommand | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `11` |
| <a id="draft-uc-06"></a>[`DRAFT-UC-06`](./use-cases.md#draft-uc-06) | Research batch-processing of NDJSON | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` |

## Convention

| Prefix | Document | Kind |
| --- | --- | --- |
| `REQ-*` | [`requirements.md`](./requirements.md) | Committed requirement |
| `DRAFT-REQ-*` | [`draft/requirements.md`](./draft/requirements.md) | Draft requirement |
| `ARCH-*`, `DRAFT-ARCH-*` | [`architecture.md`](./architecture.md) | Crate or component |
| `UC-*`, `DRAFT-UC-*` | [`use-cases.md`](./use-cases.md) | Use case |
| `CLS-*`, `SEQ-*`, `ACT-*` | not yet redrawn | Class, sequence step, activity step |

Every ID is a link target in both directions: an element row here links to the document that defines
the element, and the document links back here. IDs are stable — an ID is never reused for a different
thing, and a withdrawn element keeps its ID with its row marked withdrawn. That is Rule R-ID-3, whose
four instances are listed in [`architecture.md` §9](./architecture.md#9-superseded-ids--rule-r-id-3).
