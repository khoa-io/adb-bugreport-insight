# `apps.json` — inventory and process table

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Requirement: `REQ-INSIGHT-04`.

Separates installed/user inventory from process attribution over time.
Process identity is `process_key`, never `pid`.

### 4.3 `apps.json`

#### Purpose

`apps.json` separates **installed/user inventory** from **process attribution over time**.

Reference snapshot: 83 live process records, with crash-loop PID churn roughly every 3 seconds.
`pid` keys would turn one failing app into hundreds of "processes".

**The link between package and process is a first-class object, not an implicit join.**
`ProcessRecord.processName` is not a package name — a webview child process lists the host package *plus* its own via `packageList={…}` (`ProcessRecord.java:508`), which is the authoritative many-to-many mapping.
The join is stored, not recomputed.

#### Top-level keys

The fourteen envelope keys, with `capture` replaced by `capture_ref` per rule 6 of [§2.9](02-evidence-conventions.md#29-null-absent-and-raw), plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | `{ source, boot_epoch, uptime_seconds_at_start, log_window_ref }`. **Must not re-derive capture context.** |
| `channels` | array of object | yes | One entry per evidence channel actually present, with its own window and completeness. |
| `inventory` | object | yes | `{ counts, packages[], processes[], shared_uids[], index }`. |
| `lifecycle_events` | array of object | yes | `am_proc_start` / `am_proc_bound` / `am_proc_died` / `am_kill`, normalised. |
| `activity_events` | array of object | yes | `usagestats` `ACTIVITY_RESUMED` / `ACTIVITY_STOPPED` with timestamps. |
| `attribution` | object | yes | PID-reuse evidence, kill-attribution split, unresolved list. |

#### Field tables

##### `channels[]`

A channel shorter than the log window cannot bound full-window counts.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | enum | yes | `dumpsys_package`, `dumpsys_activity_processes`, `am_proc_event_log`, `usagestats`, `logcat_main`, `fs_archive`, `pm_list_packages`. |
| `present` | boolean | yes | Whether the channel exists in this archive. |
| `first_at` / `last_at` | string \| null | yes | The channel's own window. `null` for a point-in-time snapshot. |
| `duration_seconds` | number \| null | yes | Window length. |
| `covers_log_window` | boolean \| null | yes | Whether the channel spans the whole `capture.log_window`. `null` for snapshots. |
| `coverage_ratio` | number \| null | yes | **Required** when `covers_log_window` is false. See invariant 2 of [§2.3](01-envelope.md#23-counts-are-match-blocks). |
| `incomplete_reason` | enum \| null | yes | Why the channel cannot cover the log window. Closed set: `event_log_buffer_retention`, `gated_by_user_build`, `permission_denied`, `declined_by_service`. **`not_emitted_by_platform` is removed** — "the platform never produced this" is a claim about dumpstate's emission contract, not an archive observation. A channel that is `present: false` and has no window simply has `coverage_ratio: null`; where a consumer needs to know the section was *expected*, that is `scope.sections_not_found` in `system.json`. |
| `snapshot_at` | string \| null | yes | For point-in-time channels, when the dump ran. |
| `blocks[]` | array of object | no | For channels split across duplicate sections: `{ index, command, line_start, line_end }`. `index` disambiguates the two `SYSTEM LOG` blocks. |
| `carries` | array of string | yes | What the channel contains. Where a count appears it is the **channel-wide** count, never a filtered subset; a filtered subset belongs in `inventory`, where the filter is visible. |
| `present` when false | — | — | A channel with `present: false` carries no window and no `coverage_ratio`. Every key is still present, `null` per [§2.9](02-evidence-conventions.md#29-null-absent-and-raw) rule 1, and the gap is recorded in `evidence_gaps[]` with `reason: SECTION_ABSENT`. |

##### `inventory.counts`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `packages` | integer | yes | `Package [` block count. | `Settings.java:5118` |
| `processes` | integer | yes | `*APP*` + `*PERS*` record count. | `ProcessList.java:4454-4477,4469` |
| `processes_persistent` / `processes_app` | integer | yes | The two record kinds, summing to `processes`. | `ProcessList.java:4469` |
| `distinct_process_names` | integer | yes | Distinct `process_name` values across `processes[]`. | derived |
| `installed_packages_with_live_process` | integer | yes | Distinct package names appearing in `inventory.index.process_name_to_packages` ∩ installed packages. **Renamed from `packages_with_live_process`**: it is a *package* count, and the previous name invited exactly the confusion with `distinct_process_names` that this artifact exists to prevent. A consumer can compute the coverage ratio as this ÷ `packages`. | derived |
| `users` | integer | yes | `UserInfo` count. | `UserManagerService.java:7311-7331` |
| `shared_uids` | integer | yes | `SharedUser [` block count. | `Settings.java` shared-user dump |
| `package_user_state_blocks` | integer | yes | `User N:` block count inside the package section. | `Settings.java:5471-5510` |
| `lru` | object | yes | `{ total, non_activity, non_service }`, read verbatim off the LRU header. | `ProcessList.java:4323-4331` |
| `car_service_process_names` | integer | yes | Distinct process names under the automotive-service prefix. | `ProcessRecord.java:1396` |
| `car_service_packages` | integer | yes | Distinct package names under the same prefix. Stored alongside `car_service_process_names` because the two are **not** equal, and the difference is the signal. | `Settings.java:5118` |
| `lifecycle_counts` | object | yes | **Removed as a bare-integer object.** Per [§2.3](01-envelope.md#23-counts-are-match-blocks) no artifact may report a bare number, and this one was the clearest violation: five integers with no `pattern`, no `raw_count`, no `coverage`, and no `suppressed_noise`. Each count is now a `counts.am_proc_*_hits` match block, and the event-log window they were measured over is carried once on `lifecycle_channel_window`. | — |
| `lifecycle_channel_window` | object | yes | The event-log buffer's own window and coverage. See below. | `dumpstate.cpp:1028` |
| `sources_available` | array of string | yes | Evidence sources found in this archive. | — |
| `sources_absent` | array of string | yes | Evidence sources searched for and not found. **The absence is the observation; no reason is published for it.** Where the reason is known from the build (a `user`-build gate) it is recorded as an `evidence_gaps[]` entry with `reason: SECTION_ABSENT`; where it is a platform property no archive records, `reason: UNVERIFIABLE`. | — |

`lifecycle_channel_window` identifies partial event-log coverage.
Every count that depends on it must carry its own `coverage` block too, per invariant 2 of [§2.3](01-envelope.md#23-counts-are-match-blocks).

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `first_event_at` / `last_event_at` | string \| null | yes | The first and last `am_proc_*` stamp. `null` when the channel is empty. |
| `duration_seconds` | number \| null | yes | Window length. `null` when fewer than two events. |
| `covers_full_log_window` | boolean | yes | `duration_seconds >= capture.log_window.duration_seconds`. |
| `coverage_ratio` | number \| null | yes | `duration_seconds ÷ capture.log_window.duration_seconds`. **`null` when the window is empty.** |
| `section_titles` | array of string | yes | Which sections the events were found in, with `section_ordinal` disambiguating repeats. Replaces `all_in_first_system_log_block`: that boolean encoded a claim about *where* events sit that is wrong on the reference archive — every `am_proc_*` line lives in the `EVENT LOG`, not in either `SYSTEM LOG` — and a boolean cannot be corrected without rewriting it. Naming the sections is both true and self-correcting. |
| `sections_scanned` | array of object | yes | One entry per duplicate section that was searched: `{title, ordinal, start_line, end_line, matched}`. This is what makes the §2.10 Trap 1 de-duplication checkable rather than assumed. |

##### `inventory.packages[]` — `PackageEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `package_name` | string | yes | **Primary key.** | `Settings.java:5118-5122` |
| `identity_hash` | string | yes | The parenthesised `System.identityHashCode`. **Not an identity**; per-boot and allocator-dependent. Stored so a human can cross-reference a section. | `Settings.java:5120-5122` |
| `app_id` | integer | yes | **Join accelerator only** — reassigned on reinstall, and shared (`android.uid.system` apps all share one `app_id`). | `Settings.java:5129` |
| `shared_uid_name` | string \| null | yes | `android.uid.system`. **This is what makes `app_id=1000` interpretable.** | `Settings.java:5131-5133` |
| `version_name` | string \| null | yes | | `Settings.java:5178` |
| `version_code` / `min_sdk` / `target_sdk` | integer | yes | | `Settings.java:5154-5160` |
| `code_path` / `resource_path` | string | yes | | `Settings.java:5136,5141` |
| `code_path_prefix` | enum | yes | Derived: `/data/app`, `/system`, `/system_ext`, `/product`, `/vendor`, `/apex`, `other`. **Every prefix maps to a `tier`** — the table below is exhaustive by construction, so no package can be left unclassified. | derived from `Settings.java:5136` |
| `tier` | enum | yes | `shared_system` / `system` / `product` / `vendor` / `apex` / `third_party`. **A label, never a filter.** Computed from `code_path_prefix` and `shared_uid_name` by the table below — there is no other input and no fallback judgement. | derived |
| `tier_basis` | object | yes | **A fixed-length array of `(field, value)` pairs, not prose.** The previous form was a sentence, which meant the reader had to trust a natural-language gloss to recompute the label. Now `tier` is mechanically derivable from `tier_basis` plus the table, and a validator can assert they agree. | derived |
| `flags` / `private_flags` | array of string | yes | From `flags=[ … ]` and `privateFlags=[ … ]`. | `Settings.java:5185-5190` |
| `apk_signing_version` | integer | yes | APK signature scheme version. It does not establish install origin; system and sideloaded APKs can use the same scheme. | `Settings.java:5182-5184` |
| `apex_module_name` | string \| null | yes | | `Settings.java:5415` |
| `package_source` / `package_source_name` | integer / string | yes | 0 = `PACKAGE_SOURCE_UNSPECIFIED`, 1 = `OTHER`, 2 = `STORE`, 3 = `LOCAL_FILE`, 4 = `DOWNLOADED_FILE`. | `Settings.java:5387`; `PackageInstaller.java:601-626` |
| `installer_package_name` / `initiating_package_name` | string \| null | yes | | `Settings.java:5370,5376` |
| `last_update_time` | string \| null | yes | **The package's own freshness anchor.** | `Settings.java:5364-5369` |
| `time_stamp` | string \| null | yes | Package-manager file mtime. | `Settings.java:5364-5366` |
| `users[]` | array of object | yes | Per-user install state. Below. | `Settings.java:5471-5510` |
| `declared_permissions[]` | array of string | yes | | `Settings.java` declared-permissions block |
| `install_permissions[]` | array of string | yes | The granted set. **This is the "permissions" the requirements ask for** — per-package and per-user, so it lives here. | `Settings.java:5461-5466` |
| `provenance` | object | yes | See [§2.4](02-evidence-conventions.md#24-provenance). | — |

`users[]`:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `user_id` | integer | yes | The user this install-state block belongs to. | `Settings.java:5471` |
| `installed` | boolean | yes | The printed `installed=` flag. | `Settings.java:5476-5478` |
| `data_dir` | string \| null | yes | `null` when not installed. | `Settings.java:5503-5505` |
| `first_install_time` | string \| null | yes | The printed timestamp, **normalised to `null` when `installed == false`**, because the platform then prints a fixed epoch-era sentinel rather than a real time. | `Settings.java:5507-5509` |
| `first_install_time_raw` | string | yes | The printed value verbatim, always. The normalisation rule keys off the same-line `installed` flag, which *is* archive-observable, so `first_install_time_sentinel` is unnecessary — the previous field required recognising a hardcoded platform literal, and `first_install_time_raw` lets a consumer see the bytes the decision was made on. | — |
| `enabled` | integer | yes | `0` = `COMPONENT_ENABLED_STATE_DEFAULT`. | `Settings.java:5488-5489` |
| `stopped` / `not_launched` / `hidden` / `suspended` / `quarantined` | boolean | yes | | `Settings.java:5479-5493` |
| `ce_data_inode` / `de_data_inode` | integer \| null | yes | **`null` when `installed == false`**, because a not-installed user prints `0` for both and `0` is indistinguishable from a real inode. The `installed` flag on the same object is what distinguishes them; no separate reason field is needed. | `Settings.java:5472` |
| `install_reason` / `install_reason_name` | integer / string | yes | `0` = `INSTALL_REASON_UNKNOWN`. | `Settings.java:5500-5501`; `PackageManager.java:1920` |

##### `inventory.processes[]` — `ProcessEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `process_key` | string | yes | **Proposed key.** `"<boot_epoch>/<start_seq>"`, scoped to one demonstrated AMS lifetime; see the unresolved restart case in [§2.8](02-evidence-conventions.md#28-identity). | `ProcessList.java:2155-2157` |
| `start_seq` | integer | yes | The `ProcessRecord` start sequence. | `ProcessRecord.java:540` |
| `record_kind` | enum | yes | `persistent` / `app`, from `*PERS*` / `*APP*`. | `ProcessList.java:4469` |
| `record_identity_hash` | string | yes | Diagnostic only. A hash that stays constant across a crash loop is a cheap cross-check that the platform reuses the record. | `ProcessRecord.java:1402-1403` |
| `pid` | integer | yes | **Observation attribute, not identity.** | `ProcessRecord.java:517-518` |
| `process_name` | string | yes | The *process* name, not the package name. | `ProcessRecord.java:471-472` |
| `runtime_key` | string | yes | `"<user_id>:<process_name>"`. **Not unique** — many PIDs share one `runtime_key`. | derived |
| `attribution_key` | string | yes | `"<primary package>/<user_id>"` — the human-facing identity. | derived |
| `user_id` / `uid` | integer | yes | The user and uid from the record. | `ProcessRecord.java:465-466` |
| `uid_formatted` | string | yes | `u10a250` style. Decoding: `uid = user * PER_USER_RANGE(100000) + (appId + FIRST_APPLICATION_UID(10000))`; a trailing `iN` is an isolated uid offset. | `ProcessRecord.java:1378-1391` |
| `app_id` | integer | yes | | `UserHandle.java:402` |
| `gids` | array of integer | yes | | `ProcessRecord.java:469-478` |
| `isolated_uid` | integer \| null | yes | Present only when `uid != info.uid`. | `ProcessRecord.java:467-469` |
| `packages[]` | array of string | yes | From `packageList={…}`. **The authoritative process→package link.** | `ProcessRecord.java:508-509` |
| `package_dependencies[]` | array of string | yes | `packageDependencies={…}`. Rare but real. | `ProcessRecord.java:510-517` |
| `start_uptime_ago` | string | yes | The `startUpTime` value. **Uptime-relative, not wall clock.** | `ProcessRecord.java:522-523` |
| `start_elapsed_time_ago` | string \| null | yes | Same on the elapsed-realtime clock. | `ProcessRecord.java:524-525` |
| `start_wall_clock` | string \| null | yes | **Derived**: `snapshot_instant − start_uptime_ago`, i.e. the wall clock of the dumpsys snapshot that printed `startUpTime`, *minus* the process's age. **Not** `boot.at + start_uptime_ago`: `startUpTime` is a relative age (`ProcessRecord.java:522` formats `nowUptime − mStartUptime`), so it must be anchored to the snapshot instant, not to boot. | derived |
| `start_wall_clock_evidence` | object | yes | **An evidence locator, not a quoted string.** `{archive_path, section_title, section_ordinal, line_start, line_end, excerpt}` — a `corroborating_evidence`-shaped object per [§2.7](02-evidence-conventions.md#27-evidence). The previous form was a bare quoted log line with no `line_start`, so it was unverifiable under §2.7 despite claiming to be evidence. When the only source is the `Start proc` logcat line, the locator points there; when no such line exists, `start_wall_clock` is `null` and an `evidence_gaps[]` entry records it. | — |
| `last_activity_time_ago` | string | yes | | `ProcessRecord.java:520-521` |
| `persistent` / `removed` | boolean | yes | Only printed when set. | `ProcessRecord.java:527-529` |
| `start_seq_conflict` | boolean | yes | `true` if two entries share a `start_seq`. A repeated sequence can reflect a repeated observation or an AMS restart. It does not by itself prove disagreement or the same process instance. | derived |
| `code_path` / `data_dir` | string | yes | | `ProcessRecord.java:503-505` |
| `instruction_set` / `required_abi` | string \| null | yes | | `ProcessRecord.java:496-499` |
| `class_name` | string \| null | yes | | `ProcessRecord.java:500-502` |
| `compat` | string \| null | yes | The printed `compat={…}` braces, verbatim. | `ProcessRecord.java:518-520` |
| `oom_adj` | object | yes | `{max, cur_raw, set_raw, cur, set}`. | oom-adj dump |
| `oom_adj_name` | string | yes | From the LRU line: `pers`, `fg`, `vis`, `psvc`, `cch`, `sys`. | `ProcessList.java:1161-1180,4341` |
| `cur_proc_state` / `set_proc_state` | integer | yes | | process-state dump |
| `capability` | string | yes | `LCMNFUA`. **7 chars, positional, `-` = absent.** Carried as the raw string; no per-character decode, because that decode was not traced to a source. | `ProcessList.java:4343` |
| `lru` | object | yes | `{index, oom_adj_name, capability, flags, fixed, service, top_activity, client, cch_empty}`. | `ProcessList.java:4333-4400` |
| `services[]` / `providers[]` / `receivers[]` / `connections[]` | array | yes | Component wiring. **`crashes.json` reads these; it does not re-parse them.** | `ProcessRecord.java:559-563` |
| `lifecycle_refs` | array of object | yes | `{event_kind, at, event_seq_index}` linking to `lifecycle_events[]`. | derived |
| `provenance` | object | yes | See [§2.4](02-evidence-conventions.md#24-provenance). **No `lifecycle_unresolved_reason`.** Join failures belong in `attribution.unresolved[]`, keyed by machine-readable `reason` and `count`, rather than prose on a ProcessEntry. | — |

##### `lifecycle_events[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `kind` | enum | yes | `am_proc_start` / `am_proc_bound` / `am_proc_died` / `am_kill`. | `EventLogTags.logtags:20,22,25,34` |
| `at` | string | yes | `MM-DD HH:MM:SS.mmm` from the logcat stamp. | `logprint.cpp` threadtime |
| `uid_column_raw` | string | yes | **The raw third column, verbatim.** Never coerced at parse time — see the note below. | `logprint.cpp:1492-1513` |
| `pid` / `tid` | integer | yes | | same |
| `user_id` | integer | yes | `am_proc_start`'s first arg is `UserHandle.getUserId(app.getStartUid())` — the **start** uid, not `info.uid`. | `ProcessList.java:2858-2861` |
| `start_uid` | integer | yes | `am_proc_start` only. | `ProcessList.java:2860` |
| `process_name` | string | yes | | `ProcessList.java:2861` |
| `hosting_type` / `hosting_component` | string \| null | yes | `service`, `activity`, `""`. | `ProcessList.java:2862-2863` |
| `oom_adj` | integer \| null | yes | `am_proc_died`, `am_kill`. | `logtags:22,34` |
| `reason` | string \| null | yes | `am_kill` only. | `logtags:34` |
| `rss_kib` | integer \| null | yes | `am_kill` only (`Rss\|2\|2`). | `logtags:34` |
| `process_key` | string \| null | yes | `null` when no `ProcessRecord` interval contains `at`, with the cause recorded in `attribution.unresolved[]` rather than in a prose field on the event. | derived |
| `provenance` | object | yes | Carries `block_index` to disambiguate the two `SYSTEM LOG` blocks, and `channel_window` so a consumer can see the event sits inside a short-retention buffer. | — |

> **The logcat uid column is a `getpwuid` name when it is ≤ 5 characters, else a decimal uid, else six spaces when `entry->uid < 0`** (`logprint.cpp:1492-1513`).
> It is *never* guaranteed to be an integer.
> Measured on the reference archive: 41,230 numeric and 6,038 symbolic (`root` 4060, `media` 822, `radio` 635, `shell` 200, `logd` 144, `wifi` 61, `lmkd` 18, `artd` 3, `gps` 2).
> Any uid-joining code must try the name table first.

##### `activity_events[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `at` | string | yes | **Second granularity, unlike logcat's milliseconds.** | `UserUsageStatsService.java:1087` |
| `type` | string | yes | `ACTIVITY_RESUMED`, `ACTIVITY_STOPPED`, `USER_UNLOCKED`, `SCREEN_INTERACTIVE`, … | `UserUsageStatsService.java:1088-1124` |
| `package_name` / `class_name` | string | yes | **This is what ties a crash loop to a specific activity.** | same |
| `range_raw` | string | yes | Verbatim. The locale form contains U+202F narrow no-break spaces and an en dash — **do not regex-split on a space.** | same |
| `user_id` | integer \| null | yes | The user the enclosing `usagestats` block declares. **The `ACTIVITY_*` line itself carries no `user=` field**, so this is a block-level join; where the enclosing block declares no user, the value is `null` and an `evidence_gaps[]` entry records it. The open question this field carried is **closed**: the archive does contain a block for a second user, so coverage is not user-0-only. | — |
| `range_header_form` | enum | yes | **Moved to the `channels[]` entry for this section.** It is a property of *which dumpsys invocation produced the section*, not of an individual event, and two invocations render it differently — modelling it per-event let a parser merge a locale-formatted window with an epoch-millisecond one and produce a 24-hour span. | `UserUsageStatsService.java:1157-1163` |
| `range_window_seconds` | number \| null | yes | The interval parsed from the section's range header, so a consumer can tell whether these events describe the log window or something far wider. `null` when the header carries no year or no offset and the interval therefore cannot be reconstructed — on the reference archive the offset is recoverable only from the `----- pid N at … -----` stamps inside the block, which is a reconstruction, not a read. | derived |

##### `attribution`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kill_sources` | object | yes | `{ am_kill_events, ams_crash_loop_kill_lines, app_errors_am_process_crashed_too_much }`. **Three disjoint buckets, never summed into one `kills` number.** The second key is renamed from `ams_crash_loop_kills` to `…_kill_lines`: the bytes are a logcat line, not a kill notification, and calling them *kills* asserted an attribution the archive does not carry. |
| `kill_source_coverage` | object | yes | **Renamed from `kill_source_completeness`, and the verdict strings are removed.** One entry per bucket: `{ channel, window_start, window_end, coverage_ratio, covers_log_window }` — the same numbers `channels[]` already carries. A prose verdict like `"complete_for_the_window"` or a 100-character `"PARTIAL — …"` is a judgment about completeness that no archive byte settles: a zero in a channel covering a third of the log window cannot establish absence over the rest of it. |
| `pid_reuse` | object | yes | `{worst_process, pid_first, pid_last, distinct_pid_count, pid_max, window_first, window_last, window_seconds, start_proc_events}`. **`pid_delta` and `estimated_full_pid_cycles` are removed** — a PID *range span* is not a count, and dividing it by an assumed cycle size found nowhere in the archive produced a number [§5.2](11-limitations.md#52-reference-facts) has already retracted. `distinct_pid_count` and `pid_max` answer the recycling question with two measured values: if `distinct_pid_count` approaches the event count, PIDs are not being reused. `conclusion` is removed with the rest; the identity model above already states it. |
| `unresolved[]` | array of object | yes | Events that could not be joined, each with a `reason` from **one closed enum** shared by `unresolved[]` and `lifecycle_events[].unresolved_reason`: `no_process_record`, `outside_snapshot_range`, `isolated_uid`, plus `count`. Use the same machine-readable reason vocabulary for aggregation and event records. |

`am_kill: 0` is a **bounded negative**, not a proof.
The event-log buffer covers a fraction of the log window, so the honest claim is *"no `am_kill` event in the covered interval"*, and `kill_source_coverage` is what makes that bound visible.
Any consumer wanting to assert no memory-pressure kill across the whole window must read `/proc/vmstat oom_kill` from `memory.json`, which is a different channel with a different window.

##### `inventory.index`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `package_to_process_keys` | object | yes | Package → process keys. |
| `process_name_to_packages` | object | yes | Process name → packages, for names that are not package names (`system`, `.adservices`, `android.process.media`, …). |
| `uid_to_process_keys` | object | yes | UID → process keys. |
| `unmapped_process_names` | object | yes | `{count, examples}`. |

#### Identity model

Three derived keys, each for a different purpose:

| Key | Formula | Use |
| --- | --- | --- |
| `process_key` | `"<boot_epoch>/<start_seq>"` | Proposed identity within one demonstrated AMS lifetime. Events do not carry this sequence; attribution needs separate evidence. |
| `attribution_key` | `"<package_name>/<user_id>"` | "Which app, on which profile." The human-facing identity. |
| `runtime_key` | `"<user_id>:<process_name>"` | The pre-identity grouping. **Not unique.** |

`start_seq` is adopted rather than invented because **AOSP itself uses it as the instance identity**: `ActivityManagerService.java:930` compares `existingApp.getStartSeq() == app.getStartSeq()`; `:4475` rejects an attach whose `startSeq` differs; `:4814` matches a provider process on `getStartUid() == uid && getStartSeq() == startSeq`.
The counter resets when AMS restarts (`ProcessList.java:463`), while the device boot instant can remain unchanged.
Safe cross-restart attribution needs an evidenced lifetime discriminator.
See [identity limits](02-evidence-conventions.md#28-identity).

**Correlating an event to an app requires observed lifetime evidence.**
Event-log rows do not carry `startSeq`.
Use PID/UID/process name together with observed start/death boundaries and clocks; PID reuse and missing events can leave multiple candidates.
`startUpTime` is a relative age at the snapshot (`ProcessRecord.java:522`), and `lastActivityTime` is activity history, not a death time.
Do not add either printed age directly to the boot instant to invent a lifetime interval.

1. **Event → process instance.**
   Join only when the observed lifecycle and identity evidence determine one candidate; mark a heuristic join as inferred.
2. **Process instance → app.**
   Use the observed `packageList={…}` membership.

Unresolved step 1 goes in `attribution.unresolved[]` with reason (`no_process_record`, `outside_snapshot_uptime_range`, `isolated_process`); never drop or guess.

#### Provenance and freshness

Five distinct clocks, and `provenance.capture_kind` plus `captured_at_time_base` name which one applies:

| Object | Clock | Example value | `capture_kind` |
| --- | --- | --- | --- |
| `packages[]` | wall clock, `lastUpdateTime` | `2026-05-18 23:50:51` | `disk_artifact` |
| `packages[].users[]` | wall clock, `firstInstallTime` | `2026-05-18 23:51:03` | `disk_artifact` |
| `processes[]` | **uptime**, `startUpTime` | `-108ms` | `point_in_time_snapshot` |
| `lifecycle_events[]` | logcat wall clock, **event-log buffer** | `00:27:01.587` | `live_log` |
| `activity_events[]` | usage-stats wall clock, **second granularity** | `00:27:03` | `point_in_time_snapshot` |

A package entry is as fresh as its last update — **126 days** before capture for every system package on this archive, and **11 s** for the one package sideloaded during this boot.
A process record is snapshot-scoped: fresh only at `capture.uptime.at_snapshot_dumpsys_activity_seconds`.
A lifecycle event is inside the *event log's* window (0.314 of the log window), **not** necessarily inside the log window.

Channel coverage on this archive:

| Channel | Window | Coverage | What it can bound |
| --- | --- | --- | --- |
| `logcat_main` | 00:26:47.418 → 00:27:52.080 | 1.00 | everything (277 `Start proc`, 273 `crashed too many times`) |
| `am_proc_event_log` | 00:26:59.872 → 00:27:20.176 | **0.314** | process churn **only in that 20.3 s** (121 starts, 40 deaths) |
| `dumpsys_activity_processes` | snapshot at ≈00:27:41.9 | n/a | liveness at that instant, nothing before |
| `dumpsys_package` | snapshot at 00:27:48 (0.831 s) | n/a | install state at that instant |
| `usagestats` | 24 h window, second granularity | >1 | activity history, pre-boot included |
| `fs_archive` | file mtimes | n/a | anything — this is where the 126-day-stale ANR files live |

[§2.3 invariant 2](01-envelope.md#23-counts-are-match-blocks) requires `coverage_ratio: 0.314` alongside `am_proc_died: 40`.

