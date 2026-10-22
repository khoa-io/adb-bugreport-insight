# Integration sequence and verification

Integrate in dependency order: `ARCH-ARCHIVE` → `ARCH-MCP-TOOLS-ARC` and `ARCH-MCP-SRV` → `ARCH-CLI-FLAGS`.
Establish [core](../swe.4-unit-verification/01-verification-measures.md) → [handler](./01-component-verification.md) → real executable checks.
The [runtime design](../swe.2-architecture/03-runtime-and-interactions.md) defines startup and EOF shutdown.

## Real process measures

[protocol.rs](../../crates/cli/tests/protocol.rs) owns all measures; Cargo supplies `CARGO_BIN_EXE_bugreport-insight`.
The `Server` helper launches the binary with separate stdin, stdout, and stderr pipes; `initialize` sends the handshake and initialized notification.

| Measures | Expected assertion |
| --- | --- |
| `initialize_succeeds_over_real_stdio`, `tools_and_read_only_resources_capabilities_are_advertised` | Handshake reports the server name and protocol version; tools/resources capabilities are present without prompts/logging or resource subscriptions |
| `instructions_name_every_tool_and_stay_short` | Instructions are nonempty, name all seven tools, and stay within 40 lines |
| `tools_list_returns_exactly_the_seven_committed_tools`, `every_tool_declares_a_closed_schema` | Wire list contains exactly the seven names as a set; schemas have root object type and reject additional properties |
| `all_seven_tools_answer_over_stdio` | Each tool returns `ok` and all four correctly typed bounding fields, with no error body, on its synthetic happy path |
| `excerpt_cap_supports_explicit_byte_follow_up_and_repeatable_grep`, `sanitized_excerpt_follow_up_uses_source_bytes_instead_of_emitted_bytes` | Real line/byte/grep excerpts cap at 512 emitted bytes; cafe prefix/tail are 512/88, repeated requests repeat results, source progress survives NUL expansion, and clipping reaches the envelope without inventing omitted matches |
| `archive_info_reports_marker_states_and_allows_source_follow_up` | Valid/missing/oversized/invalid/NUL/unreadable marker states serialize over real stdio; unavailable values remain null while other metadata/session survive; source IDs support read_entry, with a corrupt-fixture line read producing a domain error; partial byte reads do not certify CRC |
| `line_read_preserves_utf8_text_over_stdio` | A real read_entry line request preserves multilingual text, Utf8 classification, source-byte count, full line length and offset through JSON serialization |
| `partial_success_retains_bounding_fields_without_becoming_an_error` | A capped listing remains successful, retains the four fields and reports the exact omitted item count |
| `stdout_carries_json_rpc_and_nothing_else`, `verbose_adds_stderr_detail_only` | Captured nonblank stdout lines parse as JSON-RPC/JSON; verbose diagnostics reach stderr |
| `client_disconnect_exits_zero` | Closing stdin causes successful process termination |
| `domain_failure_is_a_tool_error_not_a_protocol_error` | Missing-entry and unreadable-range failures use isError with stable code/message/address/status, no successful envelope fields, and subsequent successful calls preserve the session |
| `unknown_parameter_is_rejected_with_a_reason`, `both_range_pairs_is_a_protocol_error`, `an_unavailable_pattern_mode_is_refused_over_the_wire`, `unknown_tool_is_rejected` | SDK parameter failure uses isError; local range conflict uses JSON-RPC error without a successful result or completeness fields; unsupported pattern uses `-32602`; unknown tool is refused |
| `a_full_tool_sweep_leaves_the_archive_unchanged` | Exact fixture bytes before and after all seven calls match |
| `an_unopenable_archive_never_starts_a_server`, `missing_archive_is_a_usage_error`, `no_mode_selected_is_a_usage_error` | Bad ZIP exits nonzero before protocol output; missing archive/mode exits 2 with no stdout |
| `help_goes_to_stdout_and_exits_zero` | Help exits successfully and names the binary and flags |

```bash
cargo test -p bugreport-insight-cli --test protocol --offline
```

Run from the repository root.
No private archive is needed.
Record actual execution separately under [results](../verification/index.md).

## Evidence limits

