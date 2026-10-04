# 08 — Documentation restructure manifest

> **Status: PLANNED, NOT YET APPLIED.**
>
> None of the files below have been created or modified. Applying this restructure is a
> separate phase, after this design set is accepted. The only file changed in this phase is
> [`docs/design/`](./index.md) itself, plus a pointer from [`docs/index.md`](../index.md).
>
> **Spec files are unchanged.** `requirements.md`, `requirement-diagram.md`,
> `traceability-matrix.md`, `architecture.md`, `use-cases.md`, and `project-summary.md` still
> carry their pre-redesign content.

## 1. Why the restructure is needed

| File | Current lines | Diagrams | Rule 4 applies? |
| --- | --- | --- | --- |
| `requirement-diagram.md` | 602 | 6 `requirementDiagram` | **yes** — over 500 |
| `artifact-schemas.md` | 4,973 | 0 | no — no diagrams, but 4,973 lines is unsustainable |
| `requirements.md` | 543 | 0 | no — but one spec for 48 requirements is too long |
| `architecture.md` | 111 | 1 `flowchart` | no |
| `use-cases.md` | 94 | 1 `flowchart` | no |
| `traceability-matrix.md` | 94 | 0 | no |
| `project-summary.md` | 83 | 0 | no |

Rule 4 triggers on exactly one file. Rule 3 forces UML types in two more.

## 2. Naming

The user's rule 4 names the split form `-diagram-<ID>.md`. The base document is itself called
`requirement-diagram.md`, so applying the rule literally yields
`requirement-diagram-diagram-*.md`. The naming **`docs/diagram-<ID>.md`** was proposed and
**accepted** — it delivers what the rule intends without the stutter.

Rule 4 also presupposes a diagram ID, which the docs previously lacked. Hence the new `DGM-*`
family.

## 3. Rule 3 — UML type per location

| Location | Now | Becomes |
| --- | --- | --- |
| `architecture.md` overview | `flowchart TD` | `componentDiagram` |
| `architecture.md` lifecycle | — | `stateDiagram-v2` |
| `use-cases.md` | `flowchart TD` with `style` directives | `usecaseDiagram` |
| `classes.md` | — | `classDiagram` |
| `sequences.md` | — | `sequenceDiagram` |
| `mcp-protocol.md` | — | `stateDiagram-v2` + `sequenceDiagram` |
| `diagram-DGM-*.md` ×6 | `requirementDiagram` ×6 | **unchanged** — already correct UML |

