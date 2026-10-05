# bugreport-insight

A read-only [MCP](https://modelcontextprotocol.io) server over an Android
`adb bugreport` ZIP archive. See [`docs/index.md`](./docs/index.md) for the tool
set, the workspace layout, and how to build.

```console
bugreport-insight BUGREPORT.zip --mcp
```

- [`docs/requirements.md`](./docs/requirements.md) — what the tool must do
- [`docs/diagrams/`](./docs/diagrams/index.md) — requirement-to-element diagrams
- [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) — the join between them
- [`docs/use-cases.md`](./docs/use-cases.md) — use cases and actors
- [`docs/architecture.md`](./docs/architecture.md) — what the tool is made of
- [`docs/draft/`](./docs/draft/index.md) — deferred work, including the artifact schemas

## Design decisions

[`docs/design/`](./docs/design/index.md) holds the accepted design for the
redesign: the rule-1 conflict resolution, the final ID list, the core archive
API, the offline dependency rulings, the build plan, and the open items. It is
a **historical record** of decisions made before implementation, not a plan for
work still to do.

Start at [`design/index.md`](./docs/design/index.md). Engineers should read
[`design/03-id-scheme.md`](./docs/design/03-id-scheme.md),
[`design/04a-core-archive-api.md`](./docs/design/04a-core-archive-api.md), and
[`design/04b-core-archive-types.md`](./docs/design/04b-core-archive-types.md)
first.

> The design set's crate naming (`crates/archive`) is **superseded**: the crate is
> `crates/core`, package `bugreport-insight-core`. See
> [`architecture.md` §3](./docs/architecture.md#31-committed). `docs/` wins for scope
> and `docs/design/` for reasoning.