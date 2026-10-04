# Design Decision Records

This directory holds the accepted design for the `bugreport-insight` redesign: what was
decided, why, what was rejected, and what is still open. It is the durable record — these
decisions were made across two review passes and are persisted here so they survive without
the conversation.

The specification documents in [`docs/`](../index.md) are the source of truth for *what the
tool must do*. This directory is the source of truth for *why the design is shaped this way*.
Where the two disagree, the specification documents win — but they have not yet been updated
to match, and §[the manifest](./08-doc-restructure-manifest.md) records that gap.

## The records

| File | What it carries |
| --- | --- |
| [`01-decision-record.md`](./01-decision-record.md) | The original pass: rule-1 conflict resolution, holes in the committed design, requirement quality, MCP tool surface, doc restructure, build order, deferrals. |
| [`02-amendment-1.md`](./02-amendment-1.md) | The accepted amendment: `DRAFT-` prefixes on component IDs, the intentional `crates/` deletion, and the offline dependency rulings with their verification evidence. |
| [`03-id-scheme.md`](./03-id-scheme.md) | The complete final ID list across all four families, the prefix rules, and the two-link-target convention. **Code and docs are written against this file.** |
| [`04a-core-archive-api.md`](./04a-core-archive-api.md) · [`04b-core-archive-types.md`](./04b-core-archive-types.md) | The verbatim public API of `crates/archive`: contract, limits, errors, then entry, reader, detection, search, and identity types. Engineer B codes against these without waiting. |
| [`05-mcp-server-design.md`](./05-mcp-server-design.md) | stdio lifecycle, the no-lock concurrency model, the error model, the `ArchiveStatus` contract, and the seven-tool surface assessment. |
| [`06-hostile-archive.md`](./06-hostile-archive.md) | Behaviour against hostile archives: line bombs, NUL bytes, lying headers, zip bombs, encrypted entries, nested archives. |
| [`07-build-plan.md`](./07-build-plan.md) | Stages 0–6 including 5.5, disjoint write scopes, merge gates, and the ten acceptance fixtures. |
| [`08-doc-restructure-manifest.md`](./08-doc-restructure-manifest.md) | The planned documentation restructure. **Planned, not yet applied.** |
| [`09-open-items.md`](./09-open-items.md) | The 18 GitHub issues (listed, not created), the deferrals, and the one ruling deliberately left to the user. |

## Reading order

- **To implement**: [`03`](./03-id-scheme.md) → [`04a`](./04a-core-archive-api.md) and
  [`04b`](./04b-core-archive-types.md) →
  [`05`](./05-mcp-server-design.md) → [`06`](./06-hostile-archive.md) → [`07`](./07-build-plan.md).
- **To review or disagree**: [`01`](./01-decision-record.md) first. It records the rejected
  alternatives, which is the part you need in order to argue with a decision.
- **To apply the documentation change**: [`08`](./08-doc-restructure-manifest.md).
- **To pick up the remaining work**: [`09`](./09-open-items.md).

## Standing rules these records were written under

| Rule | Statement |
| --- | --- |
| **Rule 1** | If a component's related requirement is draft, the component is draft. No mixed status. Resolved in [`01` §1](./01-decision-record.md#1-rule-1-conflict-resolution). |
| **Prefix** | A `DRAFT-` prefix is applied once, at mint time, and never modified. See [`03`](./03-id-scheme.md). |
| **Rule R-ID-3** | A superseded ID keeps its original form forever, whatever its successor's status. |
| **Rule 2** | The traceability matrix is always up to date. |
| **Rule 3** | UML diagrams in Mermaid. No `flowchart` where a UML type fits. |
| **Rule 4** | Any `.md` over 500 lines is split. No file in this directory exceeds it. |
| **Diagram gate** | Every Mermaid block passes `target/doc-validate` before it is written. See [`09` §4](./09-open-items.md#4-diagram-gate-caveats). |

## Diagram caveats

`mermaid-svg` 0.7.0 — the project's syntax gate — **does not support `componentDiagram` or
`usecaseDiagram`**. Two diagrams in this set therefore use `flowchart` with an inline comment
recording the substitution. `stateDiagram-v2`, `classDiagram`, `sequenceDiagram`, and
`requirementDiagram` are all supported and are used natively. The rationale and the evidence
are in [`09` §4](./09-open-items.md#4-diagram-gate-caveats); the affected blocks are
[`03` §2](./03-id-scheme.md#2-committed-versus-draft) and
[`05` §1](./05-mcp-server-design.md#1-the-tool-surface).