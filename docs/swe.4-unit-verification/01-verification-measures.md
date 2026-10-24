# Unit verification measures

These measures inspect the [archive contract](../swe.3-detailed-design/01-core-archive-contract.md), [types](../swe.3-detailed-design/02-core-archive-types.md), and [safety design](../swe.3-detailed-design/04-archive-safety.md).
Cargo test layout does not determine ASPICE allocation: direct core checks supply unit/design evidence; process checks belong to SWE.5.

## Selected executable checks

Links identify modules; names identify executable measures.

| Units and contract | Executable measure | Asserted outcome |
| --- | --- | --- |
| `glob::glob_is_match`, segment matching | [glob tests](../../crates/core/src/glob.rs): `star_does_not_cross_separator`, `double_star_is_the_only_separator_crossing_construct`, `malformed_patterns_are_typed_errors` | `*` stays in one path segment, `**` can cross separators, malformed patterns produce typed errors |
| `EntryReader::read_line`/`read_line_span`, bounded retention | [hostile tests](../../crates/core/tests/hostile.rs): `line_bomb_reports_true_length_and_stays_bounded`, `line_bomb_retention_is_independent_of_line_length`, `line_at_exactly_the_cap_is_complete`, `skip_past_a_bomb_line_still_lands_correctly` | Overlong line reports true length and a stopped reason; retained bytes stay bounded; exact-cap line is complete; later lines remain addressable |
| `Archive::open_with`, limit validation | [hostile tests](../../crates/core/tests/hostile.rs): `entry_over_the_declared_ceiling_is_rejected_at_open`, `too_many_entries_is_rejected_at_open`, `compression_ratio_limit_is_checked_at_open`, `nonsensical_limit_sets_are_rejected_at_open` | Declared limits reject at open with typed errors; contradictory custom limits fail |
| Reader streaming guard | [Lazy-validation tests](../../crates/core/tests/lazy_validation.rs): `lying_declared_size_is_accepted_at_open_but_rejected_when_read`, `skipped_bytes_and_lines_count_toward_the_read_ceiling`; [hostile tests](../../crates/core/tests/hostile.rs): `take_bounded_overflow_is_an_error_not_a_truncation` | Lying size passes metadata admission but produced bytes fail the limit; skipped and retained bytes count across read paths |
| Sequential chunk safety | [Lazy-validation tests](../../crates/core/tests/lazy_validation.rs): `chunk_stream_preserves_every_byte_for_stored_and_deflated_entries`, `chunk_stream_reports_overflow_instead_of_eof_and_accepts_exact_limit` | Exact payload sequence across chunks; exact ceiling succeeds, excess bytes error with typed source |
| Lazy admission and evidence boundary | [Lazy-validation tests](../../crates/core/tests/lazy_validation.rs): `declared_total_and_stored_file_limits_accept_boundary_and_reject_above`, `stopping_after_first_line_does_not_validate_corrupt_unread_tail` | Aggregate/file boundaries and explicit absence of unread-tail validation |
| Emitted excerpt cap and source progress | [text tests](../../crates/core/src/text.rs): `emitted_excerpt_cap_preserves_character_boundaries_and_source_progress`, `emitted_excerpt_cap_accounts_for_replacement_expansion`, `excerpt_cap_handles_zero_small_caps_and_json_escaping` | Line/byte constructors respect 512 emitted bytes, whole-character boundaries, exact/above/zero/small caps, lossy completeness and separate source progress; JSON escaping is outside the cap |
| UTF-8 line conversion | [text tests](../../crates/core/src/text.rs): `line_excerpt_preserves_multibyte_utf8`, `line_excerpt_replaces_invalid_utf8_and_nul_without_corrupting_valid_text` | Accents, Vietnamese, Japanese, emoji and combining characters retain exact text; malformed/truncated UTF-8 and NUL become replacement characters without corrupting valid neighbors |
| Text sanitization | [hostile tests](../../crates/core/tests/hostile.rs): `nul_bytes_become_replacement_char_and_never_break_the_read` | NUL is represented safely while source byte length remains meaningful |
| Entry identification and detection | [hostile tests](../../crates/core/tests/hostile.rs): `non_utf8_name_is_listable_and_addressable_by_id`, `nested_archive_is_detected_and_never_opened` | Non-UTF-8 names do not prevent ID lookup; nested ZIP is detected without recursive extraction |
| Error/status distinction | [hostile tests](../../crates/core/tests/hostile.rs): `corrupt_entry_is_listable_but_unreadable_never_empty`, `encrypted_entry_is_reported_not_silently_empty`, `empty_versus_unreadable_are_never_conflated`, `scanning_a_missing_entry_id_is_an_error_not_an_empty_result` | Corrupt, encrypted, absent, and empty inputs remain distinguishable |
| Marker metadata validation | [marker tests](../../crates/core/tests/markers.rs): `valid_markers_remove_one_line_ending_and_preserve_spaces_and_unicode`, `marker_source_boundary_accepts_256_and_withholds_257_without_a_prefix`, `missing_invalid_and_unreadable_markers_are_distinct_and_identity_survives`, `understated_marker_size_is_rejected_by_the_actual_stream_guard` | Exact LF/CRLF removal, preserved spaces/Unicode and exact name lookup; 256/257 boundary, null bad values, source IDs, missing-only format 0, CRC failure and lying metadata remain explicit |
| Archive format identity | [hostile tests](../../crates/core/tests/hostile.rs): `absent_version_txt_reports_archive_format_0` | Missing version metadata is reported as format 0 |
| MCP error identity | [`error.rs` mapping test](../../crates/mcp/src/error.rs): `streaming_limit_errors_keep_the_entry_id_in_structured_data` | Entry byte and compression-ratio limit failures retain the nested entry ID in structured domain error data |

