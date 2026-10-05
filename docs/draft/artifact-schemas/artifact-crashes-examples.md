# `crashes.json` — worked examples and open questions

> **Draft.** See [`index.md`](./index.md).

The worked example and open questions of [`artifact-crashes.md`](./artifact-crashes.md), split out
because together they exceed the 500-line limit. Heading levels are promoted one step; the content
is unchanged.#

### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "crashes",

  "capture_ref": { "source": "system.json#/capture", "pid_max": <pid_max>, "min_crash_interval_ms": <ms> },

  "counts": {
    "fatal_exception_hits": {
      "pattern": "FATAL EXCEPTION", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the suppressed lines are second renderings of crashes already counted: each duplicates another occurrence on (pid, second), and its logcat prefix uid differs from the uid in the message body",
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 1,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    },
    "tombstone_hits": {
      "pattern": "tombstone", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "no tombstone banner exists anywhere in the main entry; the real tombstones live in FS/data/tombstones/ as ZIP entries, which is where this artifact looks",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    },
    "signal_hits": {
      "pattern": "signal ", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the suppressed matches are teardown and thread-dump lines whose 'signal' token is not a fatal-signal banner; fatal signals use the tombstone banner form, which this artifact reads from the tombstone ZIP entries instead",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "am_crash_hits": {
      "pattern": " am_crash:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    }
  },

  "channels": [
    { "channel": "crash_buffer", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "fatal_exception_hits" },
    { "channel": "am_crash_event_log", "emitter_kind": "event_log_tag", "present": true,
      "counts_ref": "am_crash_hits" },
    { "channel": "dropbox_crash", "emitter_kind": "dumpsys_section", "present": true,
      "counts_ref": "dropbox_crash_hits" },
    { "channel": "tombstone_files", "emitter_kind": "zip_entry_listing", "present": true,
      "counts_ref": "tombstone_entry_records" },
    { "channel": "am_proc_died", "emitter_kind": "event_log_tag", "present": true,
      "counts_ref": "am_proc_died_hits" },
    { "channel": "manager_kill_lines", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "crashed_too_many_hits" },
    { "channel": "rescueparty_lines", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "rescueparty_hits" }
  ],

  "count_authority": {
    "channel": "crash_buffer",
    "rule": "the crash count is taken from the crash buffer, the only channel whose window spans the whole log window with no rate limit and no ring retention"
  },

  "java_crash_groups": [
    { "group_id": "<java_group_id>",
      "language": "java",
      "signature": {
        "exception_class": "<exception_class>",
        "signal_number": null, "signal_name": null, "sigcode_name": null,
        "top_frame": "<normalised top frame symbol, no line number>"
      },
      "packages": ["<crashing_package>"], "package_count": 1,
      "repeat_count": 271,
      "raw_line_hits": 274, "duplicate_hits": 3,
      "duplicate_detection": { "method": "same_pid_same_second", "duplicate_pids": [5769, 9384, 9408] },
      "first_seen": "<first_seen>",
      "last_seen": "<last_seen>",
      "window_seconds": <window_seconds>,
      "rate_per_second": <rate_per_second>,
      "inter_gap_p50_ms": <ms>, "inter_gap_max_ms": <ms>,
      "pid_span": { "first": <pid>, "last": <pid>, "distinct_count": <n>, "pid_max": <pid_max> },
      "classification": "crash_loop",
      "classification.thresholds": { "repeat_count": <n>, "rate_per_second": <n>,
                                      "window_seconds": <n>, "inter_gap_max_ms": <ms>,
                                      "manager_kill_count": <n> },
      "classification_met": ["repeat_count", "rate_per_second", "window_seconds",
                             "inter_gap_max_ms", "manager_kill_count"],
      "manager_kill_count": <n>, "manager_kill_reason": "<printed Reason: token, verbatim>",
      "rescueparty_failure_count": <n>, "rescueparty_disabled": <boolean>,
      "corroboration": [
        { "channel": "am_crash_event_log", "count": <n>, "count_kind": "event_log_rows" },
        { "channel": "dropbox_crash", "count": <n>, "count_kind": "logcat_lines_distinct" },
        { "channel": "am_proc_died", "count": <n>, "count_kind": "event_log_rows" },
        { "channel": "manager_kill_lines", "count": <n>, "count_kind": "logcat_lines_distinct" },
        { "channel": "rescueparty_lines", "count": <n>, "count_kind": "logcat_lines_distinct" }
      ],
      "provenance": { "evidence_origin": "crash_buffer", "archive_path": "<main_entry>",
        "section_title": "SYSTEM LOG", "section_ordinal": 0,
        "line_start": 9116, "line_end": 53708,
        "captured_at": "2026-09-24T00:27:01.751+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 15.249, "capture_kind": "live_log",
        "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "first and last crash lines both fall inside capture.log_window, and the 271st crash is the last line of the crash buffer, so the loop is still running at capture",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "logcat captures as the crashes occur; the dump neither causes nor suppresses them" } }
  ],

  "native_crash_groups": [
    { "group_id": "<native_group_id>",
      "language": "native",
      "signature": {
        "exception_class": null,
        "signal_number": <n>, "signal_name": "<SIGXXX>", "sigcode_name": "<SI_XXX>",
        "top_frame": "<demangled top frame, symbol only>",
        "abort_message_hash": "<hex>",
        "abort_message_varies": false
      },
      "packages": ["<package>"], "package_count": 1,
      "repeat_count": <n>,
      "raw_line_hits": <n>, "duplicate_hits": 0,
      "duplicate_detection": { "method": "same_pid_same_second", "duplicate_pids": [] },
      "first_seen": "<first_seen>",
      "last_seen": "<last_seen>",
      "window_seconds": <window_seconds>,
      "rate_per_second": <rate_per_second>,
      "inter_gap_p50_ms": null, "inter_gap_max_ms": null,
      "pid_span": { "first": null, "last": null, "distinct_count": null, "pid_max": <pid_max> },
      "classification": "stale_recurring",
      "classification.thresholds": { "repeat_count": <n>, "rate_per_second": <n>,
                                      "window_seconds": <n>, "inter_gap_max_ms": <ms>,
                                      "manager_kill_count": <n> },
      "classification_met": [],
      "manager_kill_count": 0, "manager_kill_reason": null,
      "rescueparty_failure_count": 0, "rescueparty_disabled": null,
      "corroboration": [ { "channel": "tombstone_files", "count": <n>, "count_kind": "logcat_lines_distinct" } ],
      "provenance": { "evidence_origin": "tombstone_files", "archive_path": "<main_entry>",
        "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "tombstone_header",
        "evidence_time_source": "embedded_trace_stamp",
        "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
        "freshness": "stale",
        "freshness_basis": { "input": "the tombstone's own in-file Timestamp line",
          "basis": "every tombstone in this group predates the capture by months; the tombstone_NN filename index is a ring counter and carries no time meaning, so only the in-file stamp is read",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "the tombstones were read from disk; dumpstate did not generate them" } }
  ],

  "java_crash_incidents": [
    { "incident_id": "crash-0001",
      "group_id": "<group_id>",
      "language": "java",
      "package": "<crashing_package>", "process": "<crashing_process>",
      "pid": <pid>, "prefix_pid": <pid>, "prefix_uid": <uid>,
      "thread_name": "<thread_name>",
      "exception_class": "<exception_class>",
      "exception_message": null,
      "top_frames": [
        "<frame_1_symbol>(<File>:<line>)",
        "<frame_2_symbol>(<File>:<line>)",
        "<frame_3_symbol>(<File>:<line>)",
        "<frame_4_symbol>(<File>:<line>)",
        "<frame_5_symbol>(<File>:<line>)"
      ],
      "occurred_at": "<occurred_at>",
      "flags_hex": "<hex>",
      "crash_handler": "<ApplicationInfo.Crash-Handler>",
      "evidence_channels": [
        { "channel": "crash_buffer", "join_basis": "same_logcat_line" },
        { "channel": "am_crash_event_log", "join_basis": "same_pid_and_window" },
        { "channel": "dropbox_crash", "join_basis": "same_pid_and_window" },
        { "channel": "am_proc_died", "join_basis": "same_pid_and_window" }
      ],
      "provenance": { "evidence_origin": "crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log",
        "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "inside capture.log_window; the inlined dropbox copy carries a Timestamp agreeing with the logcat line stamp to within a few ms",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "negative_evidence": [
    { "scope": "native tombstone search in the main entry",
      "statement": "0 occurrences of the tombstone banner in the main entry; the native crashes exist only as ZIP entries under FS/data/tombstones/",
      "source": { "archive_path": "<main_entry>", "section_title": null, "section_ordinal": null },
      "authority": "archive_absence" },
    { "scope": "am_crash_event_log channel coverage",
      "statement": "the event-log buffer ends before the crash buffer does, so this channel observes a bounded fraction of the loop and must not be used to bound anything",
      "source": { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "findings": [
    { "id": "crash-0002", "kind": "crash_loop",
      "severity": "critical", "unconstrained_severity": "critical", "confidence": "confirmed",
      "process_key": "<boot_epoch>/<start_seq>",
      "event_time": "<last_seen>",
      "attributes": { "package": "<crashing_package>", "language": "java",
                      "still_restarting_at_capture": true },
      "metrics": { "repeat_count": <n>, "window_seconds": <window_seconds>, "rate_per_second": <rate_per_second>,
                   "inter_gap_max_ms": <ms>, "manager_kill_count": <n>, "rescueparty_failure_count": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim crash-buffer line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "last crash logcat line",
          "basis": "the final occurrence is the last line of the crash buffer, so the loop is live at capture",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "id": "crash-0003", "kind": "signal_absence",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "tombstone_hits": <n>, "tombstone_real": 0, "signal_hits": <n>, "signal_fatal": 0 },
      "metrics": { "tombstone_files_in_zip": <n> },
      "evidence": { "archive_path": "FS/data/tombstones/tombstone_00", "kind": "file_entry",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "Timestamp: <YYYY-MM-DD HH:MM:SS.nnnnnnnnn+ZZZZ>\nCmdline: <package>\n…",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "tombstone_files",
        "archive_path": "FS/data/tombstones/tombstone_00",
        "captured_at": "<captured_at>", "captured_at_time_base": "tombstone_header",
        "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact", "freshness": "stale",
        "freshness_basis": { "input": "in-file Timestamp line",
          "basis": "searched and found: the raw tombstone matches in the main entry yield 0 real tombstones; the real ones are ZIP entries far older than this boot",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "evidence_gaps": [
    { "field": "counts.am_crash_hits.coverage.coverage_ratio", "reason": "REGION_EXCLUDED",
      "detail": "the event-log buffer retains only a bounded interval of the log window, so this channel observes a fraction of the loop. The ratio is published on the match block; no field may bound the crash count from it." },
    { "field": "java_crash_incidents[].evidence_channels[].join_basis", "reason": "UNVERIFIABLE",
      "detail": "the event-log and dropbox records carry no key that joins them to a logcat crash line. Join basis is therefore same_pid_and_window, which is a heuristic, and any finding resting only on it must carry confidence: inferred." },
    { "field": "am_crash_event_log slot layout", "reason": "UNVERIFIABLE",
      "detail": "the payload is positional and its field order is declared in the platform tag file, which no bugreport contains. pid is taken from the crash buffer and cross-checked against the logcat prefix; no slot is decoded on the strength of a declaration the analyzer cannot read." },
    { "field": "native_crash_groups[].pid_span", "reason": "UNVERIFIABLE",
      "detail": "tombstone pids belong to a previous boot and are meaningless against this boot's pid space, so every member is null. Publishing the numbers would invite a comparison that has no meaning." },
    { "field": "native_crash_groups[].signature.abort_message_hash", "reason": "PARSE_INDETERMINATE",
      "detail": "the abort message is free text that changes between builds, so it is hashed rather than compared literally. Where the hash varies within a group, abort_message_varies is true and the group is sub-divided by top frame; the message itself is never published as a grouping key." }
  ],

  "notes": [
    "CRASH_COUNT_IS_DISTINCT: the raw FATAL EXCEPTION count decomposes across three archive regions; the duplicates in the middle region are found by (pid, second) and suppressed. The published repeat_count is the de-duplicated count, and raw_line_hits minus duplicate_hits equals it.",
    "CRASH_COUNT_CHANNEL_IS_CRASH_BUFFER: only the crash buffer spans the whole log window with no rate limit. The event-log channel observes a fraction of it; dropbox is rate limited; the count authority is published as count_authority.",
    "NO_PID_RECYCLING_IN_LOOP: the distinct crashes produced as many distinct, monotonic pids as there were crashes. The apparent repeats were second renderings, not kernel pid recycling. process_key remains the identity because pid is still an observation attribute."
  ]
}
```

### Open questions

- **No `AM_CRASH` write site.** `grep -rl AM_CRASH frameworks/base/` returns **zero** files of
  any type, and there is no `AM_CRASH` in any `.java` under `system/` or `packages/modules/`.
  The tag is declared at `EventLogTags.logtags:56` and is demonstrably emitted (39 entries in
  the archive), so the emitter lives outside the searched trees or behind a generated alias.
  **The schema therefore decodes `am_crash` slots 1–2 as `(pid, user_id)` by observation only
  and flags it in `channels[].decode_warning`; the authoritative pid comes from the
  `am_proc_died` / `am_proc_start` pairing.**
- **The `am_crash` decode trap, stated precisely.** `am_crash` is declared
  `(User, PID, …)` at `logtags:56`, but the values actually emitted are `(PID, User, …)`.
  Proven against three matched triplets read from the archive:

  ```
  am_proc_start: [0,1553,1000,com.android.car.cluster.osdouble,restart,…]   ← PID in slot 2
  am_crash:      [1553,0,com.android.car.cluster.osdouble,814267981,…]     ← PID in slot 1
  am_proc_died:  [0,1553,com.android.car.cluster.osdouble,-800,1]           ← PID in slot 2
  ```

  `am_proc_start` and `am_proc_died` match their declarations; `am_crash` does not. Decoding
  `am_crash` from the logtags file yields `user_id = 1553`, `pid = 0`.
- **Java crashes have no native record.** A Java crash is a *sequence of log lines* in the
  crash buffer — there is no per-crash file anywhere. `FATAL EXCEPTION:` from
  `RuntimeInit.java:92`, `Process: …, PID: …` from `:94-97`, the exception class and frames via
  `Clog_e` → `Log.printlns(Log.LOG_ID_CRASH, …)` at `:84-86`. The schema must carry the
  reconstruction's own confidence and the logcat line range, and must name `crash_buffer` as
  the only authoritative Java channel via `count_authority`.
- **The logcat field order was verified against the archive, not just the format string.** A
  `threadtime` line is `MM-DD HH:MM:SS.mmm  <uid>  <pid> <tid> <L> <tag>: <message>`
  (`logprint.cpp:1552-1554`). For uid 1000 the first numeric column is `683` on **6,788** lines
  (system_server's main thread) while the second varies across 7 values (`683, 710, 717, 718,
  862, 1001, 1462` = binder threads); a 1-minute sample confirms the same for uid 1003. Had the
  order been reversed, system_server would show seven different pids.
- **Native top-frame normalisation.** `signature.top_frame` needs demangling and
  `(sym+off)` stripping. The archive's frames are already trimmed at ~120 characters, so a
  frame read from the main entry is truncated. A real run must demangle from the full ZIP
  entry; the *schema* is unaffected.
- **`exception_message` is `null` for all 271.** The archive prints `NULL` in the `am_crash`
  event and logcat carries no message line for this throw. Emitting `null` beats inventing one.
- **`native_crash_incidents[].pid_span` is `null`.** Tombstone pids (e.g. 9452) are 126-day-old
  and meaningless against this boot's pid space. A `null` is better than a number that invites
  comparison.

---

