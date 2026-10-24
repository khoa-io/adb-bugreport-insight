# Components and dependencies

## Committed elements

Nine committed elements implement archive, MCP, and static insight scope.
[Workspace](../../Cargo.toml) and package manifests own names/dependencies; the archive library is `crates/core`.
[Proposal architecture](../proposals/03-architecture.md) defines eight active deferred elements and two superseded proposal IDs.

| ID | Crate | Role | Depends on | Status |
| --- | --- | --- | --- | --- |
| <a id="arch-archive"></a>[`ARCH-ARCHIVE`](../traceability.md#arch-archive) | `crates/core` | ZIP central-directory index; bounded line, byte-range, and content search; content detection; archive identity | `zip`, `serde`, `serde_json`, `thiserror`, `sha2`, `grep-searcher`, `grep-regex`, `grep-printer` | ✅ |
| <a id="arch-mcp-srv"></a>[`ARCH-MCP-SRV`](../traceability.md#arch-mcp-srv) | `crates/mcp` | Server lifecycle, stdio transport, confirmation/error wire mapping | `bugreport-insight-core`, `serde`, `serde_json`, `thiserror`, `tokio`, `rmcp` | ✅ |
| <a id="arch-mcp-tools-arc"></a>[`ARCH-MCP-TOOLS-ARC`](../traceability.md#arch-mcp-tools-arc) | `crates/mcp` | Seven archive tools behind `ToolHandler`, with optional selection of a report in confined directory sessions and content-IO confirmation wrappers | `bugreport-insight-core`, `serde`, `serde_json` | ✅ |
| <a id="arch-cli-flags"></a>[`ARCH-CLI-FLAGS`](../traceability.md#arch-cli-flags) | `crates/cli` | The `bugreport-insight` binary: `--mcp`, `--insight`, `--force`, `--help`, `--verbose` | `clap`, `tokio`, `bugreport-insight-mcp` | ✅ |
| <a id="arch-cli-insight"></a>[`ARCH-CLI-INSIGHT`](../traceability.md#arch-cli-insight) | `crates/cli` | `--insight` mode selection and process outcome mapping | `clap`, `tokio`, `bugreport-insight-core` | ✅ |
| <a id="arch-insight-analyzers"></a>[`ARCH-INSIGHT-ANALYZERS`](../traceability.md#arch-insight-analyzers) | `crates/core` | Bounded system, apps, ANR, crash, memory, and CPU evidence analysis | Existing core archive API, `serde`, `serde_json`, `chrono`, `regex-automata` | ✅ |
| <a id="arch-insight-writer"></a>[`ARCH-INSIGHT-WRITER`](../traceability.md#arch-insight-writer) | `crates/core` | Seven artifact structures, serialization, rendering, and staged publication | `ARCH-ARCHIVE`, `ARCH-INSIGHT-ANALYZERS`, `serde`, `serde_json`, `libc` | ✅ |
| <a id="arch-mcp-res"></a>[`ARCH-MCP-RES`](../traceability.md#arch-mcp-res) | `crates/mcp` | Read-only sibling artifact discovery, bounded exact reads and archive-identity binding | `ARCH-ARCHIVE`, std filesystem, `serde_json`; thin rmcp adapter | ✅ |
| <a id="arch-mcp-insight"></a>[`ARCH-MCP-INSIGHT`](../traceability.md#arch-mcp-insight) | `crates/mcp` | Confined bugreport discovery, report-scoped archive dispatch, confirmed in-memory analysis and paged insight tools | `ARCH-ARCHIVE`, `ARCH-MCP-TOOLS-ARC`, `ARCH-INSIGHT-ANALYZERS`, `ARCH-INSIGHT-WRITER`, confirmation gate | ✅ |

## Lazy archive validation

ARCH-ARCHIVE satisfies REQ-GEN-02 using metadata admission at open and produced-byte enforcement during selected reads.
Each reader owns its ZIP parser and logical cursor over the pinned file; chunk iteration borrows that reader and retains one decoder with the shared progress guard.
The aggregate admission limit is declared-only, with no global mutable session counter; unread payloads remain unverified.
[ADR 0004](../decisions/0004-lazy-archive-validation.md) records this boundary and its tradeoffs.

## Tool-response boundary

ARCH-MCP-TOOLS-ARC constructs successful ToolEnvelope values and structured ToolError failures; ARCH-MCP-SRV maps those outcomes onto their MCP/JSON-RPC channels.
ARCH-ARCHIVE converts source line bytes into UTF-8-preserving excerpts before MCP JSON serialization; the decoder correction preserves these existing responsibilities.
Successful empty/partial results retain all four bounding fields; errors make no successful-result completeness claim.
Metadata is structured discovery/identity information, while content evidence uses locators/excerpts; bounded marker metadata and complete insight resources have distinct contracts.
[ADR 0006](../decisions/0006-tool-response-scope.md) records scope; [ADR 0007](../decisions/0007-emitted-utf8-excerpt-cap.md) assigns emitted-byte capping/source progress to ARCH-ARCHIVE and envelope truncation propagation to ARCH-MCP-TOOLS-ARC.
ARCH-ARCHIVE validates optional marker identity under [ADR 0008](../decisions/0008-validated-marker-metadata.md), returning explicit unavailable states without losing other archive metadata.
ARCH-MCP-TOOLS-ARC reports marker warnings/oversize withholding; insight consumers distinguish missing format-0 markers from invalid unknown versions.
Search budgets, persistent independent decoders and bounded handler cursor storage follow [ADR 0009](../decisions/0009-client-search-budgets-and-pagination.md); consistent collection statuses and explicit detection failures follow [ADR 0010](../decisions/0010-empty-results-and-detection-failures.md).

## Dependency rules and observed graph

`crates/core` depends on external libraries, including ZIP, serialization, SHA-256 and grep matching.
It has no workspace sibling runtime dependency.
Insight uses `chrono` for explicit device/host clocks, `regex-automata` for cached structured matching and `libc` for exclusive atomic publication on macOS/Linux.
`crates/mcp` consumes core and owns rmcp adaptation.
`crates/cli` consumes both core and MCP: `--mcp` calls `bugreport_insight_mcp::serve`, while `--insight` calls the core analysis and artifact writer APIs.
There is no reverse edge from MCP to CLI or from core to either consumer.
[REQ-MCP-01](../swe.1-requirements/02-software-requirements.md#req-mcp-01) permits this composition under [ADR 0005](../decisions/0005-cli-composes-mcp-and-core.md).
MCP must not reach CLI through production dependencies, and core must remain independent of workspace siblings; development-only fixture dependencies are excluded from this boundary.
Separate crates preserve responsibility and reuse while allowing the executable to link both libraries.

Insight analysis reads the opened archive through `ARCH-ARCHIVE` and produces in-memory artifact models through `ARCH-INSIGHT-ANALYZERS`.
`ARCH-INSIGHT-WRITER` renders into a unique sibling staging directory and renames that directory to `.<file-stem>/insight/` only after all seven files are complete.
An existing final directory causes an error before analysis output is published.
The source ZIP is never an output target.

`crates/test-fixtures` depends only on std and is a dev-dependency of core, MCP and CLI.
It builds cross-package synthetic ZIP fixtures; it is not shipped.
See [component verification](../swe.5-component-and-integration-verification/01-component-verification.md).

## Superseded IDs

These retired IDs retain their spelling under R-ID-3.

| Superseded ID | Status at retirement | Superseded by | Why |
| --- | --- | --- | --- |
| <a id="arch-core-arc"></a>`ARCH-CORE-ARC` | committed | `ARCH-ARCHIVE` | Split out of a crate whose artifact half is draft |
| <a id="arch-cli"></a>`ARCH-CLI` | committed and draft | `ARCH-CLI-FLAGS` + `DRAFT-ARCH-CLI-CMD` | Three committed flags, one draft command |
| <a id="arch-mcp"></a>`ARCH-MCP` | committed and draft | `ARCH-MCP-SRV` + `DRAFT-ARCH-MCP-RES` | `DRAFT-REQ-MCP-07` moved to its own element |
| <a id="arch-mcp-tools"></a>`ARCH-MCP-TOOLS` | committed and draft | `ARCH-MCP-TOOLS-ARC` + `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` | Seven committed tools, five draft |

## Confirmation ownership

REQ-MCP-12 is allocated to ARCH-MCP-TOOLS-ARC and ARCH-MCP-SRV, with ARCH-MCP-RES executing only approved provider requests.
The protocol-free ConfirmationGate owns bounded process-local request/token state; registry wrappers gate the five content tools, and the adapter gates resource methods before spawn_blocking.
Index-only tools and discovery remain immediate; approved work uses the existing independent core readers and resource provider, without changing crate dependencies.
Cost previews inspect the immutable index and request limits rather than reading payloads; token checks precede grep-cursor consumption.
[ADR 0013](../decisions/0013-confirmation-before-content-io.md) and [detailed design](../swe.3-detailed-design/03-mcp-server.md#content-io-confirmation) define the ownership, lifecycle and limits.

## Combined MCP and insight scope

REQ-MCP-13 is allocated to ARCH-MCP-INSIGHT and ARCH-MCP-SRV; REQ-MCP-14 is allocated to ARCH-MCP-INSIGHT, ARCH-MCP-TOOLS-ARC and ARCH-MCP-SRV; REQ-CLI-12 is allocated to ARCH-CLI-FLAGS, ARCH-ARCHIVE and ARCH-INSIGHT-ANALYZERS.
The CLI selects the immutable default or raised resource policy before opening archives or starting transport.
A single-file combined server composes the existing archive handlers/resources with three insight handlers.
A directory server owns a confined immediate-child catalog and exposes the seven archive handlers plus the three insight handlers; it has no global archive selection state.
Discovery checks central-directory metadata only and never extracts files or traverses nested/parent directories.
Unix directory sessions pin a directory descriptor and open no-follow regular children relative to it; core retains each admitted file handle for subsequent reads and hashing.
Windows directory sessions pin a directory handle, enumerate it with `GetFileInformationByHandleEx`, resolve it with `GetFinalPathNameByHandleW` and open children with `FILE_FLAG_OPEN_REPARSE_POINT`; a reparse point is refused rather than followed, and a renamed directory stays the pinned directory.
Platforms with neither implementation fail directory startup as unsupported rather than providing a pathname-check fallback.
Each archive tool accepts an optional exact `bugreport` basename; in directory mode omission is accepted only when complete discovery finds one report, and each successful result identifies the selected report because entry IDs are report-local.
The selected report's open archive context is pinned and retained for up to 300 seconds of inactivity; at most eight contexts are retained, and a full active cache returns an explicit capacity error.
The per-report handler set keeps grep cursors scoped to that report; confirmation calls share the session's 64-token pool and bind to a process-local context generation as well as the basename, so a reopened basename cannot reuse an older snapshot's token.
The insight provider validates basename IDs against the catalog before opening a candidate, generates the seven artifacts in memory after confirmation, and caches at most one report's artifacts.
Existing artifact resource reads remain a separate API for already-published sibling files; insight tools require no publication and perform no filesystem writes.
This composition preserves crate boundaries, read-only access and stdout protocol integrity.