Blocking reads/waits lack per-exchange timeouts; passing runs cannot prove bounded response time.
stdout checks parse captured nonblank lines; they do not assert every byte.
Immutability checks compare archive bytes, excluding mtime and other filesystem side effects.
The tool-name test compares sorted name lists, verifying exact membership and uniqueness independently of discovery order under ADR 0011.
The separate closed-schema test checks every discovered tool's description and input schema; [executed discovery evidence](../../verification/results/unordered-tool-discovery.md) records the targeted verdict.
Instructions tests do not individually assert each cost, bound, intended ordering, or absence of archive content; inspect `INSTRUCTIONS` in [adapter.rs](../../crates/mcp/src/adapter.rs) for those clauses.

The [real-case reference workflow](../../verification/results/real-case-user-validation.md) checks source locators, complete resources and archive/output SHA-256 and mtime preservation separately from synthetic process checks.
Full requirement verdicts remain SWE.6 work.

## Insight process

`ARCH-ARCHIVE` → `ARCH-INSIGHT-ANALYZERS` → `ARCH-INSIGHT-WRITER` → `ARCH-CLI-INSIGHT` is the additional dependency sequence.
[Domain process check](../../crates/cli/tests/insight.rs) `insight_cli_analyzes_all_domains_and_preserves_input_bytes_and_metadata` runs the real binary with a whitespace-containing archive filename and verbose diagnostics.
It asserts seven files, empty stdout, stderr progress, parsed findings across all six domains, bounded README, unchanged input bytes and unchanged mtime.

[Protocol process checks](../../crates/cli/tests/protocol.rs) add `insight_writes_the_fixed_artifact_set_and_preserves_the_archive`, `insight_rejects_mcp_and_positional_archive_combinations`, and `insight_rejects_non_zip_without_publishing_output`.
They assert collision preservation, exit codes, missing/bad input behavior, fixed filenames and absence of partial output.
The existing real-stdio checks run as regressions because CLI mode selection changed; the MCP interface itself did not change.

Run `cargo test -p bugreport-insight-cli --offline` after core checks.
The [reference acceptance](../../crates/core/tests/insight_reference.rs) and manual real-report CLI trial supplement these deterministic process checks.
Results and targeted requirement verdicts are in the [execution record](../../verification/results/insight-generation.md).

The process regression `insight_cli_publishes_when_meminfo_has_no_recognized_summary` exercises `ARCH-INSIGHT-ANALYZERS` through `ARCH-CLI-INSIGHT` with an unfamiliar meminfo section.
It requires seven published files, an accurate section locator, null RAM, empty stdout and unchanged archive bytes/mtime; see the [missing-memory execution record](../../verification/results/missing-memory-provenance.md).

## MCP insight resources

`ARCH-MCP-RES` provider checks in [resources.rs](../../crates/mcp/tests/resources.rs) run before the CLI-consuming layer.
Checks assert exact bytes/MIME/size/identity, unchanged source/output, no-write absence and late publication, incomplete/foreign sets, selected JSON/README identity, malformed/non-UTF-8 content, size refusal, exact URI rejection and symlink refusal. The descriptor-level race test replaces the checked directory and a leaf with symlinks before open; pinned directory handles and no-follow opens keep reads inside the checked sibling tree.
[Protocol process checks](../../crates/cli/tests/protocol.rs) `mcp_resources_return_all_generated_artifacts_byte_for_byte` and `mcp_missing_resources_are_empty_without_generating_files_and_refresh_after_publication` generate through the actual CLI, then use MCP resource discovery/reads.
They check every file's complete text and MIME/size, private zero-TTL response metadata, absence without generation, unknown-resource/cursor errors, late publication and the unchanged seven-tool list.
The existing capability check now expects tools plus read-only resources, with no subscriptions/list-change notifications.
Wire checks exercise 2025-06-18 and 2025-11-25 handshake clients; the SDK handles negotiated result metadata.
Run `cargo test -p bugreport-insight-mcp --test resources --offline` then `cargo test -p bugreport-insight-cli --test protocol --offline`.
The [execution record](../../verification/results/mcp-insight-resources.md) records fresh verdicts and the real reference-archive trial.

## Budgeted search pages

[Protocol tests](../../crates/cli/tests/protocol.rs) grep_pages_fit_wire_budgets_and_preserve_every_match, grep_empty_pages_continue_and_invalid_budgets_preserve_cursor and grep_reports_long_line_and_encrypted_coverage_gaps_over_stdio exercise the actual CLI/MCP surface.
They measure serialized result bytes for old and new protocol versions, compare all matching lines across pages, sum source consumption to prove no replay, preserve archive bytes, and check empty-page continuation, invalid-input retry and explicit coverage gaps.
Run `cargo test -p bugreport-insight-cli --test protocol --offline`; [executed results](../../verification/results/search-pagination.md) bind these checks to the accepted contract.

