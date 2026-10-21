# MCP insight resources — 2026-10-17

The user's follow-up, “Don't forget that the insight should be accessible via MCP”, promotes `DRAFT-REQ-MCP-07` to `REQ-MCP-07` and `DRAFT-ARCH-MCP-RES` to `ARCH-MCP-RES`.
TASK016 completes the six SWE concerns for read-only access to the seven generated artifacts.
The [generation execution](insight-generation.md) remains historical evidence for TASK015; analyzer MCP tools remain deferred.

## Tree, environment and fixtures

Base revision is `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus the preceding uncommitted insight implementation and this resource provider/adapter/tests/docs change.
Platform is macOS on Apple silicon, with Rust/Cargo 1.99.0 and offline cached rmcp 3.5.0.
Resource traits, constructors, error codes and protocol metadata were checked against that local SDK source before implementation.
The provider tests generate synthetic artifacts through core; wire tests generate them through the real CLI.
The real trial uses the existing seven-file output for `tmp/reference-archive.zip`, SHA-256 `10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92`.
Private report content, the manual client script and detailed local logs remain under ignored `tmp/`.
No project files were staged or committed; pre-existing work was preserved.

## Design and selected measures

[Provider design](../../docs/swe.3-detailed-design/03-mcp-server.md#insight-resource-provider) defines the fixed sibling paths, SHA-bound URI whitelist, exact UTF-8 text, 64 MiB per-file ceiling, identity validation and errors.
[Integration measures](../../docs/swe.5-component-and-integration-verification/02-integration-and-verification.md#mcp-insight-resources) identify all executable checks.
`InsightResources` is protocol-free; the adapter adds resources alongside the seven unchanged tools and runs filesystem/JSON work through `spawn_blocking`.
Resources never generate or modify files.
Missing output lists no resources, while publication during a running session becomes visible on the next list call.

| REQ-MCP-07 clause | Demonstrating assertion/review | Verdict |
| --- | --- | --- |
| Seven discoverable generated artifacts; MIME, size and SHA-bound URI | `complete_set_returns_exact_bytes_mimes_sizes_and_identity_without_writes`; real resource process check; reference trial below. | Passed. |
| Exact complete text rather than summaries/prefixes | Provider and wire compare every returned resource byte to its generated file; full reference resource comparison. | Passed. |
| Bound archive and fixed filename namespace | System/selected JSON and README identity tests; rejection of unknown, foreign, encoded traversal, query and filesystem URIs. | Passed. |
| Explicit absence, incomplete/malformed/non-UTF-8/symlink/oversized failures | Eight provider checks, missing-resource wire check and source review of metadata plus actual `take(limit + 1)` guards. | Passed. |
| Read-only access and no automatic generation | Byte/mtime comparisons for input and output; absence creates no sibling directory; later publication is performed by a separate CLI invocation. | Passed. |
| Resources capability, unchanged tools and protocol behavior | Capability test, both resource process checks, full existing archive/MCP regressions; no resource subscriptions/list-change capability. | Passed. |

The source inspection confirms that accepted names originate only from the seven-file constant, actual reads are bounded and no provider code invokes generation or writes files.
The 64 MiB limit measures raw UTF-8 file bytes; JSON-RPC serialization overhead is additional.
The ceiling is a read bound, not a claim about total process peak memory.

## Executed checks

| Command/method | Result |
| --- | --- |
| `cargo test -p bugreport-insight-mcp --offline --test resources` | Eight provider checks passed. |
| `cargo test -p bugreport-insight-cli --offline --test protocol` | 24 process checks passed, including two resource flows and the updated capability assertion. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo build --workspace --offline` | Passed. |
| `cargo test --workspace --offline` | 147 passed, 0 failed/ignored: 39 core unit, 20 hostile, 19 original reference, 1 insight reference, 35 MCP handler, 8 resource provider, 24 protocol process and 1 insight domain process. |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Passed. |
| `python3 scripts/check_docs.py` | Passed; current links/anchors checked after the documentation updates. |
| `python3 scripts/diagrams.py render` and `check` | Passed; two architecture views regenerated, all nine SVGs fresh. |
| Raster preview review of component/runtime SVGs | Labels and messages readable; resource read and generation paths distinguishable. |
| `git diff --check` | Passed. |
| Manual real-stdio client against `bugreport-insight --mcp tmp/reference-archive.zip` | Listed/read every artifact, compared exact bytes and MIME/size, verified seven tools, exit 0 and unchanged source/output hashes/mtimes. |

Iteration found an SDK discriminator difference: requesting the no-initialize 2026 version through the legacy initialize flow does not establish that protocol version.
The wire checks now deliberately negotiate the supported 2025-06-18 and 2025-11-25 handshake versions instead of asserting a discriminator the SDK correctly omits for legacy clients.
Resource responses explicitly publish private cache scope and zero TTL.
A Clippy default-field reassignment warning was corrected using the SDK's `with_all_items` constructor; final workspace tests and lint were rerun successfully.

## Real-reference resource trial

The client negotiated 2025-11-25 and used `resources/list`, then `resources/read` for each returned SHA-bound URI.
All text matched the files byte for byte:

| Artifact | Raw bytes | MIME |
| --- | --- | --- |
| README.md | 9,200 | text/markdown |
| system.json | 2,087,203 | application/json |
| apps.json | 2,358,732 | application/json |
| anr.json | 688,477 | application/json |
| crashes.json | 1,191,565 | application/json |
| memory.json | 783,730 | application/json |
| cpu.json | 635,659 | application/json |

Input archive SHA-256/size/nanosecond mtime and every artifact's SHA-256/size/mtime were identical before and after MCP access.
The server still listed exactly seven archive tools and exited zero after stdin EOF.
No artifact generation or output write occurred in this MCP session.

## Boundaries

External archive/artifact mutation is not snapshot-isolated; the provider rechecks the selected file and identity, and the tested static symlinks are refused.
Linux/Windows execution, hosted CI and independent external review were not performed; local provider/process checks and the macOS real-reference trial supply this targeted evidence.
The earlier 23-requirement conformance review and recorded archive/MCP discrepancies remain separate work under SWE.6-T1.
No selected TASK016 check remains unexecuted.
