# `apps.json` — worked examples and open questions

> **Illustrative example for the promoted `UC-03` contract.**
> See [`index.md`](../index.md).

Examples/open questions from [`06-apps-artifact.md`](../06-apps-artifact.md), split at the 500-line limit.
Heading levels are promoted one step; the content is unchanged.

### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "apps",

  "capture_ref": {
    "source": "system.json#/capture",
    "boot_epoch": "<boot.at>",
    "boot_epoch_uncertainty_seconds": <uncertainty_seconds>,
    "uptime_seconds_at_start": <seconds>,
    "at_snapshot_dumpsys_activity_seconds": <seconds>
  },

  "channels": [
    { "name": "logcat_main", "present": true,
      "first_at": "MM-DD HH:MM:SS.mmm", "last_at": "MM-DD HH:MM:SS.mmm",
      "duration_seconds": <log_window_seconds>, "covers_log_window": true, "coverage_ratio": 1.0,
      "incomplete_reason": null, "snapshot_at": null,
      "blocks": [
        { "index": 0, "command": "<logcat argv, first block>", "line_start": <line>, "line_end": <line> },
        { "index": 1, "command": "<logcat argv, -T bounded second block>", "line_start": <line>, "line_end": <line> }
      ],
      "carries": ["Start proc", "has crashed too many times, killing!", "am_* events"] },
    { "name": "am_proc_event_log", "present": true,
      "first_at": "MM-DD HH:MM:SS.mmm", "last_at": "MM-DD HH:MM:SS.mmm",
      "duration_seconds": <event_log_seconds>, "covers_log_window": false, "coverage_ratio": <coverage_ratio>,
      "incomplete_reason": "event_log_buffer_retention", "snapshot_at": null,
      "blocks": [{ "index": 0, "command": "<section_title>", "line_start": <line>, "line_end": <line> }],
      "carries": ["am_proc_start", "am_proc_bound", "am_proc_died", "am_kill", "am_uid_running", "am_uid_active"] },
    { "name": "dumpsys_activity_processes", "present": true, "snapshot_at": "<captured_at>",
      "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["PID mappings", "Process LRU list", "*APP*/*PERS* records"] },
    { "name": "dumpsys_package", "present": true, "snapshot_at": "<captured_at>",
      "duration_seconds": <duration_seconds>, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["Package [ blocks", "SharedUser [ blocks", "per-user state blocks"] },
    { "name": "usagestats", "present": true,
      "snapshot_at": "<captured_at>", "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": false, "coverage_ratio": null,
      "incomplete_reason": "permission_denied", "blocks": [],
      "carries": ["ACTIVITY_RESUMED", "ACTIVITY_STOPPED"],
      "range_header_form": "timeRange_locale",
      "range_raw": "<verbatim range header, U+202F and en dash intact>",
      "range_window_seconds": null },
    { "name": "fs_archive", "present": true, "snapshot_at": null,
      "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["FS/** ZIP entries"] },
    { "name": "pm_list_packages", "present": false,
      "snapshot_at": null, "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null,
      "blocks": [], "carries": [] }
  ],

  "inventory": {
    "counts": {
      "packages": <n>, "processes": <n>, "processes_persistent": <n>, "processes_app": <n>,
      "distinct_process_names": <n>, "installed_packages_with_live_process": <n>,
      "users": <n>, "shared_uids": <n>, "package_user_state_blocks": <n>,
      "lru": { "total": <n>, "non_activity": <n>, "non_service": <n> },
      "car_service_process_names": <n>,
      "car_service_packages": <n>,
      "lifecycle_channel_window": {
        "first_event_at": "MM-DD HH:MM:SS.mmm", "last_event_at": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <event_log_seconds>, "covers_full_log_window": false,
        "coverage_ratio": <coverage_ratio>,
        "section_titles": ["<section_title>"],
        "sections_scanned": [
          { "title": "<section_title>", "ordinal": 0, "start_line": <line>, "end_line": <line>, "matched": <n> },
          { "title": "<section_title>", "ordinal": 1, "start_line": <line>, "end_line": <line>, "matched": 0 }
        ]
      },
      "sources_available": ["dumpsys package", "dumpsys activity processes", "am_proc event log",
                            "usagestats", "logcat main", "FS/** ZIP entries"],
      "sources_absent": ["pm list packages -f -U -i", "per-package dumpsys section"]
    },
    "counts_am_proc_hits": {
      "_note": "each am_proc_* count is a match block per §2.3; abbreviated here to two keys",
      "am_proc_died_hits": {
        "pattern": "am_proc_died", "pattern_kind": "tag", "case_sensitive": true,
        "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
        "coverage": { "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
                      "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
        "regions": [{ "archive_path": "<main_entry>", "section_title": "<section_title>",
                      "section_ordinal": 0, "line_start": <line>, "line_end": <line>, "line_count": <n> }]
      },
      "am_kill_hits": {
        "pattern": "am_kill", "pattern_kind": "tag", "case_sensitive": true,
        "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
        "coverage": { "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
                      "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
        "regions": []
      }
    },

    "packages": [
      { "package_name": "<package_name>",
        "identity_hash": "<hex>", "app_id": <n>, "shared_uid_name": null,
        "tier": "third_party",
        "tier_basis": [["code_path_prefix", "/data/app"], ["package_source", 1],
                       ["apk_signing_version", 2], ["flags_has_SYSTEM", false]],
        "version_name": "<n>", "version_code": <n>, "min_sdk": <n>, "target_sdk": <n>,
        "code_path": "/data/app/~~<hash>==/<package_name>-<n>-<hash>==",
        "code_path_prefix": "/data/app",
        "flags": ["DEBUGGABLE","HAS_CODE","ALLOW_CLEAR_USER_DATA","TEST_ONLY","ALLOW_BACKUP"],
        "apk_signing_version": 2,
        "package_source": 1, "package_source_name": "PACKAGE_SOURCE_OTHER",
        "initiating_package_name": "<initiating_package_name>",
        "last_update_time": "<YYYY-MM-DD> HH:MM:SS",
        "install_permissions": ["<package_name>.<permission>", "<vendor.permission>.<name>"],
        "users": [
          { "user_id": 0, "installed": false, "data_dir": null, "enabled": 0,
            "first_install_time": null, "first_install_time_raw": "<epoch-era sentinel, verbatim>",
            "ce_data_inode": null, "de_data_inode": null,
            "install_reason": 0, "stopped": true },
          { "user_id": 10, "installed": true,
            "data_dir": "/data/user/10/<package_name>",
            "ce_data_inode": <n>, "de_data_inode": <n>, "enabled": 0,
            "first_install_time": "<YYYY-MM-DD> HH:MM:SS",
            "install_reason": 0, "stopped": false, "not_launched": false }
        ],
        "provenance": { "evidence_origin": "dumpsys_package", "archive_path": "<main_entry>",
          "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "now",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "package updated within the same boot as capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "package_name": "<automotive_system_package>",
        "identity_hash": "<hex>", "app_id": <n>, "shared_uid_name": "android.uid.system",
        "tier": "shared_system",
        "tier_basis": [["shared_uid_name", "android.uid.system"], ["code_path_prefix", "/system"],
                       ["flags_has_SYSTEM", true]],
        "version_name": "<n>", "version_code": <n>, "min_sdk": <n>, "target_sdk": <n>,
        "code_path": "/system/app/<AppDir>", "code_path_prefix": "/system",
        "flags": ["SYSTEM","HAS_CODE","PERSISTENT","ALLOW_CLEAR_USER_DATA","ALLOW_BACKUP"],
        "apk_signing_version": 3,
        "package_source": 0, "package_source_name": "PACKAGE_SOURCE_UNSPECIFIED",
        "last_update_time": "<YYYY-MM-DD> <HH:MM:SS>",
        "users": [
          { "user_id": 0, "installed": true,
            "data_dir": "/data/user/0/<package_name>",
            "ce_data_inode": <inode>, "de_data_inode": <inode>,
            "first_install_time": "<YYYY-MM-DD> <HH:MM:SS>", "enabled": 0, "stopped": false }
        ],
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "system package last updated 128 days before capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "processes": [
      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "persistent", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<process_name>",
        "runtime_key": "<user_id>:<process_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": 0, "uid": <uid>, "uid_formatted": "<uid_formatted>", "app_id": <app_id>,
        "gids": [<gids>],
        "isolated_uid": null,
        "packages": ["<package_name>"], "package_dependencies": [],
        "start_uptime_ago": "-<N>ms", "start_elapsed_time_ago": "-<N>ms",
        "start_wall_clock": "<captured_at>",
        "start_wall_clock_evidence": { "archive_path": "<main_entry>", "kind": "line_range",
          "section_title": "<section_title>", "section_ordinal": 0,
          "locator": { "line_start": <line>, "line_end": <line> },
          "excerpt": "<verbatim Start proc line>", "excerpt_truncated": false,
          "excerpt_bytes": <n>, "excerpt_complete": true },
        "last_activity_time_ago": "-<N>ms",
        "persistent": true, "removed": false, "start_seq_conflict": false,
        "code_path": "/system/app/<AppDir>/<App>.apk",
        "data_dir": "/data/user/<user_id>/<package_name>",
        "instruction_set": "<isa>", "required_abi": "<abi>",
        "class_name": "<package_name>.<ApplicationClass>",
        "compat": "<compat braces, verbatim>",
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "cur_proc_state": <n>, "set_proc_state": <n>, "capability": "<7-char raw>",
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "fixed": true },
        "providers": ["<provider_class>"],
        "lifecycle_refs": [],
        "provenance": { "evidence_origin": "dumpsys_activity_processes",
          "archive_path": "<main_entry>",
          "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer 'ending at: HH:MM:SS'",
            "basis": "process liveness is only true at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "persistent", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<system_process_name>", "runtime_key": "<user_id>:<process_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": 0, "uid": <uid>, "app_id": <app_id>,
        "packages": ["<package_name>"], "package_dependencies": ["<package_name>"],
        "start_uptime_ago": "-<N>s<N>ms",
        "start_wall_clock": "<captured_at>",
        "persistent": true,
        "code_path": "/system/priv-app/<AppDir>/<App>.apk",
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "fixed": true },
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer", "basis": "liveness at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "app", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<package_name>",
        "runtime_key": "<user_id>:<package_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": <n>, "uid": <uid>, "uid_formatted": "<uid_formatted>", "app_id": <app_id>,
        "gids": [<gids>],
        "packages": ["<package_name>"],
        "start_uptime_ago": "-<N>s<N>ms",
        "start_wall_clock": "<captured_at>",
        "last_activity_time_ago": "-<N>s<N>ms",
        "persistent": false,
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "top_activity": true },
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer", "basis": "liveness at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "index": {
      "package_to_process_keys": { "<package_name>": ["…/<start_seq>"] },
      "process_name_to_packages": {
        "system": ["android", "com.android.providers.<name>", "com.android.server.<name>"],
        ".<suffix>": ["com.<vendor>.<suffix>.api"]
      },
      "unmapped_process_names": { "count": <n>,
        "examples": ["system", "android.process.<media>", "android.car.<usb>.handler",
                     ".<suffix>", "<WebViewLoader>"] }
    }
  },

  "lifecycle_events": [
    { "kind": "am_proc_start", "at": "MM-DD HH:MM:SS.mmm",
      "uid_column_raw": "<uid column, verbatim>", "pid": <pid>, "tid": <tid>,
      "user_id": <n>, "start_uid": <uid>, "process_name": "<process_name>",
      "hosting_type": "service",
      "hosting_component": "{<package>/<package>.<ServiceClass>}",
      "process_key": "<boot_epoch>/<start_seq>",
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "kind": "am_proc_start", "at": "MM-DD HH:MM:SS.mmm",
      "uid_column_raw": "<uid column, verbatim>", "pid": <pid>, "tid": <tid>,
      "user_id": <n>, "start_uid": <uid>,
      "process_name": "<WebViewLoader process>", "hosting_type": "", "hosting_component": "",
      "process_key": null,
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "activity_events": [
    { "at": "<captured_at>", "type": "ACTIVITY_RESUMED",
      "package_name": "<package_name>",
      "class_name": "<package_name>.<ActivityClass>",
      "user_id": <n>,
      "range_window_seconds": null,
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
        "freshness": "now",
        "freshness_basis": { "input": "usagestats event timestamp",
          "basis": "second granularity; inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "attribution": {
    "kill_sources": {
      "am_kill_events": 0,
      "ams_crash_loop_kill_lines": <n>,
      "app_errors_am_process_crashed_too_much": <n>
    },
    "kill_source_coverage": {
      "am_kill_event_log": { "channel": "am_proc_event_log",
        "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <event_log_seconds>, "covers_log_window": false,
        "coverage_ratio": <coverage_ratio> },
      "ams_logcat": { "channel": "logcat_main",
        "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <log_window_seconds>, "covers_log_window": true,
        "coverage_ratio": 1.0 }
    },
    "pid_reuse": {
      "worst_process": "<process_name>",
      "pid_first": <n>, "pid_last": <n>, "distinct_pid_count": <n>, "pid_max": <n>,
      "window_first": "MM-DD HH:MM:SS.mmm", "window_last": "MM-DD HH:MM:SS.mmm",
      "window_seconds": <seconds>,
      "start_proc_events": <n>
    },
    "unresolved": [
      { "reason": "outside_snapshot_range", "count": <n> },
      { "reason": "isolated_uid", "count": <n>, "example": "<uid_formatted>" }
    ]
  },

  "evidence_gaps": [
    { "field": "inventory.shared_uids[].*", "reason": "UNVERIFIABLE",
      "detail": "the SharedUser header prints a name and an identity hash but no id; the id is read from the archive where present and left absent where not." },
    { "field": "lifecycle_events[].uid_column_raw", "reason": "PARSE_INDETERMINATE",
      "detail": "the logcat uid column renders as a getpwuid name when short, a decimal uid otherwise, and blanks below zero. uid_column_raw preserves it verbatim and no coercion is attempted at parse time; consumers must resolve it." },
    { "field": "attribution.kill_sources.am_kill_events", "reason": "REGION_EXCLUDED",
      "detail": "zero am_kill events in the event-log interval. This is a bounded negative over the covered window only, not an assertion that no memory-pressure kill occurred in the rest of the log window." },
    { "field": "activity_events[].range_window_seconds", "reason": "PARSE_INDETERMINATE",
      "detail": "the range header carries no year and no timezone offset; the interval is only reconstructible from the pid stamps inside the block, which is a reconstruction rather than a read, so the field is null." },
    { "field": "activity_events[]", "reason": "UNVERIFIABLE",
      "detail": "whether these events cover a second user profile is settled here: the archive does contain a second-user block in the same section, and the enclosing user ID must be preserved." }
  ],

  "findings": [],

  "notes": [
    "EMPTY_FINDINGS_BY_DESIGN: apps.json is inventory and attribution, not diagnosis. Process churn, crash loops and memory pressure are findings belonging to cpu.json, crashes.json and memory.json respectively; duplicating them here would let two artifacts carry two answers to one question. This note satisfies §2.1's rule that an empty findings array must say why.",
    "UNATTRIBUTED_EVENTS_ARE_DECLARED: lifecycle events that cannot be joined to a process instance are counted in attribution.unresolved[] with a reason from a closed enum, never dropped and never guessed."
  ]
}
```

### Open questions

- **`Cmd line:` is not an AMS field.**
  `Cmd line:` is emitted by `art/runtime/signal_catcher.cc:55-69` (with the comment *"The string `Cmd line:` is chosen to match the format used by debuggerd"*), not by `ProcessRecord.dump()` (`ProcessRecord.java:462-565`).
  In the reference archive all 251 `Cmd line:` lines are inside `VM TRACES JUST NOW` (184) and `VM TRACES AT LAST ANR` (67); these lines supply ANR trace evidence.
  **Consequence: `apps.json` has no `cmd_line` field.**
  It belongs to `anr.json`.
- **`am_kill` is zero here.**
  Every kill-shaped field in `attribution.kill_sources` is therefore `0` on this archive, and the `Rss|2|2` 2-byte-int decoding and the `Reason` string vocabulary could not be validated against a real sample.
  Treat both as **spec-derived, not observation-derived**.
- **`oom_adj` object field names.**
  `oom adj: max=… curRaw=… setRaw=… cur=… set=…` was read out of the process dump and `Process LRU list` for the symbol, but the exact printer was not pinned — it is emitted by the process-state / `ProcessList` oom-adj dump, not by `ProcessRecord.dump()`.
- **`capability` string `LCMNFUA`.**
  Mapped to `ActivityManager.printCapabilitiesSummary` via `ProcessList.java:4343`, but the per-character definition was not read.
  The schema carries the raw string only, with no decoded per-capability booleans.
- **`am_proc_start`'s `user_id` versus `uid`.**
  `ProcessList.java:2859` logs `UserHandle.getUserId(app.getStartUid())` while `ProcessRecord.java:465-466` dumps `user #<userId> uid=<info.uid>`.
  These may disagree across start/record uids; retain both without normalization.
  No observed disagreement confirms this failure mode.
- **Per-user process split.**
  Measured 83 records = **50 user-10, 24 user-0, 9 shared-uid** (the last group prints as a bare numeric uid by `ProcessRecord.java:1377-1378`, so it is not attributable to a user from that field alone).
  Not in `counts` above because it needs the same parse as `uid_formatted`.
  If it is wanted first-class: `counts.processes_by_user: {"0": 24, "10": 50, "shared_uid": 9}`.
- **`dumpsys usagestats` user coverage.**
  The section prints `user=0` and then per-user 24 h / 7 d blocks.
  Whether it covers user 10 at all in this archive **is not confirmed**.
  Verify profile coverage before using `activity_events[]` or trusting `activity_events` on multi-user devices.

---