## Empty and partial classification wire outcomes

[Protocol test](../../crates/cli/tests/protocol.rs) empty_collections_and_partial_detection_failures_are_distinct_over_stdio launches the shipped CLI and checks empty listings/path/type queries, encrypted-file partial classification, Unknown versus unreadable, direct-entry domain errors and session continuity.
It also confirms archive bytes stay unchanged.
Run `cargo test -p bugreport-insight-cli --test protocol --offline`; [executed results](../../verification/results/empty-and-detection-outcomes.md) map the assertions to REQ-MCP-06/08 and REQ-MCP-ARC-06.

## Range boundaries and subtree filters

[ADR 0012 evidence](../../verification/results/range-boundaries.md) records core partial-read/strict-read and safety checks, component EOF/reversed/empty/unterminated cases, subtree segment boundaries and the real stdio flow.
The measures are [partial_ranges.rs](../../crates/core/tests/partial_ranges.rs), range/subtree cases in [tools.rs](../../crates/mcp/tests/tools.rs) and eof_ranges_and_subtree_boundaries_work_over_stdio in [protocol.rs](../../crates/cli/tests/protocol.rs).

## Confirmation wire exchanges

[Protocol tests](../../crates/cli/tests/protocol.rs) every_content_operation_previews_cost_before_execution_over_stdio and confirmations_bind_tool_budgets_cursor_pages_and_resource_uris exercise REQ-MCP-12 without automatic helper approval.
They remove the archive after startup and verify all five content tools and both resource methods still return previews, while index listing works; an approved identity request then fails to read the missing archive and its token cannot be replayed.
The no-IO challenge case exercises both 2025-06-18 and 2025-11-25 clients.
Changed grep budgets and resource URIs reject approval without consuming the original token; repeated cursor previews leave the search cursor available for confirmed execution, and the confirmed continuation is single-use.
Existing process tests use a clearly marked helper that explicitly accepts previews before testing prior content/resource behavior; request_once keeps confirmation tests independent of that helper.
Run cargo test -p bugreport-insight-cli --test protocol --offline; [execution record](../../verification/results/content-io-confirmation.md) records results and cost-estimate limitations.

## SDK task-failure boundary

The real rmcp service and test-only fault transports exercise both outer receive-task panics and nested send-task panics through the same supervisor used by stdio production startup.
The resulting ServeError::ServerTask is separately mapped to exit 1 by the CLI unit assertion; the normal launched-CLI protocol suite remains the regression check for EOF, request recovery and Unix response-pipe failures.
This composed boundary evidence does not inject a panic into the released executable or demonstrate a native non-Unix response-pipe failure.
[The executed correction](../../verification/results/server-task-failures.md) records the failing nested-task case and current results under REQ-CLI-03/05 and ARCH-MCP-SRV/ARCH-CLI-FLAGS.

## Directory archive tool wire checks

REQ-MCP-14 process verification is pending.
The directory-mode protocol exchange must discover ten unique tools, inspect each archive schema for optional `bugreport`, select reports returned by `list_bugreports`, and exercise at least one index query and one confirmed content query per report.
It must show that results identify their report, entry IDs and grep continuation remain report-local, a cursor cannot switch reports, ambiguous or truncated discovery requires selection, a bound-file selector mismatch is rejected, same-name context reopening cannot reuse a confirmation token, and archive bytes remain unchanged.
The existing directory tool-list assertion has been updated for ten names; it has not been executed for this change.

## Raised limits and combined MCP insight

The real CLI process checks `force_admits_large_entry_count_and_explains_default_rejection` and combined-mode protocol tests exercise CLI → core admission and CLI → MCP → core analysis.
The force fixture contains 10,001 stored empty entries: default MCP startup and static insight reject with a measured limit and retry guidance; the forced runs succeed with a stderr warning while preserving ZIP bytes and modification time.
Combined-mode checks must verify advertised tools, request-bound confirmation, named-bugreport selection, artifact reads, unchanged source files and no generated sibling output.
Directory checks include parent, nested and symlink candidates and caller attempts to address outside the discovered set.
The [execution record](../../verification/results/force-and-scoped-insight-mcp.md) records actual commands, results and verification limits.
