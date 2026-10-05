# bugreport-insight

A read-only [MCP](https://modelcontextprotocol.io) server over an Android
`adb bugreport` ZIP archive. See [`docs/index.md`](./docs/index.md) for the tool
set, the workspace layout, and how to build.

```console
bugreport-insight BUGREPORT.zip --mcp
```

<copilot-ref kind="file" target-id="docs/requirements.md" label="Requirements" />
<copilot-ref kind="file" target-id="docs/requirement-diagram.md" label="Requirement Diagram" />
<copilot-ref kind="file" target-id="docs/traceability-matrix.md" label="Traceability Matrix" />
<copilot-ref kind="file" target-id="docs/artifact-schemas.md" label="Artifact Schemas (DRAFT)" />
<copilot-ref kind="file" target-id="docs/use-cases.md" label="Use Case Diagram" />
<copilot-ref kind="file" target-id="docs/architecture.md" label="Architecture Diagram" />

## Design decisions

[`docs/design/`](./docs/design/index.md) holds the accepted design for the
redesign: the rule-1 conflict resolution, the final ID list, the core archive
API, the offline dependency rulings, the build plan, the planned documentation
restructure, and the open items.

Start at [`design/index.md`](./docs/design/index.md). Engineers should read
[`design/03-id-scheme.md`](./docs/design/03-id-scheme.md),
[`design/04a-core-archive-api.md`](./docs/design/04a-core-archive-api.md), and
[`design/04b-core-archive-types.md`](./docs/design/04b-core-archive-types.md)
first.

> The design set's crate naming (`crates/archive`) is **superseded**: the crate is
> `crates/core`, package `bugreport-insight-core`. See
> [`architecture.md` §3](./docs/architecture.md#31-committed). `docs/` wins for scope
> and `docs/design/` for reasoning.