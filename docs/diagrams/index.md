# Requirement diagrams

The traceability view: which system element satisfies each requirement, with the risk and the
verification method that requirement carries. Each ID below is an anchor target — the badge printed
next to every requirement in [`../requirements.md`](../requirements.md) and
[`../draft/requirements.md`](../draft/requirements.md) links to its row here, and the row's ID links
forward to the requirement text it visualizes.

The `Requirement` column that used to live in this table is deliberately gone: the requirement's text
is defined once, in the specification file that owns it. Restating it here was the third copy.

## 1. The set

| Canvas | Covers | Blocks |
| --- | --- | --- |
| [`req-core-library.md`](./req-core-library.md) | `REQ-GEN-01/02`, `DRAFT-REQ-GEN-03`, `REQ-LIB-01` | 1 |
| [`req-mcp-server.md`](./req-mcp-server.md) | The ten committed `REQ-MCP-*` | 1 |
| [`req-mcp-tools.md`](./req-mcp-tools.md) | `REQ-MCP-ARC-01`…`07` committed; the draft analyzer tools and resource surface | 2 |
| [`req-cli.md`](./req-cli.md) | `REQ-CLI-03/04/05` committed; the seven draft toggles | 2 |
| [`req-draft-artifacts.md`](./req-draft-artifacts.md) | `DRAFT-REQ-GEN-04`, `DRAFT-REQ-ART-01`…`08` | 1 |

Seven blocks over 50 requirements. A single canvas carrying all of them is unreadable at normal zoom
— a 17-node version already rendered 3,250 px wide — so the set is partitioned by concern, and the
MCP tool requirements live on their own canvas rather than with the server's, because merged they
would put 20 requirement nodes on one canvas.

**Counts: 50 requirements — 23 committed, 27 draft.** Every committed requirement is implemented;
every draft one is not.

## 2. Why some elements appear on more than one canvas

`mcp_server` appears on three. The CLI canvas uses it to satisfy `REQ-CLI-03`, because the `--mcp`
flag is the CLI's contract. The MCP server canvas uses it to satisfy the `REQ-MCP-*` set, because
the server implementation is a separate concern from the flag that starts it. The MCP tools canvas
uses it because every tool is reachable over the protocol, and each tool therefore carries **two**
`satisfies` edges — one from `mcp_server` and one from the capability behind it (`core_archive` for
the `ARC` family, `analyzer_crates` for the analyzer families). That is why 50 requirements yield 62
relationships rather than 50.

## 3. Index

## Committed

