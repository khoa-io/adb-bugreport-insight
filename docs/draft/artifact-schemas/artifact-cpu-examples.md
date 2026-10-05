# `cpu.json` — worked examples and open questions

> **Draft.** See [`index.md`](./index.md).

The worked example and open questions of [`artifact-cpu.md`](./artifact-cpu.md), split out because
together they exceed the 500-line limit. Heading levels are promoted one step; the content is
unchanged.#

### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "cpu",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "cpu_clean_channels": ["dumpsys_cpuinfo"],
    "cpu_perturbed_channels": ["top", "iotop", "dumpsys_meminfo", "bugreport_procdump"],
    "page_scanning_sections": [
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true },
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true }
    ],
    "page_scanning_total_ms": <page_scanning_total_ms>
  },

  "utilization": [
    { "channel": "dumpsys_cpuinfo", "line_start": <line>,
      "window": { "start_utc": "<captured_at>", "end_utc": "<captured_at>", "duration_ms": <ms> },
      "window_source": "section_body",
      "age_at_print_ms": { "start": <ms>, "end": <ms> },
      "total_pct": <pct>, "pct_user": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
      "pct_irq": <pct>, "pct_softirq": <pct>,
      "pct_nice": null, "pct_steal": null,
      "online_cpus": <n>,
      "null_reasons": {
        "pct_nice": "the aggregate cpu row this channel prints has no nice bucket; the ratio is structurally unavailable from these bytes",
        "pct_steal": "same; steal is only readable per-CPU via the cpu_monitor channel, never in this aggregate row"
      },
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "background_sampler",
        "freshness": "in_window",
        "freshness_basis": { "input": "the window header this section prints",
          "basis": "the sample window closed before dumpstate started and also falls inside capture.log_window, so it cannot include collection cost",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "channel": "top", "line_start": <line>,
      "window": { "start_utc": null, "end_utc": null, "duration_ms": null },
      "window_source": "unknown",
      "total_pct": null,
      "pct_user": <pct>, "pct_nice": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
      "pct_irq": <pct>, "pct_softirq": <pct>, "pct_steal": <pct>,
      "online_cpus": <n>,
      "threads_total": <n>, "running": <n>, "sleeping": <n>, "stopped": <n>, "zombie": <n>,
      "null_reasons": {
        "total_pct": "the aggregate row is normalised across all online cores, so it is not a 0-100 busy figure and cannot be compared with the other channel",
        "window": "this channel prints no window header, so its sampling interval is not derivable from the bytes"
      },
      "semantics_verified": false,
      "provenance": { "evidence_origin": "top", "section_title": "CPU INFO", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "the reading carries no printed clock; positioned only by its line_start within the archive",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "per_cpu_core": [
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> }
  ],

  "processes": [
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "minor_faults": <n>, "major_faults": <n>, "is_thread_row": false,
      "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "minor_faults": <n>, "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>",
      "pct_total": <pct>, "minor_faults": <n>, "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<thread_name>", "pct_total": <pct>,
      "is_thread_row": true, "window_ref": "<utilization[].window_ref>" }
  ],

  "contention": {
    "art_java_monitor": {
      "event_count": <n>,
      "channel_present": true,
      "threshold_ms": <ms>,
      "threshold_derivation": "the reporting floor is a platform constant scaled by a build-variant factor. On this build the floor is high enough that a stall well below it emits no line at all, so the absence of short stalls is NOT evidence of health. The exact figure must be read from the build configuration, not assumed.",
      "wait_ms_max": <ms>, "wait_ms_min": <ms>,
      "events_over_threshold_1s": <n>,
      "events_in_system_server": <n>,
      "events_over_threshold_1s_in_system_server": <n>,
      "events": [
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_name": "<owner_name>", "owner_tid": <tid>,
          "owner_method": "<owner_method>(<signature>)",
          "owner_file": "<owner_file>", "owner_line": <n>,
          "waiters": <n>, "waiter_method": "<waiter_method>(<signature>)",
          "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim as ART chose to print it>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_method": "<owner_method>(<signature>)",
          "owner_line": <n>, "waiters": <n>, "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_method": "<owner_method>(<signature>)",
          "owner_line": <n>, "waiters": <n>, "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_line": <n>, "waiters": <n>,
          "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" }
      ],
      "events_truncated_in_example": <n>
    },
    "art_runtime_mutex": {
      "channel_present": true, "event_count": 0,
      "format": "<the channel's full line format, as documented above>",
      "short_format": "<the channel's abbreviated line format>",
      "zero_reason": "the channel is enabled and was searched, but no event satisfied its rate limit during the log window. Presence-with-zero is a result, not an absence."
    },
    "art_rate_limiting": {
      "java_monitor_is_sampled": true,
      "java_monitor_sampling_rule": "the emission probability is proportional to how far the wait exceeded the configured threshold, so a short wait is logged far less often than a long one",
      "java_monitor_sample_pct_observed": [<n>],
      "java_monitor_threshold_source": "a platform build option; not readable from a bugreport, so threshold_ms above must be published with threshold_derivation",
      "runtime_mutex_is_rate_limited": true,
      "runtime_mutex_rate_limit": "a fixed minimum interval between emitted events, scaled by a per-monitor attempt counter",
      "counting_caveat": "no field in contention is a contention count. The sampled channel's true population is unbounded below because sampling probability varies per wait duration, and the Java-monitor channel has a reporting floor above which shorter waits are unobservable. The published event_count is a count of lines, never of stalls."
    },
    "dvm_lock_sample": {
      "sampled_event_count": <n>, "in_system_server": <n>, "system_server_pct": <pct>,
      "field_order_source": "the positional layout this artifact assumes; it is declared in the platform's event-log tag file, which no bugreport contains, so each entry is also published in decomposed form",
      "example": { "timestamp_utc": "<captured_at>", "pid": <pid>, "tid": <tid>,
        "process_name": "<process_name>", "is_sensitive_thread": <n>, "thread_name": "<thread_name>",
        "wait_ms": <ms>, "waiter_file": "<File>", "waiter_line": <n>,
        "waiter_method": "<return type> <class>.<method>(<arg types>)",
        "owner_file": "-", "owner_line": <n>,
        "owner_method": "<return type> <class>.<method>(<arg types>)",
        "sample_percent": <n>,
        "owner_file_note": "an owner_file of '-' is a space optimisation meaning owner_file == waiter_file" }
    },
    "provenance": { "evidence_origin": "art_monitor_contention", "archive_path": "<main_entry>",
      "section_title": "SYSTEM LOG", "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
      "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
      "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "in_window",
      "freshness_basis": { "input": "logcat line stamp",
        "basis": "contention events are logged as they occur, inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "live_log_capture" }
  },

  "load": {
    "sources": [
      { "source": "bugreport_header", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>,
        "uptime_ms_at_read": null, "uptime_qualifier": "<uptime as printed in the header>",
        "line_start": <line> },
      { "source": "uptime_section", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>,
        "uptime_qualifier": "<uptime as printed>", "captured_at": "<captured_at>", "line_start": <line> },
      { "source": "dumpsys_cpuinfo", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>, "line_start": <line> }
    ],
    "online_cpus": <n>,
    "uptime_ms_at_read": <ms>,
    "load_15m_implies_uptime_ms": <ms>,
    "load_history_conflict": true,
    "provenance": { "evidence_origin": "loadavg", "captured_at": "<captured_at>",
      "captured_at_time_base": "dumpsys_section_snapshot", "age_seconds_at_capture": <age>,
      "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "section body",
        "basis": "a kernel exponentially-weighted average, not a dump-scanned value",
        "threshold_seconds": 3600 },
      "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" }
  },

  "stall_signals": {
    "psi_cpu": { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                 "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "psi_io":  { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                 "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "iotop": { "window_ms": <ms>, "row_granularity": "thread", "sort_column": "<column>",
      "max_io_delay_pct": <pct>, "max_swap_delay_pct": <pct>,
      "max_sched_delay_pct": <pct>, "max_mem_delay_pct": <pct>,
      "resolution_floor_note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O; the same rows carry non-zero throughput",
      "provenance": { "capture_kind": "command_output", "captured_at": null, "freshness": "undated",
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } },
    "proc_churn": {
      "am_proc_start_count": <n>, "am_proc_died_count": <n>, "am_proc_bound_count": <n>,
      "am_uid_running_count": <n>, "am_uid_active_count": <n>,
      "window": { "start_utc": "<captured_at>", "end_utc": "<captured_at>",
                  "duration_seconds": <seconds>, "coverage_ratio": <coverage_ratio> },
      "rate_per_s": null,
      "rate_valid": false,
      "rate_suppressed_reason": "the event-log buffer covers only a fraction of the log window, and a per-boot churn rate computed over a window this short is dominated by the initial burst of runtime startup. The arithmetic result is withheld rather than published with a caveat, because rate_valid gates rate_per_s.",
      "provenance": { "capture_kind": "live_log", "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line", "freshness": "in_window",
        "collector_perturbed": false,
        "collector_perturbed_basis": "live_log_capture" } }
  },

  "graphics": {
    "device_uptime_ms_at_capture": <ms>,
    "per_process": [
      { "pid": <pid>, "process_name": "<process_name>",
        "stats_since_ns": <ns>, "total_frames": <n>, "janky_frames": <n>, "janky_pct": <pct>,
        "p50_ms": <ms>, "p90_ms": <ms>, "p95_ms": <ms>, "p99_ms": <ms> }
    ],
    "window_note": "the per-process statistics window is independent per process and is not the same as utilization[].window; compare them only after reading stats_since_ns against device_uptime_ms_at_capture",
    "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "section_duration_ms": <ms>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": false, "collector_perturbed_basis": "section_measured_before_dump" }
  },

  "power_history": {
    "batterystats_reset_time": "<YYYY-MM-DD-HH-mm-ss>",
    "batterystats_duration_s": <seconds>,
    "batterystats_history_used_pct": <pct>,
    "powerstats_earliest_data_time": "<captured_at>",
    "freshness": "stale",
    "quarantine_reason": "battery and power history both begin before this boot. They describe a previous boot, are stored here only so a reader can see they exist, and MUST NOT be joined to utilization[] or to any uptime-derived field.",
    "provenance": { "capture_kind": "persisted_aggregate", "captured_at": "<captured_at>",
      "captured_at_time_base": "dumpsys_section_snapshot", "freshness": "stale" }
  },

  "counts": {
    "dvm_lock_sample_hits": {
      "pattern": " dvm_lock_sample:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "these lines are NOT contention counts. They are a duration-weighted probability sample and are reported separately under contention.dvm_lock_sample as sampled_event_count",
      "coverage": { "window_start": "<captured_at>", "window_end": "<captured_at>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    },
    "long_monitor_contention_hits": {
      "pattern": "Long monitor contention", "pattern_kind": "substring", "case_sensitive": false,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "<captured_at>", "window_end": "<captured_at>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    },
    "monitor_contention_with_tid_hits": {
      "pattern": "Monitor contention with tid", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the channel is present and enabled; zero events satisfied its rate limit. Presence-with-zero is a result.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "held_by_thread_hits": {
      "pattern": "held by thread", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the single match falls inside an archived trace section that predates this boot by months. Reporting it as a live lock-contention finding is the exact false positive this schema exists to prevent.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    }
  },

  "findings": [
    { "id": "cpu-001", "kind": "clean_window",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "channel": "dumpsys_cpuinfo", "capture_kind": "background_sampler" },
      "metrics": { "window_duration_ms": <ms>, "age_before_dump_ms": <ms>,
                   "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
                   "pct_irq": <pct>, "pct_softirq": <pct> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim CPU-usage window header>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "the window header plus the section footer",
          "basis": "the window closed before dumpstate started and inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "id": "cpu-002", "kind": "host_interrupt_injection",
      "severity": "info", "unconstrained_severity": "info", "confidence": "inferred",
      "inference": true,
      "process_key": null, "event_time": null,
      "attributes": { "inference_basis": ["ro.ranchu", "<fingerprint emulator segment>",
                                          "steal_time_ms = 0 on every core", "top reporting 0%host"] },
      "metrics": { "pct_irq": <pct>, "steal_time_ms": <ms>, "cores_measured": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim per-core usage row>",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpu_monitor", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "see cpu-001" } },

    { "id": "cpu-003", "kind": "load_history_conflict",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "load_history_conflict": true },
      "metrics": { "load_1m": <n>, "load_5m": <n>, "load_15m": <n>, "online_cpus": <n>,
                   "uptime_ms_at_read": <ms>, "load_15m_implies_uptime_ms": <ms> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim header uptime and load-average line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpstate_header",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "now",
        "freshness_basis": { "input": "the uptime the header itself prints",
          "basis": "printed in the archive header itself",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" } },

    { "id": "cpu-004", "kind": "contention_localized",
      "severity": "medium", "unconstrained_severity": "medium", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "localized_to": ["<process_name>", "<process_name>"],
                      "owner_method": "<owner_method>(<signature>)" },
      "metrics": { "art_java_monitor_event_count": <n>, "wait_ms_max": <ms>, "wait_ms_min": <ms>,
                   "events_over_threshold_1s": <n>, "events_in_system_server": <n>,
                   "dvm_lock_sample_sampled_event_count": <n>, "dvm_lock_sample_max_wait_ms": <ms> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim contention line>",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "art_monitor_contention", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "in_window",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "logged as they occur, inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" } },

    { "id": "cpu-005", "kind": "no_io_stall",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O; the same table carries non-zero throughput" },
      "metrics": { "max_sched_delay_pct": <pct>, "max_io_delay_pct": <pct>,
                   "psi_io_full_avg10": <pct>, "psi_cpu_full_avg10": <pct> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim iotop row>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "iotop", "section_title": "IOTOP", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "the reading carries no printed clock; its window comes from the tool's own sampling loop, not from a stamp",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "evidence_gaps": [
    { "field": "utilization[].pct_steal", "reason": "NOT_PRINNED",
      "detail": "the aggregate CPU row this channel prints has no steal bucket, so steal is structurally unavailable from these bytes. It is never defaulted to 0; where the aggregate channel omits it, the value is null with a stated reason and steal is read per-CPU from the cpu_monitor channel instead." },
    { "field": "utilization[].pct_nice", "reason": "NOT_PRINNED",
      "detail": "same structural absence as steal for the aggregate row. Note the top channel does print a nice column, so the two entries of this array legitimately disagree on which buckets exist." },
    { "field": "contention.art_java_monitor.threshold_ms", "reason": "UNVERIFIABLE",
      "detail": "the reporting floor is a platform constant scaled by a build-variant factor. No bugreport records either value, so a tool cannot derive the floor from the archive; it must be supplied by configuration and published alongside threshold_derivation. Its practical consequence is that stalls below the floor are unobservable and their absence is not evidence of health." },
    { "field": "contention.dvm_lock_sample.example field order", "reason": "UNVERIFIABLE",
      "detail": "the payload is positional and its layout is declared in the platform event-log tag file, which no bugreport contains. The example is published in decomposed form so a reader need not trust the assumed slot order to use the numbers." },
    { "field": "load.load_15m_implies_uptime_ms", "reason": "AMBIGUOUS_SOURCE",
      "detail": "load_15m implies far more history than the printed uptime admits, and nothing in the archive adjudicates why — whether the kernel does not reset the load average across the first userspace processes or seeds the average from a previous boot. load_history_conflict publishes the conflict and load_1m must never be reported without uptime_ms_at_read." },
    { "field": "stall_signals.proc_churn.rate_per_s", "reason": "REGION_EXCLUDED",
      "detail": "the event-log buffer retains only a fraction of the log window, and a rate computed over so short an interval is dominated by the startup burst rather than by steady-state churn. rate_valid is false and the arithmetic result is withheld rather than published with a caveat." },
    { "field": "power_history", "reason": "UNVERIFIABLE",
      "detail": "both power sections describe a previous boot. They are quarantined in their own top-level object so that no join to utilization[] or to any uptime-derived field is structurally possible." }
  ],

  "negative_evidence": [
    { "scope": "ART runtime-mutex contention channel",
      "statement": "channel enabled and searched; 0 'Monitor contention with tid' events satisfied its rate limit during the log window",
      "source": { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "notes": [
    "CPUINFO_WINDOW_PREDATES_DUMP: the dumpsys cpuinfo window closed before dumpstate started, because this channel prints pre-existing background samples rather than taking a fresh one. collector_perturbed is false for this channel and true for top, iotop, dumpsys_meminfo and bugreport_procdump.",
    "NO_STEAL_IN_GLOBAL_RATIO: the aggregate CPU row structurally cannot contain steal. pct_steal is null for that channel with null_reasons stating why; it is never 0 by default.",
    "CHANNELS_DISAGREE_BY_DESIGN: two CPU channels disagree because they measure different windows against different denominators. They are two entries of utilization[] and are never merged; neither is corrected toward the other.",
    "CONTENTION_IS_NEVER_A_COUNT: no field in contention is a count of stalls. The sampled channel's true population is unbounded below because sampling probability varies per wait duration, and the Java-monitor channel has a reporting floor above which shorter waits are unobservable.",
    "HELD_BY_THREAD_IS_STALE: the only match for this pattern in the archive falls inside an archived trace section that predates this boot by months, so it is counted as stale_region rather than as live contention.",
    "PROC_CHURN_RATE_GATED: the arithmetic per-boot churn rate over so short and so poorly covered a window is withheld rather than published with a caveat. rate_valid is false and rate_per_s is null.",
    "POWER_HISTORY_QUARANTINED: battery and power history describe a previous boot and are never joined to utilization[] or to any uptime-derived field."
  ]
}
```

### Open questions

- **`top` and `procrank` semantics are unverified.** Neither binary is in this source drop. For
  `top` only the invocation is citable (`dumpstate.cpp:1690-1691`), so column names are read off
  the output header rather than a format string. Specifically unverified: whether `%CPU` is
  scaled by thread count (`-H` is passed), the exact meaning of `0%host`, and the denominator
  behind `400%cpu`. Every `top`-derived field therefore carries `semantics_verified: false`.
- **No per-UID CPU channel exists.** `/proc/uid_stat` does not exist in Android 15 — no producer
  or consumer in this tree — and `ProcessCpuTracker` derives per-UID CPU by summing per-process
  deltas from `/proc/<pid>/stat` (`:57-78, :353`). The task's "per-UID accounting" does not exist
  as a separate evidence source here. **No `per_uid[]` array was created.** The nearest thing is
  `am_uid_running` (68) / `am_uid_active` (64), which are *state* events, not time accounting.
- **`rate_per_s` gating is a hard-won lesson.** A first draft computed `121 / 64.662 = 1.873`
  and set `rate_valid: true`, which is wrong. The `EVENT LOG` window is 64.662 s, well under the
  120 s floor the field's own rule requires. It is recorded here because it is exactly the kind
  of error a "compute every available number" implementation makes silently, and exactly what
  the `matched_evidence` / `suppressed_noise` / `coverage_ratio` requirements exist to catch.
- **`busy_pct_since_boot` is pure derivation and its denominator is a construction.** busy
  (user+nice+system+iowait+irq+softirq) ÷ sum of all buckets including `guest_time_ms` and
  `guest_nice_time_ms`. Both guest buckets are 0 on this device. A first pass mis-added the
  buckets and shipped wrong values; the corrected values are what the worked example carries.
  AOSP publishes only raw milliseconds (`CpuInfoReader.java:859-880`) and never a busy
  percentage, so **any implementation computing it needs a unit test against a hand-summed row.**
- **`per_cpu_core[].is_online` and the frequency fields** are parsed from a C++ `CpuInfo{…}`
  `toString` whose formatter was not located in this drop. The field names are read off the
  output.
- **The ART monitor threshold depends on `kIsDebugBuild`, not on `ro.build.type`.** A 1000 ms
  floor on this `userdebug` build. Whether a `user` build of the same image would show a 100 ms
  floor **was not verified**, and since `lock_profiling_threshold_` is zero on user builds the
  whole branch is skipped anyway (`monitor.cc:458`) — so on a `user` build this channel emits
  nothing at all, and `channel_present: false` is the correct status there.
- **`is_debuggable` flags were raised by the Build System Developer and not cross-checked here.**
  The related, verifiable fact is that `ro.debuggable` and `ro.secure` are never consulted by
  dumpstate — see [§3.1](./build-variant.md#31-isuserbuild-is-the-single-primitive).

---

