# `memory.json` — worked examples and open questions

> **Draft.** See [`index.md`](./index.md).

The worked example and open questions of [`artifact-memory.md`](./artifact-memory.md), split out
because together they exceed the 500-line limit. Heading levels are promoted one step; the content
is unchanged.#

### Worked example

Abbreviated; every value was measured on the reference archive.

```jsonc
{
  "schema_version": 1,
  "artifact": "memory",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "page_scanning_sections": [
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true },
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true }
    ],
    "page_scanning_total_ms": <page_scanning_total_ms>,
    "swap_timeline": [
      { "channel": "proc_meminfo", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": true,
        "zram_physical_kb": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "proc_meminfo", "section_title": "MEMORY INFO", "section_ordinal": 0,
          "line_start": <line>, "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none",
            "basis": "a raw /proc/meminfo read carries no clock and dumpstate suppresses the mtime for procfs paths, so this reading is undated",
            "threshold_seconds": 3600 },
          "collector_perturbed": false, "collector_perturbed_basis": "undated_source" } },
      { "channel": "top", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "top", "section_title": "CPU INFO", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock", "threshold_seconds": 3600 },
          "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "bugreport_procdump", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": <zram_physical_kb>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "procrank", "section_title": "PROCRANK", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock",
            "threshold_seconds": 3600 },
          "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "dumpsys_meminfo", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": <zram_physical_kb>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
          "section_ordinal": 0, "line_start": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
          "freshness_basis": { "input": "the section footer's ending-at stamp",
            "basis": "inside capture.log_window", "threshold_seconds": 3600 },
          "collector_perturbed": true,
          "collector_perturbed_basis": "section_measured_during_dump" } }
    ],
    "swap_growth_kb": <swap_growth_kb>,
    "page_scanning_ms_in_interval": <page_scanning_ms_in_interval>
  },

  "system": {
    "total_ram_kb": <total_ram_kb>,
    "memory_level": "<status, verbatim>",
    "free_ram_kb": <free_ram_kb>,
    "free_ram_components_kb": { "cached_pss": <kb>, "cached_kernel": <kb>, "free": <kb> },
    "free_pct": <free_pct>,
    "used_ram_kb": <used_ram_kb>,
    "used_pss_kb": <used_pss_kb>,
    "kernel_used_kb": <kernel_used_kb>,
    "total_pss_kb": <total_pss_kb>,
    "lost_ram_kb": <lost_ram_kb>,
    "lost_ram_pct": <lost_ram_pct>,
    "tuning": { "memory_class_mb": <n>, "large_memory_class_mb": <n>, "oom_adj_score_kb": <kb>,
                "restore_limit_kb": <kb>, "low_ram": null, "high_end_gfx": true },
    "gpu_kb": <gpu_kb>,
    "dma_buf": { "total_kb": <kb>, "mapped_kb": <kb>, "unmapped_kb": <kb>, "heaps_kb": <kb>, "heaps_pool_kb": <kb> },
    "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
      "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
      "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "the section footer's ending-at stamp",
        "basis": "the body prints an uptime and realtime reading, so the reading cannot outlive this boot and falls inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "swap": {
    "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_pct": <pct>,
    "zram_physical_kb": <zram_physical_kb>, "zram_physical_pct_of_ram": <pct>,
    "compression_ratio": <compression_ratio>, "compression_ratio_derived": true,
    "zram_block_io": { "read_ios": <n>, "read_sectors": <n>, "write_ios": <n>,
                       "write_sectors": <n>, "io_ticks": <n>,
                       "window": "cumulative since boot", "line_start": <line> },
    "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "processes": [
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "zswap_kb": <zswap_kb>, "zswap_estimated": true,
      "rss_total_kb": <rss_total_kb>, "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "rss_total_kb": <rss_total_kb>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "rss_total_kb": <rss_total_kb>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "bugreport_procdump", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": 0, "rss_total_kb": <rss_total_kb>,
      "source_channel": "bugreport_procdump", "line_start": <line> },
    { "pid": <pid>, "process_name": "dumpstate", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": 0, "source_channel": "dumpsys_meminfo", "line_start": <line> }
  ],

  "categories": [
    { "category": "Native",      "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".apk mmap",   "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Dalvik",      "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".Heap",       "depth": 1, "parent_category": "Dalvik",   "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Dalvik Other","depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Unknown",     "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".so mmap",    "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> }
  ],

  "procstats": [
    { "sub_section": "CURRENT", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "period_total_uptime": "<uptime as printed>",
      "period_complete": false, "period_flags": ["partial", "swapped-out-pss"], "aggregated_over_n": <n>,
      "checkin_period": { "start_clock": "<YYYY-MM-DD-HH-mm-ss>", "start_realtime_ms": <ms>,
                          "end_realtime_ms": <ms>, "complete": false, "window_ms": <ms> },
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "a single sample whose window falls inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "sub_section": "AGGREGATED_24H", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "aggregated_over_n": <n>,
      "period_complete": false, "period_flags": ["partial"],
      "backing_state_file": "<path to the on-disk state file the block was read from>",
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start precedes this boot by months; the window size in the header is a query parameter, not a fact about the data. capture_kind persisted_aggregate is the deciding input, not the wall clock alone.",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } },

    { "sub_section": "AGGREGATED_3H", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "aggregated_over_n": <n>,
      "period_complete": false, "period_flags": ["partial"],
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start is identical to the longer-window block above, so the label in the header is not supported by the data",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } }
  ],

  "pressure_indicators": {
    "psi_memory": { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                    "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "psi_io":     { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                    "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "vmstat_oom_kill": <n>,
    "vmstat_pswpin": <n>, "vmstat_pswpout": <n>,
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "age_seconds_at_capture": null, "freshness": "undated",
      "freshness_basis": { "input": "none",
        "basis": "kernel cumulative counters carry no printed clock; they cannot outlive this boot but cannot be placed in the log window either",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "undated_source" }
  },

  "kills": [],
  "kill_evidence": {
    "lmkd_alive": true, "lmkd_using_psi": true, "lmkd_kill_log_matches": <n>,
    "vmstat_oom_kill": <n>, "am_kill_count": <n>,
    "am_low_memory_count": <n>, "am_low_memory_value": <n>,
    "am_meminfo_count": <n>, "am_pss_count": <n>,
    "provenance": { "capture_kind": "live_log", "captured_at": "<captured_at>",
      "captured_at_time_base": "logcat_line", "freshness": "in_window", "collector_perturbed": false }
  },

  "io": {
    "iotop": { "window_ms": <ms>, "line_start": <line>, "section_duration_ms": <ms>,
      "row_granularity": "thread", "sort_column": "<column>",
      "max_io_delay_pct": <pct>, "max_swap_delay_pct": <pct>,
      "max_sched_delay_pct": <pct>, "max_mem_delay_pct": <pct>,
      "resolution_floor_note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O. The same rows carry non-zero throughput, so a zero delay percentage must not be read as an absence of wait." },
    "storaged": { "line_start": <line>, "basis": "cumulative since process start",
                  "top_reader": "<uid or process, verbatim> <rchar> fg_rchar / <bytes> fg_rbytes" },
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "freshness": "undated",
      "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock", "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "counts": {
    "am_kill_hits": {
      "pattern": "am_kill", "pattern_kind": "word_boundary", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": null,
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
      "regions": []
    }
  },

  "findings": [
    { "id": "mem-001", "kind": "no_pressure",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "memory_level": "<status, verbatim>" },
      "metrics": { "free_pct": <free_pct>, "psi_memory_full_avg10": <pct>, "vmstat_oom_kill": <n>, "lmkd_kill_count": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim total-RAM line carrying the status>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },

    { "id": "mem-002", "kind": "swap_delta",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "page_scanning_ms_in_interval": <page_scanning_ms_in_interval> },
      "metrics": { "swap_growth_kb": <swap_growth_kb>, "page_scanning_total_ms": <ms>, "vmstat_oom_kill": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim swap/ZRAM summary line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "collection_swap_timeline", "archive_path": "<main_entry>",
        "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "this is a delta between two snapshots; neither endpoint carries a clock, so the EDGE is undated even though its endpoints are ordered by line_start",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } },

    { "id": "mem-003", "kind": "stale_aggregate",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "capture_kind": "persisted_aggregate" },
      "metrics": { "age_seconds": <age>, "aggregated_over_n": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim procstats header block>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_procstats", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start precedes this boot by months; capture_kind persisted_aggregate is the deciding input, not the wall clock alone",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } },

    { "id": "mem-004", "kind": "largest_allocator",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": "<boot_epoch>/<start_seq>", "event_time": null,
      "attributes": { "package_name": "<package_name>" },
      "metrics": { "pss_total_kb": <pss_total_kb>, "total_pss_kb": <total_pss_kb>, "native_category_kb": <pss_total_kb> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim per-process PSS row>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "evidence_gaps": [
    { "field": "kill_evidence.am_meminfo_count", "reason": "REGION_EXCLUDED",
      "detail": "the event log is captured before the memory dump that writes this tag, so a zero here is structural for any archive whose event log precedes its memory sections and says nothing about whether memory telemetry works." },
    { "field": "kill_evidence.am_low_memory_count", "reason": "UNVERIFIABLE",
      "detail": "the byte value is read, but its meaning depends on a platform writer the analyzer cannot inspect. It must not be read as memory exhaustion without cross-checking memory_level and the lmkd kill count, which is why no interpreted field is published for it." },
    { "field": "collection.swap_timeline[].captured_at", "reason": "NOT_PRINNED",
      "detail": "a raw procfs read and a top footer print no clock, and dumpstate suppresses the mtime for procfs and sysfs paths. These readings are undated rather than stale; a snapshot that carries no clock is unlabelled, and collapsing undated into stale would make legitimate evidence look like historical residue." },
    { "field": "collection.swap_timeline[] ordering", "reason": "UNVERIFIABLE",
      "detail": "dumpstate runs sections on a thread pool and archive order is completion order, so no statement-execution order can be recovered from any archive. The timeline is therefore ordered by line_start and nothing claims to know the collector's intended order." },
    { "field": "processes[].zswap_kb", "reason": "PARSE_INDETERMINATE",
      "detail": "the per-process ZSwap column is a device-wide compression ratio apportioned across processes, not a measurement of that process. Every row therefore carries zswap_estimated: true and no consumer may treat the number as this process's own swapped bytes." },
    { "field": "swap.zram_orig_size_kb / zram_compressed_size_kb", "reason": "NOT_PRINNED",
      "detail": "no bugreport section exposes the original or compressed data size for a zram device, so no such field exists in this schema. Only the physical bytes used are published." }
  ],

  "notes": [
    "SWAP_DELTA_OVERLAPS_PAGE_SCANNING: a measurable amount of the swap growth falls between two identified timeline nodes whose interval contains page-scanning sections. The overlap is published as page_scanning_ms_in_interval; whether it explains the delta is a causal claim and belongs in README.md, not in this artifact.",
    "PROCSTATS_LABELS_ARE_QUERY_PARAMETERS: the longer and shorter aggregate headers resolve to the same printed start time, so the window named in the header is a query parameter rather than a fact about the data.",
    "ZSWAP_IS_APPORTIONED_NOT_MEASURED: every zswap_kb in this artifact is a device-wide ratio apportioned per process. Each row carries zswap_estimated: true."
  ]
}
```