| ID | Satisfied by | Canvas | Risk | Verify |
| --- | --- | --- | --- | --- |
| <a id="req-gen-01"></a>[`REQ-GEN-01`](../requirements.md#req-gen-01) | `zip_reader` | [Core library](./req-core-library.md) | high | test |
| <a id="req-gen-02"></a>[`REQ-GEN-02`](../requirements.md#req-gen-02) | `zip_reader` | [Core library](./req-core-library.md) | high | test |
| <a id="req-lib-01"></a>[`REQ-LIB-01`](../requirements.md#req-lib-01) | `core_library` | [Core library](./req-core-library.md) | medium | inspection |
| <a id="req-mcp-01"></a>[`REQ-MCP-01`](../requirements.md#req-mcp-01) | `mcp_server` | [MCP server](./req-mcp-server.md) | medium | inspection |
| <a id="req-mcp-02"></a>[`REQ-MCP-02`](../requirements.md#req-mcp-02) | `mcp_server` | [MCP server](./req-mcp-server.md) | medium | inspection |
| <a id="req-mcp-03"></a>[`REQ-MCP-03`](../requirements.md#req-mcp-03) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | test |
| <a id="req-mcp-04"></a>[`REQ-MCP-04`](../requirements.md#req-mcp-04) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | inspection |
| <a id="req-mcp-05"></a>[`REQ-MCP-05`](../requirements.md#req-mcp-05) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | test |
| <a id="req-mcp-06"></a>[`REQ-MCP-06`](../requirements.md#req-mcp-06) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | test |
| <a id="req-mcp-08"></a>[`REQ-MCP-08`](../requirements.md#req-mcp-08) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | test |
| <a id="req-mcp-09"></a>[`REQ-MCP-09`](../requirements.md#req-mcp-09) | `mcp_server` | [MCP server](./req-mcp-server.md) | high | test |
| <a id="req-mcp-10"></a>[`REQ-MCP-10`](../requirements.md#req-mcp-10) | `mcp_server` | [MCP server](./req-mcp-server.md) | medium | test |
| <a id="req-mcp-11"></a>[`REQ-MCP-11`](../requirements.md#req-mcp-11) | `mcp_server` | [MCP server](./req-mcp-server.md) | medium | test |
| <a id="req-mcp-arc-01"></a>[`REQ-MCP-ARC-01`](../requirements.md#req-mcp-arc-01) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="req-mcp-arc-02"></a>[`REQ-MCP-ARC-02`](../requirements.md#req-mcp-arc-02) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="req-mcp-arc-03"></a>[`REQ-MCP-ARC-03`](../requirements.md#req-mcp-arc-03) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="req-mcp-arc-04"></a>[`REQ-MCP-ARC-04`](../requirements.md#req-mcp-arc-04) | `core_archive` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="req-mcp-arc-05"></a>[`REQ-MCP-ARC-05`](../requirements.md#req-mcp-arc-05) | `core_archive` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="req-mcp-arc-06"></a>[`REQ-MCP-ARC-06`](../requirements.md#req-mcp-arc-06) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="req-mcp-arc-07"></a>[`REQ-MCP-ARC-07`](../requirements.md#req-mcp-arc-07) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="req-cli-03"></a>[`REQ-CLI-03`](../requirements.md#req-cli-03) | `mcp_server` | [CLI](./req-cli.md) | medium | test |
| <a id="req-cli-04"></a>[`REQ-CLI-04`](../requirements.md#req-cli-04) | `cli` | [CLI](./req-cli.md) | low | inspection |
| <a id="req-cli-05"></a>[`REQ-CLI-05`](../requirements.md#req-cli-05) | `cli` | [CLI](./req-cli.md) | low | inspection |

## Draft

A `DRAFT-` row is not traceability evidence. The tool is not required to satisfy it, and the
requirement may change or be withdrawn without notice. See
[`../draft/requirements.md`](../draft/requirements.md).

| ID | Satisfied by | Canvas | Risk | Verify |
| --- | --- | --- | --- | --- |
| <a id="draft-req-gen-03"></a>[`DRAFT-REQ-GEN-03`](../draft/requirements.md#draft-req-gen-03) | `scanner_framework` | [Core library](./req-core-library.md) | high | test |
| <a id="draft-req-gen-04"></a>[`DRAFT-REQ-GEN-04`](../draft/requirements.md#draft-req-gen-04) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | medium | test |
| <a id="draft-req-mcp-07"></a>[`DRAFT-REQ-MCP-07`](../draft/requirements.md#draft-req-mcp-07) | `mcp_server` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="draft-req-mcp-arc-08"></a>[`DRAFT-REQ-MCP-ARC-08`](../draft/requirements.md#draft-req-mcp-arc-08) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="draft-req-mcp-arc-09"></a>[`DRAFT-REQ-MCP-ARC-09`](../draft/requirements.md#draft-req-mcp-arc-09) | `core_archive` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="draft-req-mcp-anr-01"></a>[`DRAFT-REQ-MCP-ANR-01`](../draft/requirements.md#draft-req-mcp-anr-01) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="draft-req-mcp-app-01"></a>[`DRAFT-REQ-MCP-APP-01`](../draft/requirements.md#draft-req-mcp-app-01) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="draft-req-mcp-crs-01"></a>[`DRAFT-REQ-MCP-CRS-01`](../draft/requirements.md#draft-req-mcp-crs-01) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="draft-req-mcp-mem-01"></a>[`DRAFT-REQ-MCP-MEM-01`](../draft/requirements.md#draft-req-mcp-mem-01) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="draft-req-mcp-cpu-01"></a>[`DRAFT-REQ-MCP-CPU-01`](../draft/requirements.md#draft-req-mcp-cpu-01) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | medium | test |
| <a id="draft-req-mcp-tools-analysis"></a>[`DRAFT-REQ-MCP-TOOLS-ANALYSIS`](../draft/requirements.md#draft-req-mcp-tools-analysis) | `analyzer_crates` | [MCP tools](./req-mcp-tools.md) | high | test |
| <a id="draft-req-cli-01"></a>[`DRAFT-REQ-CLI-01`](../draft/requirements.md#draft-req-cli-01) | `cli` | [CLI](./req-cli.md) | high | test |
| <a id="draft-req-cli-02"></a>[`DRAFT-REQ-CLI-02`](../draft/requirements.md#draft-req-cli-02) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-06"></a>[`DRAFT-REQ-CLI-06`](../draft/requirements.md#draft-req-cli-06) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-07"></a>[`DRAFT-REQ-CLI-07`](../draft/requirements.md#draft-req-cli-07) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-08"></a>[`DRAFT-REQ-CLI-08`](../draft/requirements.md#draft-req-cli-08) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-09"></a>[`DRAFT-REQ-CLI-09`](../draft/requirements.md#draft-req-cli-09) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-10"></a>[`DRAFT-REQ-CLI-10`](../draft/requirements.md#draft-req-cli-10) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-cli-11"></a>[`DRAFT-REQ-CLI-11`](../draft/requirements.md#draft-req-cli-11) | `cli` | [CLI](./req-cli.md) | medium | test |
| <a id="draft-req-art-01"></a>[`DRAFT-REQ-ART-01`](../draft/requirements.md#draft-req-art-01) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | medium | inspection |
| <a id="draft-req-art-02"></a>[`DRAFT-REQ-ART-02`](../draft/requirements.md#draft-req-art-02) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | medium | test |
| <a id="draft-req-art-03"></a>[`DRAFT-REQ-ART-03`](../draft/requirements.md#draft-req-art-03) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | high | test |
| <a id="draft-req-art-04"></a>[`DRAFT-REQ-ART-04`](../draft/requirements.md#draft-req-art-04) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | medium | test |
| <a id="draft-req-art-05"></a>[`DRAFT-REQ-ART-05`](../draft/requirements.md#draft-req-art-05) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | high | test |
| <a id="draft-req-art-06"></a>[`DRAFT-REQ-ART-06`](../draft/requirements.md#draft-req-art-06) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | high | test |
| <a id="draft-req-art-07"></a>[`DRAFT-REQ-ART-07`](../draft/requirements.md#draft-req-art-07) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | high | test |
| <a id="draft-req-art-08"></a>[`DRAFT-REQ-ART-08`](../draft/requirements.md#draft-req-art-08) | `artifact_writer` | [Draft artifacts](./req-draft-artifacts.md) | medium | test |

## 4. Notation

- `requirement` — functional behaviour the tool must exhibit.
- `performanceRequirement` — a bound on resource usage: archive size, entry count, result caps.
- `designConstraint` — a rule constraining how the system is built rather than what it does.
- `interfaceRequirement` — part of a user-facing contract: the command line, or the MCP tool surface
  an agent calls.
- `satisfies` — the element implements the requirement.

## 5. Not depicted

`classes.md`, `sequences.md`, and `activities.md` were removed during the redesign and have not been
redrawn. They described the previous implementation's function names (`scan_anrs()`,
`open_archive()`, `run_subcommand()`) and crate layout (`subcommand-anr`, `subcommand-support`), none
of which matches the committed design, and they carried no element IDs and no requirement mapping —
which is part of why they drifted.

`activities.md` stays deleted: no committed workflow exists to depict, because "serve MCP over
stdio" is a state, not a workflow. If they are redrawn, their elements take `CLS-*`, `SEQ-*`, and
`ACT-*` IDs with rows in [`../traceability-matrix.md`](../traceability-matrix.md). Until then, a
committed requirement can have no class or sequence diagram — `REQ-GEN-01` and `REQ-GEN-02` are
committed and have neither.