**Gate limitation, recorded honestly:** `mermaid-svg` 0.7.0 — the project's syntax gate —
**does not support `componentDiagram` or `usecaseDiagram`.** Both blocks must render as
`flowchart` until that is resolved, with an inline comment saying why. See
[`09` §4](./09-open-items.md#4-diagram-gate-caveats).

Inline `style` blocks and `<br/>` labels are removed everywhere.

## 4. Rewritten in place

| File | Target lines | Contents |
| --- | --- | --- |
| `docs/index.md` | ~70 | Real Markdown links. The `<copilot-ref/>` tags at `index.md:1-6` render as nothing outside Zed |
| `docs/requirements.md` | ~300 | **Committed requirements only**, 23 IDs, with the rewrites at [`03` §5](./03-id-scheme.md#5-family-1--req-) |
| `docs/requirements-draft.md` | ~270 | **New.** 25 draft IDs, banner at top |
| `docs/requirement-diagram.md` | ~130 | **Index and narrative, zero diagrams.** Links to all six `DGM-*` files. Fixes the missing prefix at `requirement-diagram.md:447` |
| `docs/architecture.md` | ~220 | `componentDiagram` + `stateDiagram-v2`, inline. Deletes the `ripgrep` claim (`:53,62,96`) and the `tar` claim (`:53`). Crate table gets a `Committed elements` column, **not a status word** |
| `docs/use-cases.md` | ~140 | `usecaseDiagram` + six use cases, four prefixed |
| `docs/traceability-matrix.md` | ~180 | **No `mixed` row.** 17 `ARCH-*` rows (13 live + 4 superseded), 6 `UC-*` rows. Adds `CLS-*`, `SEQ-*`, `DGM-*` to the convention table; retires `ACT-*`. **Fixes `:38` ("three mixed rows") and `:65` (`UC-01` range)** |
| `docs/project-summary.md` | ~95 | Updated to 23 / 25 |

## 5. New files

| File | Target lines | Contents |
| --- | --- | --- |
| `docs/diagram-DGM-ARCHIVE.md` | ~150 | `ARCH-ARCHIVE` → `REQ-GEN-01/02`, `REQ-MCP-03`, `REQ-MCP-ARC-01`…`07` |
| `docs/diagram-DGM-SERVER.md` | ~150 | `ARCH-MCP-SRV` → `REQ-MCP-01`…`06`, `08`…`11` |
| `docs/diagram-DGM-RES.md` | ~60 | `DRAFT-ARCH-MCP-RES` → `DRAFT-REQ-MCP-07`. One node; needs a home that is not the server |
| `docs/diagram-DGM-CLI.md` | ~90 | `ARCH-CLI-FLAGS` → `REQ-CLI-03/04/05` |
| `docs/diagram-DGM-ANALYSIS.md` | ~150 | Five analyzers → `DRAFT-REQ-GEN-03`, five `DRAFT-REQ-MCP-*` |
| `docs/diagram-DGM-ARTIFACTS.md` | ~170 | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` → `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01`…`08` |
| `docs/classes.md` | ~180 | **Revived.** `classDiagram` of the [`04a`](./04a-core-archive-api.md) and [`04b`](./04b-core-archive-types.md) types. **Closes the `REQ-GEN-01`/`02` traceability hole** |
| `docs/sequences.md` | ~200 | **Revived.** `sequenceDiagram` ×3: startup and `initialize`; `read_entry`; `grep_archive` |
| `docs/mcp-protocol.md` | ~260 | stdio lifecycle, result envelope, `ArchiveError` → `ToolError` mapping, the `ArchiveStatus` projection table, the `REQ-MCP-08` four-way matrix |

## 6. Deleted

| File | Why |
| --- | --- |
| `docs/activities.md` | **Stays deleted.** No committed workflow exists to depict — "serve MCP over stdio" is a state diagram. `ACT-*` is retired. Reviving it would produce a document of invented filler |
| `docs/classes.md` | Deleted at HEAD; **revived** by this restructure |
| `docs/sequences.md` | Deleted at HEAD; **revived** by this restructure |

## 7. Untouched

`docs/artifact-schemas.md` — 4,973 lines, no diagrams, so rule 4 does not apply. Entirely draft
and off the committed critical path. It does need splitting eventually; that is issue 14.

## 8. Rule 3 verification

Every new Mermaid block must pass `target/doc-validate` before it is committed:

```bash
cd target/doc-validate
mkdir -p svg
CARGO_HOME=../cargo-home SVG_OUT_DIR=svg \
  cargo run --quiet --offline --bin doc-validate -- ../../docs/design/03-id-scheme.md
```

Note two operational details discovered while writing this design set:

- **`SVG_OUT_DIR` must already exist.** The validator does not create it and panics on write.
- **The binary must be named**: `cargo run --bin doc-validate --`, because the crate has two
  binaries and cargo cannot choose.

## 9. Definition of done for this phase

The restructure is applied when:

1. All six `DGM-*` files exist and each carries a link back to `requirement-diagram.md`.
2. `requirements-draft.md` exists and `requirements.md` contains no `DRAFT-` requirement.
3. `traceability-matrix.md` has no `mixed` row and enumerates all 17 `ARCH-*` and 6 `UC-*` IDs.
4. Every count in every document reads 23 / 25 / 48.
5. `requirement-diagram.md:447` carries the `DRAFT-` prefix.
6. `traceability-matrix.md:65` enumerates `UC-01`'s requirements rather than ranging them.
7. `docs/index.md` links every spec document **and** the design set.
8. Every Mermaid block passes `target/doc-validate`.
9. No file exceeds 500 lines, or is split per rule 4.