## Execution and completion

Run from the repository root with a Rust 2024 capable toolchain and locally available dependencies:

```bash
cargo test -p bugreport-insight-core --lib --offline
cargo test -p bugreport-insight-core --test hostile --offline
```

The suite generates synthetic fixtures; no real bugreport is needed.
A selected measure passes only when its assertions execute successfully.
Record command, tested revision and working-tree changes, environment, outcome, and failures using the [shared strategy](../verification/01-strategy.md).

These tests bound retained buffers, not peak process memory or all hostile resource combinations.
The full [requirement review](../swe.6-software-verification/01-requirement-verification.md) must assess those claims separately.
Historical fixture sizes are design inputs, not fixed literals in every test.

Boundary evidence needs actual fixture size and assertions on both sides of the cap.
Short-input assertions below a large ceiling do not prove ceiling enforcement.
Retention assertions cover retained bytes, not total decompression or serialized UTF-8 length.
[Acceptance cases](../swe.6-software-verification/01-requirement-verification.md#acceptance-review-cases) identify where those broader observations are still needed.

## Insight units

[Detailed design](../swe.3-detailed-design/05-insight-generation.md) identifies the `insight` units implementing `REQ-INSIGHT-02`–`05`.
[Core insight tests](../../crates/core/src/insight/tests.rs) use the shared six-domain synthetic ZIP and small adversarial reports.

| Check symbols | Assertion scope |
| --- | --- |
| `invalid_version_marker_does_not_become_format_zero` | Missing marker alone derives format 0; invalid marker leaves version unknown with marker-status attributes and an evidence gap. |
| `all_domains_emit_evidence_and_readme_has_nine_ordered_sections` | Fixed seven files, parsed domain structures, current findings and nine ordered README sections. |
| `crash_records_are_deduplicated_and_native_history_stays_separate`, `identical_crashes_in_old_and_current_windows_form_separate_groups`, `event_only_crash_stays_reported_without_inflating_primary_count` | Java deduplication, stale native/current Java separation, freshness cohorts and secondary event authority. |
| `retained_anr_files_do_not_create_current_incidents` | Retained historical traces are not current incidents; live/event joins remain inferred. |
| `memory_sections_without_summary_metrics_keep_finding_provenance` | Empty, unfamiliar, timed-out and check-in-style meminfo sections still validate; finding provenance locates the header, metrics/clocks stay null, freshness stays undated and gaps remain explicit. |
| `memory_snapshots_keep_units_and_explicit_delta_edges` | KiB units, separate procstats, swap nodes/edges and unknown temporal overlap without causal claims. |
| `cpu_windows_cumulative_counters_and_sampled_contention_are_distinct` | Printed sample windows, cumulative core arithmetic, guest exclusion, top denominator and sampled contention remain distinct. |
| `app_install_state_and_window_local_process_identity_are_explicit`, `uid_names_and_user_scoped_uids_are_decoded` | Uninstalled-user sentinels, per-user UID decoding and window-local fallback identities. |
| `unknown_clocks_and_emulator_properties_remain_null`, `logcat_year_boundary_is_resolved_without_host_clock`, `current_window_uses_first_and_last_lines_not_embedded_clock_minimum` | Unknown clocks/nulls, year rollover and exclusion of old embedded clocks from the current cohort. |
| `excerpts_preserve_utf8_and_report_nul_incompleteness`, `validator_rejects_corrupt_counts_and_freshness` | 512-byte UTF-8 boundary, NUL completeness and rejected count/freshness/source-context violations. |
| `newline_count_excludes_unterminated_tail`, `table_separators_do_not_replace_service_sections`, `section_budget_failure_does_not_emit_partial_analysis` | Newline semantics, correct section boundaries and hard retention-budget failure. |
| `configurable_crash_thresholds_are_applied_and_published`, `prior_digest_decline_does_not_remove_required_artifact_identity`, `hostile_uid_and_uptime_numbers_do_not_overflow`, `raw_searches_include_outer_side_text_entries` | Applied thresholds, mandatory digest, numeric overflow handling and explicit raw-search scope. |

[Writer tests](../../crates/core/src/insight/output.rs) inject a write failure and racing nonempty/empty destinations.
They assert staging cleanup, absent partial final output and preservation of competing output.
Run `cargo test -p bugreport-insight-core --lib --offline`; the [execution record](../../verification/results/insight-generation.md) binds selected measures to actual results.

## Search page measures

[scan_pages.rs](../../crates/core/tests/scan_pages.rs) verifies one-byte UTF-8/unterminated-line continuation, duplicate-scope elimination, complete-line anchored glob matching, explicit long-line gaps, retained deflate decoding and cumulative entry ceilings.
Run `cargo test -p bugreport-insight-core --test scan_pages --offline`.
[Executed result](../../verification/results/search-pagination.md) maps these assertions to REQ-MCP-06/09 and REQ-MCP-ARC-05.

## Range boundaries and subtree filters

[ADR 0012 evidence](../../verification/results/range-boundaries.md) records core partial-read/strict-read and safety checks, component EOF/reversed/empty/unterminated cases, subtree segment boundaries and the real stdio flow.
The measures are [partial_ranges.rs](../../crates/core/tests/partial_ranges.rs), range/subtree cases in [tools.rs](../../crates/mcp/tests/tools.rs) and eof_ranges_and_subtree_boundaries_work_over_stdio in [protocol.rs](../../crates/cli/tests/protocol.rs).

## Confirmation gate units

[ConfirmationGate tests](../../crates/mcp/src/confirmation.rs) verify exact operation/argument binding, mismatch preservation, single-use replay refusal, independent gate instances, 300-second expiry, 64-token capacity and oversized-argument refusal without consuming an existing approval.
[Registry wrapper test](../../crates/mcp/src/registry.rs) uses an invocation counter to prove the content handler is not entered before approval and cannot execute again with the consumed token.
Run cargo test -p bugreport-insight-mcp --lib --offline; [execution evidence](../../verification/results/content-io-confirmation.md) records the REQ-MCP-12 verdict and limitations.

## Server-task failure supervision

[Server supervisor checks](../../crates/mcp/src/server.rs) actual_sdk_service_task_panic_is_a_server_failure and actual_sdk_send_task_panic_is_a_server_failure inject panics through real rmcp transports, not manufactured error values.
Both must produce ServeError::ServerTask with the fault diagnostic; the second specifically detects a nested SDK send-task JoinError inside a successful outer join.
The CLI server_task_failure_exits_nonzero unit check maps that typed failure to exit 1, while the real CLI protocol suite covers normal EOF, Unix broken stdout and request recovery.
[Executed SDK fault evidence](../../verification/results/server-task-failures.md) records the regression and current gates; native non-Unix failure behavior remains separate.

## Raised limits and insight sessions

REQ-GEN-02, REQ-CLI-12 and REQ-MCP-13 require selected-policy checks as well as normal-default regressions.
Core checks compare default and forced admission and produced-byte guards, retained text budget boundaries, and unchanged integrity failures.
MCP checks exercise marker discovery, exact basename addressing, scope confinement, confirmation before generation, fixed artifact names and bounded UTF-8 paging.
The [targeted execution record](../../verification/results/force-and-scoped-insight-mcp.md) identifies selected test symbols and actual results.

## Directory archive selection

REQ-MCP-14 adds report selection and per-report handler state to `Insights::archive_tool` and `registry_with_gate`.
Verification remains pending; the unit measures must cover omitted selection for zero, one, multiple and truncated catalogs, basename/path validation, recognition and pinned no-follow opens, report-local grep cursor behavior, request-bound confirmation arguments including context-generation binding, the shared 64-token pool, the eight-context limit and 300-second idle expiry.
These checks do not replace SWE.5 protocol assertions for exposed tool schemas and successful result payloads.
