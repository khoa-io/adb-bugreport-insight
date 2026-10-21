# Deferred architecture

> Proposal record.
> The single-report artifact path is defined in [committed SWE.2](../swe.2-architecture/index.md).

<a id="32-draft--recorded-not-built"></a>

## Draft elements

Remaining proposed capabilities are not implemented.
`core`, `cli`, and `mcp` exist; analyzer MCP tools and per-analysis toggles remain deferred; artifact resources are committed.
Do not implement against these proposed elements without promoting their scope.

| ID | Crate | Role | Status |
| --- | --- | --- | --- |
| <a id="draft-arch-cli-cmd"></a>[`DRAFT-ARCH-CLI-CMD`](../traceability.md#draft-arch-cli-cmd) | `crates/cli` | Per-analysis CLI toggles | 🚧 |
| <a id="draft-arch-mcp-tools-analysis"></a>[`DRAFT-ARCH-MCP-TOOLS-ANALYSIS`](../traceability.md#draft-arch-mcp-tools-analysis) | `crates/mcp` | Five analyzer tools | 🚧 |
| <a id="draft-arch-anr"></a>[`DRAFT-ARCH-ANR`](../traceability.md#draft-arch-anr) | — | ANR scanner | 🚧 |
| <a id="draft-arch-apps"></a>[`DRAFT-ARCH-APPS`](../traceability.md#draft-arch-apps) | — | App inventory scanner | 🚧 |
| <a id="draft-arch-stacktrace"></a>[`DRAFT-ARCH-STACKTRACE`](../traceability.md#draft-arch-stacktrace) | — | Tombstone and Java stack-trace scanner | 🚧 |
| <a id="draft-arch-memory"></a>[`DRAFT-ARCH-MEMORY`](../traceability.md#draft-arch-memory) | — | Memory-pressure and LMK scanner | 🚧 |
| <a id="draft-arch-cpu"></a>[`DRAFT-ARCH-CPU`](../traceability.md#draft-arch-cpu) | — | CPU / I-O pressure scanner | 🚧 |
| <a id="draft-arch-support"></a>[`DRAFT-ARCH-SUPPORT`](../traceability.md#draft-arch-support) | — | Shared CLI and JSON plumbing | 🚧 |

The five analyzer elements and `DRAFT-ARCH-SUPPORT` were the `subcommand-*` crates.
**Dropped:** absent from `Cargo.toml`; this form will not return.
Withdrawn IDs remain stable ([ID conventions](../conventions.md#ids-and-links)).

<a id="draft-arch-core-art"></a>`DRAFT-ARCH-CORE-ART` is superseded by [`ARCH-INSIGHT-ANALYZERS`](../swe.2-architecture/02-components-and-dependencies.md#arch-insight-analyzers) and [`ARCH-INSIGHT-WRITER`](../swe.2-architecture/02-components-and-dependencies.md#arch-insight-writer).

<a id="draft-arch-mcp-res"></a>`DRAFT-ARCH-MCP-RES` is superseded by [`ARCH-MCP-RES`](../swe.2-architecture/02-components-and-dependencies.md#arch-mcp-res).

## Related proposals

See [deferred requirements](02-requirements.md), [use cases](./01-use-cases.md), and [artifact schemas](artifact-schemas/index.md).
Their IDs identify proposals, not shipped capabilities.
