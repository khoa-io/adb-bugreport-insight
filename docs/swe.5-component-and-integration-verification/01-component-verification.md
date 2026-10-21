# Component verification

The [MCP design](../swe.3-detailed-design/03-mcp-server.md) connects `ARCH-MCP-TOOLS-ARC` and `ARCH-MCP-SRV` to `ARCH-ARCHIVE`.
Checks invoke registry/handlers with shared synthetic archives, excluding transport/CLI startup.

All named measures are in [crates/mcp/tests/tools.rs](../../crates/mcp/tests/tools.rs).

| Measure | Boundary and expected assertion |
| --- | --- |
| `registry_returns_exactly_the_seven_committed_tools`, `tool_names_are_unique`, `tools_list_matches_the_committed_set` | Registry/tool definitions contain the committed seven names without duplicates |
| `every_description_states_a_cost_class_and_a_bound`, `unknown_parameters_are_rejected` | Descriptions carry cost/bound guidance; typed parameters reject unknown fields |
| `envelope_always_carries_the_four_bound_fields`, `stopped_reason_uses_the_agreed_snake_case_vocabulary` | All seven successful handlers include the common bound fields with expected types; a returned stop reason uses the agreed vocabulary |
| `success_reports_ok`, `zero_matches_reports_empty_not_ok`, `missing_entry_reports_not_found`, `beyond_eof_range_reports_empty_partial_content` | Successful, empty and absent results retain their meanings; EOF returns explicitly partial available content |
| `corrupt_entry_is_never_reported_as_empty`, `not_found_is_reserved_for_a_missing_entry` | Decode failure is distinct from successful empty and missing entry |
| `archive_info_reports_the_bound_archive`, `archive_info_omits_the_digest_when_declined` | Identity is for the bound archive; declining digest does not manufacture one |
| `list_entries_honours_a_path_prefix`, `list_entries_reports_what_it_omitted`, `find_entries_matches_paths_without_reading_content` | Index queries respect scope and report item truncation |
| `find_entries_refuses_regex_rather_than_downgrading`, `find_entries_reports_an_invalid_glob` | Unsupported or malformed patterns are typed refusals |
| `entry_info_reports_detection_with_its_basis`, `find_by_type_filters_on_detected_content_type`, `find_by_type_rejects_an_unknown_type` | Detection reports basis and type filtering refuses unknown types |
| `every_grep_match_carries_a_byte_offset`, `grep_archive_honours_an_entry_scope`, `coverage_is_reported_relative_to_the_requested_scope` | Search returns locators and measures entry coverage against caller scope |
| `a_stalled_scan_reports_its_coverage_gap`, `the_empty_result_has_the_same_shape_as_a_successful_one` | Coverage fields exist on a completed miss; hit/miss results retain the same payload shape |
| `a_stalled_scan_is_never_reported_as_empty` | If the fixture produces truncation, status is not empty and notes explain it |
| `read_entry_rejects_both_range_pairs`, `read_entry_rejects_neither_range_pair`, `read_entry_returns_lines_with_offsets`, `read_entry_by_byte_range_caps_the_excerpt` | Both/neither pairs are rejected; a two-line result carries offsets; a 25-byte read reports excerpt bytes <= 512 |
| `reading_tools_do_not_mutate_the_fixture`, `archive_status_is_serialised_snake_case` | Read paths preserve fixture length; statuses serialize to the four specified values |

Run after the [core measures](../swe.4-unit-verification/01-verification-measures.md):

```bash
cargo test -p bugreport-insight-mcp --test tools --offline
```

The [fixture guide](../verification/02-fixtures-and-environments.md) supplies prerequisites.
The suite checks representative statuses; it does not claim every four-status-by-seven-tool combination.
Conditional stalled-scan assertions do not prove every stop mechanism ran.
Byte-range coverage does not establish the 512-byte line-result cap.
The 25-byte read test cannot exercise 512-byte clipping, above-cap requests or UTF-8 replacement expansion.
The targeted excerpt, pagination and outcome checks below cover the additional bounds and wire behaviors.

## Insight component boundary

The reusable core generation API is exercised by [domain checks](../../crates/core/src/insight/tests.rs) and [reference acceptance](../../crates/core/tests/insight_reference.rs).
The CLI consumes that API through [insight process checks](02-integration-and-verification.md#insight-process).
Together they cover analyzer-to-model validation, artifact rendering/publication and process outcome mapping for the three insight architecture elements.
See the [result](../../verification/results/insight-generation.md); the unchanged MCP handlers retain the measures above.

## Search pagination components

[pagination.rs tests](../../crates/mcp/src/tools/pagination.rs) verify the eight-session ceiling, idle expiry, single-use cursor replay refusal, preservation of a pending locator when a too-small response budget is retried with a larger budget, and propagation of the selected archive entry limit while page limits remain independent.
Run `cargo test -p bugreport-insight-mcp --lib --offline`; [executed results](../../verification/results/search-pagination.md) own the verdict.

## Collection outcomes and detection failures

[Tool-layer tests](../../crates/mcp/tests/tools.rs) collection_misses_are_empty_but_zero_output_limits_are_partial_ok, type_queries_preserve_failures_and_distinguish_unknown_from_unreadable, type_failures_and_matches_have_separate_bounded_omission_counts, all_failed_and_directory_only_type_queries_have_honest_outcomes and type_coverage_describes_samples_rather_than_full_file_validation verify [ADR 0010](../decisions/0010-empty-results-and-detection-failures.md).
They exercise clean misses, clipped matches, partial/all-failed scopes, directories/empty ZIPs, 205 failures capped to 200 details, and successfully classified 8 KiB samples with uninspected non-text tails.
Run `cargo test -p bugreport-insight-mcp --test tools --offline`; [execution evidence](../../verification/results/empty-and-detection-outcomes.md) records actual verdicts.
Core detection heuristics and readers are unchanged, so existing core suites provide regression coverage rather than new core-unit behavior.

## Range boundaries and subtree filters

[ADR 0012 evidence](../../verification/results/range-boundaries.md) records core partial-read/strict-read and safety checks, component EOF/reversed/empty/unterminated cases, subtree segment boundaries and the real stdio flow.
The measures are [partial_ranges.rs](../../crates/core/tests/partial_ranges.rs), range/subtree cases in [tools.rs](../../crates/mcp/tests/tools.rs) and eof_ranges_and_subtree_boundaries_work_over_stdio in [protocol.rs](../../crates/cli/tests/protocol.rs).

## Content IO confirmation components

[Registry](../../crates/mcp/src/registry.rs) wraps the five content handlers before execution; the two index queries remain immediate.
Existing tool behavior helpers explicitly accept returned previews on the same registry instance before assessing prior content outcomes; this test-only acceptance is not production behavior.
The gate/wrapper unit checks above and raw [stdio checks](02-integration-and-verification.md#confirmation-wire-exchanges) separately establish request binding and absence of pre-approval execution.
Descriptions and typed confirmation fields are checked with the existing tool-surface measures; [execution evidence](../../verification/results/content-io-confirmation.md) records review of actual work and bounds.
