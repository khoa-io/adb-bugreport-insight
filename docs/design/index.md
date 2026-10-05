# Design decision records

This directory is the durable record of the `bugreport-insight` redesign: what was decided, why, what
was rejected, and what is still open. The decisions were made across two review passes before
implementation and are persisted here so they survive without the conversation.

> **These are historical records, not a plan.** Sprint 1 is complete: all 23 committed requirements
> are implemented and tested. Where a record describes work as "to be implemented", read it as the
> reasoning that led to what shipped. The committed specification in
> [`../requirements.md`](../requirements.md) wins wherever the two disagree.

The specification documents are the source of truth for *what the tool must do*. This directory is
the source of truth for *why the design is shaped this way*.

## The records

| File | What it carries |
| --- | --- |
| [`01-decision-record.md`](./01-decision-record.md) | The original pass — rule-1 conflict resolution, holes in the committed design, requirement quality, MCP tool surface, build order, deferrals — with Amendment 1 folded into §5. |
| [`03-id-scheme.md`](./03-id-scheme.md) | The complete final ID list across all four families, the prefix rules, and the two-link-target convention. **Code and docs are written against this file.** |
| [`04a-core-archive-api.md`](./04a-core-archive-api.md) · [`04b-core-archive-types.md`](./04b-core-archive-types.md) | The public API of `crates/core`: contract, limits, errors, then entry, reader, detection, search, and identity types. The shipped `lib.rs` doc comment matches. |
| [`05-mcp-server-design.md`](./05-mcp-server-design.md) | stdio lifecycle, the no-lock concurrency model, the error model, the `ArchiveStatus` contract, and the seven-tool surface assessment. |
| [`06-hostile-archive.md`](./06-hostile-archive.md) | Behaviour against hostile archives: line bombs, NUL bytes, lying headers, zip bombs, encrypted entries, nested archives. |
| [`07-build-plan.md`](./07-build-plan.md) | Stages 0–6 including 5.5, disjoint write scopes, merge gates, and the ten acceptance fixtures. **Complete.** |
| [`09-open-items.md`](./09-open-items.md) | The 18 items with a verdict each — resolved, moot, or still open. |

Two records were retired by the restructure: `02-amendment-1.md`, folded into `01` §5, and
`08-doc-restructure-manifest.md`, superseded by the restructure actually being applied.

## Reading order

- **To review or disagree**: [`01`](./01-decision-record.md) first. It records the rejected
  alternatives, which is the part you need in order to argue with a decision.
- **To understand the shipped code**: [`03`](./03-id-scheme.md) →
  [`04a`](./04a-core-archive-api.md) and [`04b`](./04b-core-archive-types.md) →
  [`05`](./05-mcp-server-design.md) → [`06`](./06-hostile-archive.md).
- **To pick up remaining work**: [`09`](./09-open-items.md).

## Standing rules these records were written under

| Rule | Statement |
| --- | --- |
| **Rule 1** | If a component's related requirement is draft, the component is draft. No mixed status. Resolved in [`01` §1](./01-decision-record.md#1-rule-1-conflict-resolution). |
| **Prefix** | A `DRAFT-` prefix is applied once, at mint time, and never modified. See [`01` §5.1](./01-decision-record.md#51-the-draft--prefix-on-component-ids). |
| **Rule R-ID-3** | A superseded ID keeps its original form forever, whatever its successor's status. See [`01` §5.2](./01-decision-record.md#52-rule-r-id-3--a-superseded-id-never-changes-form). |
| **Rule 2** | The traceability matrix is always up to date. |
| **Rule 3** | UML diagrams in Mermaid. No `flowchart` where a UML type fits — except where the syntax gate forbids it. |
| **Rule 4** | Any `.md` over 500 lines is split. No file in this repository exceeds it. |
| **Diagram gate** | Every Mermaid block should pass a syntax gate before it is written. |

## Diagram caveats

`mermaid-svg` 0.7.0 — the syntax gate this set was written against — **does not support
`componentDiagram` or `usecaseDiagram`**. Two diagrams therefore use `flowchart` with an inline
comment recording the substitution: the overview in [`../architecture.md`](../architecture.md) §2 and
the actor view in [`../use-cases.md`](../use-cases.md). `stateDiagram-v2`, `classDiagram`,
`sequenceDiagram`, and `requirementDiagram` are all supported and are used natively.

The gate binary itself (`target/doc-validate`) is **not present in this working tree**, so no Mermaid
block in this restructure was machine-validated. That is a gap, not a pass: re-run the gate before
treating the diagrams as verified.
