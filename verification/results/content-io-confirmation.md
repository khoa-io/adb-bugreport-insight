# Content IO confirmation verification

Execution and review date: **2026-10-19**; the filename retains the accepted decision's 2026-10-18 date and existing incoming links.
Tested revision: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus the existing uncommitted TASK015–TASK019 implementation, including `confirmation.rs`, registry wrappers, adapter resource gates and protocol tests.
Environment: macOS, `aarch64-apple-darwin`, Rust/Cargo 1.99.0, offline Cargo dependencies.
This increment records previously missing evidence; it changes no product behavior.

## Scope and selected measures

The accepted contract is [ADR 0013](../../docs/decisions/0013-confirmation-before-content-io.md), [REQ-MCP-11/12](../../docs/swe.1-requirements/02-software-requirements.md#req-mcp-11), and the [confirmation protocol and cost scope](../../docs/swe.3-detailed-design/03-mcp-server.md#content-io-confirmation).
Affected architecture is ARCH-MCP-SRV, ARCH-MCP-TOOLS-ARC and ARCH-MCP-RES; existing archive/content-reader contracts remain applicable.
REQ-MCP-04/05/06/07 also supply the initialization, discovery, tool and resource surfaces exercised here.
Synthetic ZIP fixtures keep the refusal, lifecycle and wire checks deterministic and avoid publishing real bugreport content.
The workspace run additionally exercises the existing private reference-archive suites.

| Level and measure | Expected result | Actual result |
| --- | --- | --- |
| SWE.4: `tokens_bind_operation_and_arguments_without_consuming_on_mismatch` | Changed operation/arguments refuse execution and preserve the original token; matching retry succeeds once; replay fails | Pass |
| SWE.4: `tokens_expire_and_capacity_is_bounded` | A token at 300 seconds is refused; 64 pending requests fill the pool and the next preview is refused | Pass; expiry is simulated by changing the unit's recorded issuance instant |
| SWE.4: `oversized_arguments_do_not_retain_state_or_consume_existing_approval` | Arguments over 65,536 serialized JSON bytes fail without retaining requests or consuming a valid existing token | Pass |
| SWE.4: `tokens_do_not_cross_gate_instances` | A token from another gate cannot approve execution | Pass |
| SWE.5: `wrapped_handler_runs_only_after_exact_confirmation` | A counting handler is never entered for preview/replay; matching approval enters it once, even when it subsequently fails | Pass |
| SWE.5/6: `every_content_operation_previews_cost_before_execution_over_stdio` | All five tools and both resource methods return challenges before content/provider work, for protocol versions 2025-06-18 and 2025-11-25 | Pass; after initialization the ZIP path is removed, index listing still works, and previews still succeed; an approved archive_info reaches the missing-file failure and replay is refused |
| SWE.5/6: `confirmations_bind_tool_budgets_cursor_pages_and_resource_uris` | Changed scan budgets and resource URIs fail; restoring original arguments permits execution; each continuation needs approval and preview does not consume its cursor | Pass |
| SWE.5: `every_description_states_a_cost_class_and_a_bound` and `instructions_name_every_tool_and_stay_short` | All seven descriptions state their work/bound, and initialization names all tools within 40 lines | Pass in workspace suite; semantic accuracy additionally reviewed below |

Focused commands passed: `cargo test --offline -p bugreport-insight-mcp confirmation` (5 tests), `cargo test --offline -p bugreport-insight-cli --test protocol confirm` (1 test), and `cargo test --offline -p bugreport-insight-cli --test protocol every_content_operation_previews_cost_before_execution_over_stdio` (1 test).

## Actual IO and guidance review

Source review compared [CLI help](../../crates/cli/src/main.rs), [initialization/resource guidance](../../crates/mcp/src/adapter.rs), [tool descriptions](../../crates/mcp/src/tools/handlers.rs), [cost previews and gate](../../crates/mcp/src/confirmation.rs), and the [resource provider](../../crates/mcp/src/resources.rs) against the archive identity, detection, reader and search implementations.
The [runtime IO description](../../docs/swe.2-architecture/03-runtime-and-interactions.md#shared-state-and-actual-io) agrees with these operations.
CLI help describes the operating modes without asserting cheaper IO than they perform; initialization and tool/resource guidance explain the approval exchange and work after approval.

| Operation | Reviewed cost and refusal boundary |
| --- | --- |
| Startup, initialize, tools/list, list_entries, find_entries | Startup admits ZIP metadata; discovery and index queries do not hash or decompress content and require no confirmation |
| archive_info | Preview uses cached index metadata; after confirmation identity may hash the full stored ZIP and rereads two marker entries, allowing 256 bytes plus an overflow probe per marker (514 source bytes total) |
| entry_info | After confirmation obtains a fresh ZIP reader and at most an 8 KiB detection sample |
| find_by_type | After confirmation attempts detection for every file; the preview permits 8 KiB per file and does not confuse match-output limits with classification work |
| read_entry | After confirmation can decompress the skipped prefix; preview allows the 256 MiB entry ceiling plus overflow probe, independently of 512-byte emitted excerpts |
| Each grep_archive page | Gate precedes handler/cursor access; preview uses the requested scan_budget capped at 32 MiB, with independent confirmation for subsequent pages |
| resources/list | Gate precedes provider delegation; potential work includes full ZIP hashing, marker reads and a complete system.json manifest, with a 64 MiB accepted manifest ceiling |
| resources/read | Gate binds the exact URI before provider delegation; potential work additionally includes a complete requested artifact with a 64 MiB accepted ceiling; system.json can be read twice |

No execution is automatically approved by production code; test helpers that retry explicitly are separate from raw gate and wire checks.
Tool wrappers share one 64-request gate and resource methods share a separate gate; inspection confirms that each gate is checked before its corresponding handler/provider is called.
The normalized tool arguments include typed adapter defaults and exclude confirmation; resource binding includes the exact read URI.
The metadata-only plans deliberately overestimate cached/short/early-failure work and describe hash input, decoded source bytes, and accepted artifact sizes rather than physical IO or elapsed time.
Resource reads can consume one extra overflow-probe byte when detecting a concurrently growing oversized artifact; the 64 MiB fields describe accepted sizes, not an all-filesystem-read upper bound.
The first successful archive digest choice is cached, including declining hashing; concurrent uncached calls can compute before one value wins, and resources can reopen after an earlier decline.

## Regression gates and limits

| Command | Expected result | Actual result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Clean workspace formatting | Pass |
| `cargo build --workspace --offline` | All workspace crates build | Pass |
| `cargo test --workspace --offline` | All current workspace regressions pass | Pass, 198 tests, zero failures/ignored tests; CLI protocol 39, CLI unit/insight 2, core unit 45, core integrations 62, MCP unit 7, MCP resources 8, MCP tools 42 |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | No warnings across targets | Pass |

No source or diagram changes were necessary during this verification increment.
Full requirement-by-requirement qualification remains separate from this targeted REQ-MCP-11/12 evidence.
The checks establish challenge ordering, lifecycle and observed integration behavior; they do not measure exact physical reads, CPU time or elapsed cost, and they do not authenticate a human client's consent.
Search-cursor idle expiry remains independent of confirmation expiry; existing pagination measures cover its lifecycle rather than treating a preview as a cursor reservation.
