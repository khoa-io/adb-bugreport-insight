# `memory.json` — worked examples and open questions

> **Illustrative example for the promoted `UC-03` contract.**
> See [`index.md`](../index.md).

Examples/open questions from [`09-memory-artifact.md`](../09-memory-artifact.md), split at the 500-line limit.
Heading levels are promoted one step; the content is unchanged.

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
        "swap_total_kib": <swap_total_kib>, "swap_used_kib": <swap_used_kib>, "swap_used_kib_derived": true,
        "zram_physical_kib": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "proc_meminfo", "section_title": "MEMORY INFO", "section_ordinal": 0,
          "line_start": <line>, "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none",
            "basis": "a raw /proc/meminfo read carries no clock and dumpstate suppresses the mtime for procfs paths, so this reading is undated",
            "threshold_seconds": 3600 },
          "collector_perturbed": false, "collector_perturbed_basis": "undated_source" } },
      { "channel": "top", "line_start": <line>,
        "swap_total_kib": <swap_total_kib>, "swap_used_kib": <swap_used_kib>, "swap_used_kib_derived": false,
        "zram_physical_kib": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "top", "section_title": "CPU INFO", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock", "threshold_seconds": 3600 },
          "collector_perturbed": null, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "bugreport_procdump", "line_start": <line>,
        "swap_total_kib": <swap_total_kib>, "swap_used_kib": <swap_used_kib>, "swap_used_kib_derived": false,
        "zram_physical_kib": <zram_physical_kib>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "procrank", "section_title": "PROCRANK", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock",
            "threshold_seconds": 3600 },
          "collector_perturbed": null, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "dumpsys_meminfo", "line_start": <line>,
        "swap_total_kib": <swap_total_kib>, "swap_used_kib": <swap_used_kib>, "swap_used_kib_derived": false,
        "zram_physical_kib": <zram_physical_kib>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
          "section_ordinal": 0, "line_start": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
          "freshness_basis": { "input": "the section footer's ending-at stamp",
            "basis": "inside capture.log_window", "threshold_seconds": 3600 },
          "collector_perturbed": null,
          "collector_perturbed_basis": "section_measured_during_dump" } }
    ],
    "swap_growth_kib": <swap_growth_kib>,
    "page_scanning_ms_in_interval": <page_scanning_ms_in_interval>
  },

  "system": {
    "total_ram_kib": <total_ram_kib>,
    "memory_level": "<status, verbatim>",
    "free_ram_kib": <free_ram_kib>,
    "free_ram_components_kib": { "cached_pss": <kib>, "cached_kernel": <kib>, "free": <kib> },
    "free_ratio": <free_ratio>,
    "used_ram_kib": <used_ram_kib>,
    "used_pss_kib": <used_pss_kib>,
    "kernel_used_kib": <kernel_used_kib>,
    "total_pss_kib": <total_pss_kib>,
    "lost_ram_kib": <lost_ram_kib>,
    "lost_ram_pct": <lost_ram_pct>,
    "tuning": { "memory_class_mib": <n>, "large_memory_class_mib": <n>, "oom_adj_score_kib": <kib>,
                "restore_limit_kib": <kib>, "low_ram": null, "high_end_gfx": true },
    "gpu_kib": <gpu_kib>,
    "dma_buf": { "total_kib": <kib>, "mapped_kib": <kib>, "unmapped_kib": <kib>, "heaps_kib": <kib>, "heaps_pool_kib": <kib> },
    "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
      "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
      "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "the section footer's ending-at stamp",
        "basis": "the body prints an uptime and realtime reading, so the reading cannot outlive this boot and falls inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": null,
      "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "swap": {
    "swap_total_kib": <swap_total_kib>, "swap_used_kib": <swap_used_kib>, "swap_used_pct": <pct>,
    "zram_physical_kib": <zram_physical_kib>, "zram_physical_pct_of_ram": <pct>,
    "compression_ratio": <compression_ratio>, "compression_ratio_derived": true,
    "zram_block_io": { "read_ios": <n>, "read_sectors": <n>, "write_ios": <n>,
                       "write_sectors": <n>, "io_ticks": <n>,
                       "window": "cumulative since boot", "line_start": <line> },
    "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": null, "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "processes": [
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib>, "zswap_kib": <zswap_kib>, "zswap_estimated": true,
      "rss_total_kib": <rss_total_kib>, "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib>, "rss_total_kib": <rss_total_kib>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": null,
      "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib>, "rss_total_kib": <rss_total_kib>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "bugreport_procdump", "user_id": null,
      "pss_total_kib": <pss_total_kib>, "swap_pss_kib": 0, "rss_total_kib": <rss_total_kib>,
      "source_channel": "bugreport_procdump", "line_start": <line> },
    { "pid": <pid>, "process_name": "dumpstate", "user_id": null,
      "pss_total_kib": <pss_total_kib>, "swap_pss_kib": 0, "source_channel": "dumpsys_meminfo", "line_start": <line> }
  ],

  "categories": [
    { "category": "Native",      "depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": ".apk mmap",   "depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": "Dalvik",      "depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": ".Heap",       "depth": 1, "parent_category": "Dalvik",   "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": "Dalvik Other","depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": "Unknown",     "depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> },
    { "category": ".so mmap",    "depth": 0, "parent_category": null,        "pss_total_kib": <pss_total_kib>, "swap_pss_kib": <swap_pss_kib> }
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
      "collector_perturbed": null,
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
      "metrics": { "free_ratio": <free_ratio>, "psi_memory_full_avg10": <pct>, "vmstat_oom_kill": <n>, "lmkd_kill_count": <n> },
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
        "collector_perturbed": null, "collector_perturbed_basis": "section_measured_during_dump" } },

    { "id": "mem-002", "kind": "swap_delta",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "page_scanning_ms_in_interval": <page_scanning_ms_in_interval> },
      "metrics": { "swap_growth_kib": <swap_growth_kib>, "page_scanning_total_ms": <ms>, "vmstat_oom_kill": <n> },
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
        "collector_perturbed": null,
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
      "metrics": { "pss_total_kib": <pss_total_kib>, "total_pss_kib": <total_pss_kib>, "native_category_kib": <pss_total_kib> },
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
        "collector_perturbed": null, "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "evidence_gaps": [
    { "field": "kill_evidence.am_meminfo_count", "reason": "REGION_EXCLUDED",
      "detail": "zero retained rows are a bounded observation. AppProfiler can emit this tag before collection; sampling, retention and collection timing do not determine a unique reason for its absence." },
    { "field": "kill_evidence.am_low_memory_count", "reason": "UNVERIFIABLE",
      "detail": "the byte value is read, but its meaning depends on a platform writer the analyzer cannot inspect. It must not be read as memory exhaustion without cross-checking memory_level and the lmkd kill count, which is why no interpreted field is published for it." },
    { "field": "collection.swap_timeline[].captured_at", "reason": "NOT_PRINTED",
      "detail": "a raw procfs read and a top footer print no clock, and dumpstate suppresses the mtime for procfs and sysfs paths. These readings are undated rather than stale; a snapshot that carries no clock is unlabelled, and collapsing undated into stale would make legitimate evidence look like historical residue." },
    { "field": "collection.swap_timeline[] ordering", "reason": "UNVERIFIABLE",
      "detail": "dumpstate appends parallel task output when waiting on specific futures, so output order alone does not establish execution or completion order. The timeline is therefore ordered by line_start and nothing claims to know the collector's intended order." },
    { "field": "processes[].zswap_kib", "reason": "PARSE_INDETERMINATE",
      "detail": "the per-process ZSwap column is a device-wide compression ratio apportioned across processes, not a measurement of that process. Every row therefore carries zswap_estimated: true and no consumer may treat the number as this process's own swapped bytes." },
    { "field": "swap.zram_orig_size_kib / zram_compressed_size_kib", "reason": "NOT_PRINTED",
      "detail": "the inspected standard meminfo/procrank output does not expose these values; optional raw sysfs or OEM output may differ. Only the physical bytes used are published." }
  ],

  "notes": [
    "SWAP_DELTA_OVERLAPS_PAGE_SCANNING: a measurable amount of the swap growth falls between two identified timeline nodes whose interval contains page-scanning sections. The overlap is published as page_scanning_ms_in_interval; whether it explains the delta is a causal claim and belongs in README.md, not in this artifact.",
    "PROCSTATS_LABELS_ARE_QUERY_PARAMETERS: the longer and shorter aggregate headers resolve to the same printed start time, so the window named in the header is a query parameter rather than a fact about the data.",
    "ZSWAP_IS_APPORTIONED_NOT_MEASURED: every zswap_kib in this artifact is a device-wide ratio apportioned per process. Each row carries zswap_estimated: true."
  ]
}
```

### Open questions

- **top/procrank sources.**
  Toybox implements top in `external/toybox/toys/posix/ps.c` (`top_main`, `merge_deltas`, `top_common`).
  `-H` selects threads, percentages use one CPU as 100%, `400%cpu` uses configured CPU count, and `host` is the eighth proc/stat field (steal).
  Procrank is in `system/memory/libmeminfo/tools/procrank.cpp`, sharing `libsmapinfo/smapinfo.cpp` with bugreport_procdump.
  This establishes upstream semantics, not equivalence of an OEM binary.
- **ZRAM `orig_data_size` / `compr_data_size` are not exposed anywhere in a bugreport.**
  Deliberately unpopulatable, so no field was created.
  If a build exposes the raw `mm_stat` in a dumpsys section not found here, this should be revisited.
- **Per-UID CPU accounting exists.**
  `KernelCpuUidTimeReader.java:196-208` reads `/proc/uid_cputime/show_uid_stat`; `BatteryStats.java:6688-6698` prints per-UID CPU totals.
  These belong to battery accounting intervals, not the cpuinfo sampler window.
  Omitting `per_uid[]` here is draft scope; `/proc/uid_stat` is not proof that Android lacks CPU accounting.
- **am_kill/am_pss writers.**
  `ProcessRecord.java:1296-1297` emits am_kill with a caller-supplied reason and RSS; reasons are not a closed enum.
  `AppProfiler.java:917-919,983` writes am_pss and `:668,808` writes am_meminfo outside meminfo dumps.
  A zero retained count is not structurally required by collection order.
- **`ProcessStats` internal bucket boundaries are unlocated.**
  `dumpAggregatedStats` buckets by *file age* (`aggregateHours*3600*1000 − COMMIT_PERIOD/2`, `ProcessStatsService.java:865-866`); the per-PSS-bucket windows (1/3/15 min) were not found in this tree, so `procstats[].aggregation_window` is built from the printed `Start time` / `Total elapsed time` / `Aggregated over` fields rather than from a declared bucket width.
- **`am_low_memory` semantics want a second opinion.**
  The reading encoded here is "the lowest-priority band is empty", not "memory exhaustion", because the archive shows `am_low_memory: 2` with `status normal` and zero lmkd kills.
  The name `am_low_memory` permits another reading; require explicit `am_low_memory_meaning` prose.
- **Which of two `/proc/meminfo` reads fired.**
  Two dumpstate call sites read `/proc/meminfo` (`dumpstate.cpp:1689` and `dumpstate.cpp:1951`) and the archive contains one `MEMORY INFO` block, so the tool cannot tell which produced it.
  **No field depends on this:** `swap_timeline` uses `line_start` without execution-order claims; retain an open question, not `evidence_gaps[]`.
- **`busy_pct`-style derived percentages need unit tests.**
  AOSP publishes only raw milliseconds (`CpuInfoReader.java:859-880`) and never a busy percentage, so any derived percentage needs a unit test against a hand-summed row.

---