### Open questions

- **`top` and `procrank` binaries are not in this source drop** — no `system/core/toolbox/top.cpp`,
  no `system/core/toybox`, no `system/extras/procrank`. For `top` only the *invocation* is
  citable (`dumpstate.cpp:1690-1691`), so every `top`-derived field's semantics are marked
  **unverified**: whether `%CPU` is scaled by thread count (`-H` is passed), the exact meaning
  of `0%host`, and the denominator behind `400%cpu`. Column names are read off the output
  header, not off a format string.
- **ZRAM `orig_data_size` / `compr_data_size` are not exposed anywhere in a bugreport.**
  Deliberately unpopulatable, so no field was created. If a build exposes the raw `mm_stat` in
  a dumpsys section not found here, this should be revisited.
- **`/proc/uid_stat` does not exist in Android 15.** No producer or consumer in this tree;
  `ProcessCpuTracker` derives per-UID CPU by summing per-process deltas from `/proc/<pid>/stat`
  (`:57-78, :353`). There is therefore **no per-UID CPU channel** in `cpu.json` either — see
  [§4.7](./artifact-cpu.md#47-cpujson) open questions.
- **`am_kill`'s `Reason` string vocabulary is unresolved.** `EventLogTags.logtags:34` declares
  `Reason` as a string but no enum or writer populating it was found in this drop. The
  `killinfo` event's `enum kill_reasons` (`statslog.h:57-70`) is a *different*, numeric field.
  `kills[].reason_text` is therefore sourced from lmkd's own `ALOGI` (`lmkd.cpp:2485`), not from
  the event log.
- **`am_pss` has no located writer.** `EventLogTags.logtags:69` declares the tag; the
  `am_meminfo` writer (`AMS:12691`) was found but not `writeAmPs`. The tag count is 0 and the
  event log is captured before the memory dumps, so the schema does not change — but the
  producer cannot be cited.
- **`ProcessStats` internal bucket boundaries are unlocated.** `dumpAggregatedStats` buckets by
  *file age* (`aggregateHours*3600*1000 − COMMIT_PERIOD/2`, `ProcessStatsService.java:865-866`);
  the per-PSS-bucket windows (1/3/15 min) were not found in this tree, so
  `procstats[].aggregation_window` is built from the printed `Start time` / `Total elapsed time`
  / `Aggregated over` fields rather than from a declared bucket width.
- **`am_low_memory` semantics want a second opinion.** The reading encoded here is "the
  lowest-priority band is empty", not "memory exhaustion", because the archive shows
  `am_low_memory: 2` with `status normal` and zero lmkd kills. But the tag is *named*
  `am_low_memory` and a reader could reasonably read it the other way. The interpretation is
  therefore carried as mandatory prose (`am_low_memory_meaning`) rather than left implicit.
- **Which of two `/proc/meminfo` reads fired.** Two dumpstate call sites read `/proc/meminfo`
  (`dumpstate.cpp:1689` and `dumpstate.cpp:1951`) and the archive contains one `MEMORY INFO`
  block, so the tool cannot tell which produced it. **No field depends on the answer** —
  `swap_timeline` is ordered by `line_start` and nothing claims statement order — so this is
  recorded here as an open question rather than as an `evidence_gaps[]` entry, because it does
  not make any published value indeterminate.
- **`busy_pct`-style derived percentages need unit tests.** AOSP publishes only raw
  milliseconds (`CpuInfoReader.java:859-880`) and never a busy percentage, so any derived
  percentage needs a unit test against a hand-summed row. This was learned the hard way: a
  first pass mis-added the buckets and shipped wrong values.

---

