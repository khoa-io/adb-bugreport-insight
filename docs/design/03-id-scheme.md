# 03 — ID scheme

**This is the file the two implementation engineers code and write against.** Every ID below is
final. Do not invent IDs; if one is needed, mint it under these rules and add it here.

Convention rules are in [`02` §1](./02-amendment-1.md#1-the-draft-prefix) and
[`02` §2](./02-amendment-1.md#2-ruling-on-superseded-ids-rule-r-id-3).

## 1. The four families

| Family | Kind | Prefix rule |
| --- | --- | --- |
| `REQ-*` | Requirement | `DRAFT-` if not committed |
| `ARCH-*` | Architecture component | `DRAFT-` if every mapped requirement is draft |
| `UC-*` | Use case | `DRAFT-` if every mapped requirement is draft |
| `DGM-*` | Diagram index | **never** prefixed — see below |

Anchors drop the `DRAFT-` prefix: `DRAFT-ARCH-ANR` → `#draft-arch-anr`. This matches the
existing convention at `requirements.md:28`. `#` fragments cannot cleanly carry a leading
prefix, and `requirements.md` already lowercases and strips.

**`DGM-*` is never prefixed.** A diagram is a structural index into a document, not a
component. Prefixing `DGM-ARTIFACTS` would mark the specification itself draft and destroy the
ability to _specify_ draft work.

## 2. Committed versus draft

<!-- `componentDiagram` is the correct UML type here, but mermaid-svg 0.7.0 — the project's
     syntax gate — rejects it with "unknown diagram type: componentDiagram". Rendered as
     `flowchart` instead. See 09-open-items.md §4. Validated with target/doc-validate. -->

```mermaid
flowchart TD
    subgraph committed["Committed — unprefixed IDs"]
        ARC["ARCH-ARCHIVE"]
        SRV["ARCH-MCP-SRV"]
        TARC["ARCH-MCP-TOOLS-ARC"]
        FLAGS["ARCH-CLI-FLAGS"]
    end
    subgraph draft["Draft — DRAFT- prefixed IDs"]
        CART["DRAFT-ARCH-CORE-ART"]
        CCMD["DRAFT-ARCH-CLI-CMD"]
        MRES["DRAFT-ARCH-MCP-RES"]
        TANL["DRAFT-ARCH-MCP-TOOLS-ANALYSIS"]
    end
    SRV --> ARC
    TARC --> ARC
    SRV --> TARC
    FLAGS --> SRV
    CART -.-> ARC
    CCMD -.-> CART
    MRES -.-> CART
    TANL -.-> CART
```

Solid edges are committed dependencies. Dashed edges point from a draft component at the
committed crate it will later extend — `DRAFT-ARCH-CORE-ART` will depend on `ARCH-ARCHIVE`;
nothing committed depends on a draft component.

**Counts: 13 live components — 4 committed, 9 draft.** There is no mixed status.

## 3. Family 2 — `ARCH-*`

### Committed

| ID | Element | Requirements |
| --- | --- | --- |
| `ARCH-ARCHIVE` | `crates/archive` — ZIP list / read / find / grep | `REQ-GEN-01`, `REQ-GEN-02`, `REQ-MCP-03`, `REQ-MCP-09`, `REQ-MCP-ARC-01` … `07` |
| `ARCH-MCP-SRV` | `crates/mcp` server core — lifecycle, transport, error mapping | `REQ-MCP-01` … `06`, `REQ-MCP-08`, `09`, `10`, `11` |
| `ARCH-MCP-TOOLS-ARC` | The seven archive tools | `REQ-MCP-ARC-01` … `07`, `REQ-MCP-05`, `REQ-MCP-06`, `REQ-MCP-08` |
| `ARCH-CLI-FLAGS` | `crates/cli` flag parsing — `--mcp`, `--help`, `--verbose` | `REQ-CLI-03`, `REQ-CLI-04`, `REQ-CLI-05` |

### Draft

| ID | Element | Requirements |
| --- | --- | --- |
| `DRAFT-ARCH-CORE-ART` | `crates/core` artifact model and rendering | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` |
| `DRAFT-ARCH-CLI-CMD` | `cli` analyze command and seven toggles | `DRAFT-REQ-CLI-01`, `02`, `06` … `11` |
| `DRAFT-ARCH-MCP-RES` | MCP resource surface | `DRAFT-REQ-MCP-07` |
| `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` | Five analyzer tools | `DRAFT-REQ-MCP-ANR-01`, `APP-01`, `CRS-01`, `MEM-01`, `CPU-01` |
| `DRAFT-ARCH-ANR` | `anr` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-07`, `DRAFT-REQ-MCP-ANR-01` |
| `DRAFT-ARCH-APPS` | `apps` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-06`, `DRAFT-REQ-MCP-APP-01` |
| `DRAFT-ARCH-STACKTRACE` | `stacktrace` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-08`, `DRAFT-REQ-MCP-CRS-01` |
| `DRAFT-ARCH-MEMORY` | `memory` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-09`, `DRAFT-REQ-MCP-MEM-01` |
| `DRAFT-ARCH-CPU` | `cpu` crate | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-10`, `DRAFT-REQ-MCP-CPU-01` |
| `DRAFT-ARCH-SUPPORT` | `support` shared CLI/JSON plumbing | `DRAFT-REQ-CLI-01`, `06` … `11` |

## 4. Superseded IDs

Per **Rule R-ID-3** ([`02` §2](./02-amendment-1.md#2-ruling-on-superseded-ids-rule-r-id-3)),
these keep their original unprefixed form **forever**, whatever their successor's status.

| Superseded ID | Original status | Superseded by | Why |
| --- | --- | --- | --- |
| `ARCH-CORE-ARC` | committed | `ARCH-ARCHIVE` | Split out of a crate whose artifact half is draft; became its own crate |
| `ARCH-CLI` | mixed | `ARCH-CLI-FLAGS` + `DRAFT-ARCH-CLI-CMD` | Three committed flags, one draft command |
| `ARCH-MCP` | mixed | `ARCH-MCP-SRV` + `DRAFT-ARCH-MCP-RES` | `DRAFT-REQ-MCP-07` moved to its own element |
| `ARCH-MCP-TOOLS` | mixed | `ARCH-MCP-TOOLS-ARC` + `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` | Seven committed tools, five draft |

**None of the four was ever draft**, so the R-ID-3 question is moot in practice. All four rows
stay in [`traceability-matrix.md`](../traceability-matrix.md) marked `superseded by`, per the
convention at `traceability-matrix.md:95`.

**`ACT-*` is retired.** No committed workflow exists to depict — "serve MCP over stdio" is a
state diagram, and it lives in `mcp-protocol.md` when that file is created.

## 5. Family 1 — `REQ-*`

**23 committed / 25 draft / 48 total**, changed from 22 / 24 / 46 by two added requirements.

### Committed — `docs/requirements.md`

`REQ-GEN-01`, `REQ-GEN-02`, `REQ-LIB-01`, `REQ-MCP-01` … `REQ-MCP-06`, `REQ-MCP-08`,
`REQ-MCP-09`, `REQ-MCP-10`, `REQ-MCP-ARC-01` … `REQ-MCP-ARC-07`, `REQ-CLI-03`, `REQ-CLI-04`,
`REQ-CLI-05`.

**The `REQ-MCP-01` … `06` range above covers `REQ-MCP-02`** — single-archive scope. It is never
written as the defective range `01 … 10`, which would silently pull in
`DRAFT-REQ-MCP-07`.

### Added by this design

| ID | Status | Purpose |
| --- | --- | --- |
| `REQ-MCP-11` | committed | `initialize` carries `instructions`: ≤ 40 lines naming each tool, its cost class, its bound, and the call order. Publishes no archive content. |
| `DRAFT-REQ-MCP-ARC-08` | draft | Pattern mode `Regex`. Split out of `REQ-MCP-ARC-03` / `-05` because it is blocked on the absent `regex` crate — [`02` §4.2](./02-amendment-1.md#42-regex--globset--replace-do-not-defer). |

### Draft — `docs/requirements-draft.md` (planned)

`DRAFT-REQ-GEN-03`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `DRAFT-REQ-ART-08`,
`DRAFT-REQ-MCP-07`, `DRAFT-REQ-MCP-ARC-08`, `DRAFT-REQ-MCP-ANR-01`, `DRAFT-REQ-MCP-APP-01`,
`DRAFT-REQ-MCP-CRS-01`, `DRAFT-REQ-MCP-MEM-01`, `DRAFT-REQ-MCP-CPU-01`, `DRAFT-REQ-CLI-01`,
`DRAFT-REQ-CLI-02`, `DRAFT-REQ-CLI-06` … `DRAFT-REQ-CLI-11`.

### Requirement rewrites required

Decided in [`01` §3](./01-decision-record.md#3-requirement-quality), not yet applied to
[`requirements.md`](../requirements.md):

| ID | Rewrite |
| --- | --- |
| `REQ-GEN-02` | Add the numeric limits — untestable as written |
| `REQ-MCP-06` | Add every bound; require the four bounding fields always present |
| `REQ-MCP-05` | Delete the untestable clause; state the `tools/list` regression test |
| `REQ-MCP-09` | Keep locator discipline; move the causal-claim clause to the draft analyzer requirements |
| `REQ-GEN-01`, `REQ-MCP-01`, `REQ-MCP-03`, `REQ-MCP-04`, `REQ-MCP-10` | See the table at [`01` §3.5](./01-decision-record.md#35-remaining-rewrites) |
| `REQ-MCP-ARC-02` | ZIP-only — drop "and TAR" |
| `REQ-MCP-ARC-04` | Two named mutually-exclusive range pairs; address by `entry_id` |
| `REQ-MCP-ARC-05` | Reclassify to `interfaceRequirement`; add `byte_offset` to every match |
| `REQ-MCP-ARC-06`, `-07` | Add the `ContentType` and `DetectionBasis` enums |
| `REQ-CLI-03` | Split the mangled bullet; move the output-directory sentence to `DRAFT-REQ-CLI-01` |

## 6. Family 3 — `UC-*`

| ID | Status | Actor | Requirements |
| --- | --- | --- | --- |
| `UC-01` | committed | Engineer / AI agent | `REQ-MCP-01` … `06`, `08` … `11`, `REQ-MCP-ARC-01` … `07`, `REQ-GEN-01`, `REQ-GEN-02`, `REQ-CLI-03` |
| `UC-02` | committed | AI agent | `REQ-MCP-ARC-04`, `05`, `06`, `07`, `REQ-MCP-06`, `08`, `09` |
| `DRAFT-UC-03` | draft | Engineer | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01` … `08` |
| `DRAFT-UC-04` | draft | CI job | `DRAFT-REQ-CLI-01`, `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` |
| `DRAFT-UC-05` | draft | Third-party developer | `DRAFT-REQ-GEN-03`, `DRAFT-REQ-CLI-02`, `06` … `11` |
| `DRAFT-UC-06` | draft | Researcher | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-02` |

`UC-01` is **enumerated, not ranged**. The existing range `REQ-MCP-01 … 10` at
`traceability-matrix.md:65` silently includes draft `DRAFT-REQ-MCP-07` — the defect that
prompted this whole restructure.

## 7. Family 4 — `DGM-*`

All unprefixed, all committed. One file each, named `docs/diagram-<ID>.md`.

| ID | File | Elements depicted |
| --- | --- | --- |
| `DGM-ARCHIVE` | `docs/diagram-DGM-ARCHIVE.md` | `ARCH-ARCHIVE` |
| `DGM-SERVER` | `docs/diagram-DGM-SERVER.md` | `ARCH-MCP-SRV` |
| `DGM-RES` | `docs/diagram-DGM-RES.md` | `DRAFT-ARCH-MCP-RES` |
| `DGM-CLI` | `docs/diagram-DGM-CLI.md` | `ARCH-CLI-FLAGS` |
| `DGM-ANALYSIS` | `docs/diagram-DGM-ANALYSIS.md` | `DRAFT-ARCH-ANR` … `DRAFT-ARCH-CPU` |
| `DGM-ARTIFACTS` | `docs/diagram-DGM-ARTIFACTS.md` | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` |

`DGM-RES` is a single node and gets its own file anyway: it needs a home that is not
`DGM-SERVER`, because folding it back in is what created the original conflict.

## 8. Link targets

Every ID is a link target in both directions. An element row in
[`traceability-matrix.md`](../traceability-matrix.md) links to the document that defines it, and
that document links back.

| Prefix | Lives in | Anchor form |
| --- | --- | --- |
| `REQ-*`, `DRAFT-REQ-*` | [`requirements.md`](../requirements.md), [`requirements-draft.md`](../requirements-draft.md) *(planned)* | `#req-gen-01`, `#draft-req-gen-03` |
| `ARCH-*`, `DRAFT-ARCH-*` | [`architecture.md`](../architecture.md) *(restructure pending)* | `#arch-archive`, `#draft-arch-anr` |
| `UC-*`, `DRAFT-UC-*` | [`use-cases.md`](../use-cases.md) *(restructure pending)* | `#uc-01`, `#draft-uc-03` |
| `DGM-*` | `docs/diagram-<ID>.md` *(planned)* | `#dgm-archive` |
| `CLS-*` | `docs/classes.md` *(planned)* | `#cls-archive` |
| `SEQ-*` | `docs/sequences.md` *(planned)* | `#seq-startup` |

Files marked _planned_ do not exist yet. Their creation is specified in
[`08-doc-restructure-manifest.md`](./08-doc-restructure-manifest.md). **No link in this design
set points at an unlisted target** — every planned file appears in the manifest.

**IDs are stable.** An ID is never reused for a different thing, and a withdrawn element keeps
its ID with its row marked withdrawn or superseded.