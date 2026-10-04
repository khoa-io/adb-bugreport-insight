<copilot-ref kind="file" target-id="docs/requirements.md" label="Requirements" />
<copilot-ref kind="file" target-id="docs/requirement-diagram.md" label="Requirement Diagram" />
<copilot-ref kind="file" target-id="docs/traceability-matrix.md" label="Traceability Matrix" />
<copilot-ref kind="file" target-id="docs/artifact-schemas.md" label="Artifact Schemas (DRAFT)" />
<copilot-ref kind="file" target-id="docs/use-cases.md" label="Use Case Diagram" />
<copilot-ref kind="file" target-id="docs/architecture.md" label="Architecture Diagram" />

<!-- `classes.md`, `sequences.md`, and `activities.md` were removed during the redesign: they
described the previous implementation's function names and crate layout, which no longer matches the
committed design. They will be redrawn against the committed requirements once the `core` and `mcp`
crates are implemented. -->

## Design decisions

[`design/`](./design/index.md) holds the accepted design for the redesign: the rule-1 conflict
resolution, the final ID list, the `crates/archive` API, the offline dependency rulings, the
build plan, the planned documentation restructure, and the open items.

Start at [`design/index.md`](./design/index.md). Engineers should read
[`design/03-id-scheme.md`](./design/03-id-scheme.md),
[`design/04a-core-archive-api.md`](./design/04a-core-archive-api.md), and
[`design/04b-core-archive-types.md`](./design/04b-core-archive-types.md) first.
