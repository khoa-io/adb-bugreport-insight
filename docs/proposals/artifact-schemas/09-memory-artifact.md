# `memory.json` — and whether the collector perturbed the reading

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Requirement: `REQ-INSIGHT-04`.

Separates pressure, owners and memory categories, with timing evidence for possible collector effects.
The archive alone does not establish the cause of a swap delta.

### 4.6 `memory.json`

#### Purpose

`memory.json` separates **(1)** pressure/reactions (kills, OOM) at capture, **(2)** memory owners and **(3)** memory categories.
It also records collection timing beside the observed swap growth.
Temporal overlap with collection does not prove that the collector caused that growth.

#### Top-level keys

The fourteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | Per-channel presence/absence with reason. |
| `collection` | object | yes | The self-perturbation ledger, per [§2.1.5](01-envelope.md#215-collection). |
| `system` | object | yes | Global pressure verdict. **Only this object may carry a `status` or `free_ratio`.** |
| `swap` | object | yes | Swap and ZRAM, snapshot-indexed. |
| `processes` | array of object | yes | Per-process memory — the **cause**. |
| `categories` | array of object | yes | PSS by category — the **nature**. |
| `kernel_memory` | object | yes | Slab, pagetables, stacks, CMA. |
| `procstats` | array of object | yes | **One entry per sub-section, each with its own window.** Never merged. |
| `pressure_indicators` | object | yes | PSI plus `/proc/vmstat` OOM counters. |
| `kills` | array of object | yes | lmkd kills. **Empty is a finding, not an absence.** |
| `kill_evidence` | object | yes | Structured proof of the *absence* of kills and OOM. |
| `io` | object | yes | IOTOP, storaged, zram block stats. |

**Why separate objects.**
`system`, `processes[]`, and `categories[]` contain memory measurements; `kills[]` comes from separate lmkd/event-log evidence.
Even within one meminfo call, per-process reads happen sequentially on a moving system.
A single printed uptime does not make all rows atomic or mutually simultaneous.
Preserve each source and its timing.

#### The self-perturbation design

**The problem.**
The two page-scanning sections — an all-process PSS dump and the `bugreport_procdump` walk — both traverse every process's page tables.
`swap_used` growth of ~200 MiB alone cannot establish device thrashing.

There are **four** swap readings in the reference archive, not two, and the growth tracks the dump's own page-scanning work.
Page-table scans overlap collection and the collector's own processes appear in its PSS table.
These observations support a possible collector effect, not a causal verdict.

Three design rules follow:

1. **Never diff snapshots implicitly.**
   Swap lives in `collection.swap_timeline[]`, an explicit ordered list, and every delta is an *edge* between two identified nodes.
   There is **no** `swap.used_kib` scalar a consumer can compare against another artifact's scalar.
2. **Make the collector's own cost a measured quantity.**
   `collection.page_scanning_sections[]` is built from the section footers dumpstate prints unconditionally.
   Membership uses **printed per-PID rows with a swap column**, not unavailable AOSP invocation-site knowledge.
   One section's own footer is *inside* its parent's span and is **not** added again.
3. **A consumer that only reads `system` cannot be misled.**
   `system.provenance.collector_perturbed` is `null` when the only evidence is overlap with such a scan; an overlap is not proof of an effect.
   The same shared pair expresses "clean" in `cpu.json`.

The classification rule is in [§2.1.5](01-envelope.md#215-collection).
Measured growth spans pre/post-scan readings with zero kills and `oom_kill 0`; expose these facts before interpretation.
**Collector causation is not a field**: no per-process page-fault accounting establishes it.

#### Field tables

Condensed.
Grouped rows below define the fields; [source provenance](13-source-provenance.md#domain-evidence-sources) identifies their producer families.

##### `collection.swap_timeline[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `proc_meminfo` / `top` / `bugreport_procdump` / `dumpsys_meminfo`. | — |
| `line_start` | integer | yes | Archive locator, **and the ordering key** per [§2.1.5](01-envelope.md#215-collection). **There is deliberately no statement-order field**: dumpstate runs sections on a thread pool, so output append order can differ from execution and completion order (`DumpPool.cpp:37-48`). | measured |
| `swap_total_kib` | integer | yes | `SwapTotal`. | `android_os_Debug.cpp:391` |
| `swap_used_kib` | integer \| null | yes | `SwapTotal − SwapFree`; `null` if either absent. **Carries `swap_used_kib_derived`.** | `android_os_Debug.cpp:392` |
| `zram_physical_kib` | integer \| null | yes | `mem_used_total` summed over zram devices. | `sysmeminfo.cpp:178-206`; `AMS:12812-12819`; `smapinfo.cpp:289-291` |
| `section_duration_ms` | integer \| null | yes | The collector's own duration for this reading. | `smapinfo.cpp:1201-1203`; `dumpsys.cpp:563-567` |
| `provenance` | object | yes | | — |

##### `system`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `total_ram_kib` | integer \| null | yes | `Total RAM: 3,047,508K`. | `AMS:12699` → `MEMINFO_TOTAL` `android_os_Debug.cpp:383` |
| `memory_level` | string \| null | yes | `(status normal)` — AMS' own verdict. **Only this object may carry a status.** | `AMS:12700-12701` |
| `free_ram_kib` / `free_ram_components_kib` | integer / object | yes | `{cached_pss, cached_kernel, free}`. | `AMS:12702-12709` |
| `free_ratio` | number \| null | yes | `free_ram_kib / total_ram_kib`, 4 dp. **Derived.** | derived |
| `used_ram_kib` / `used_pss_kib` / `kernel_used_kib` | integer \| null | yes | | `AMS:12803-12804` |
| `total_pss_kib` | integer \| null | yes | **Derived** = `used_pss_kib + cached_pss` = 2,007,345. **The literal string `2,007,345` does not appear in the archive**, so this field **must** carry `total_pss_kib_derivation`. | `AMS:12803` + `AMS:12703` |
| `lost_ram_kib` | integer \| null | yes | The accounting residual. | `AMS:12796-12798` |
| `tuning` | object \| null | yes | `{memory_class_mib, large_memory_class_mib, oom_adj_score_kib, restore_limit_kib, low_ram, high_end_gfx}`. | `AMS:12839-12850` |
| `gpu_kib`, `dma_buf` | integer / object | yes | | `AMS:12771-12791`, `AMS:12715-12733` |
| `provenance` | object | yes | | — |

##### `swap`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `swap_total_kib` | integer \| null | yes | | `AMS:12818-12819` |
| `swap_used_kib` | integer \| null | yes | | `AMS:12814-12816` |
| `swap_used_pct` | number \| null | yes | **Derived** = `100 × swap_used_kib / swap_total_kib`; null for an absent/zero denominator. | derived |
| `zram_physical_kib` | integer \| null | yes | | `AMS:12812-12814` → `MEMINFO_ZRAM_TOTAL` `android_os_Debug.cpp:393` |
| `zram_physical_pct_of_ram` | number \| null | yes | **Derived.** | derived |
| `compression_ratio` | number \| null | yes | **Derived** = `swap_used_kib / zram_physical_kib`. Carries `compression_ratio_derived: true`. | derived |
| `zram_block_io` | object \| null | yes | Raw `/sys/block/zram0/stat` counters, **cumulative since boot**. | `dumpstate.cpp:1688,1230-1244` |
| `provenance` | object | yes | | — |

**There is deliberately no `zram_orig_size_kib` and no `zram_compressed_size_kib`.**
`SysMemInfo::MemZramDevice` deliberately skips `mm_stat` fields 1 and 2 and takes field 3, `mem_used_total` (`sysmeminfo.cpp:178-206`).
The inspected standard meminfo/procrank output does not expose `orig_data_size` or `compr_data_size`.
This is not a claim about every OEM section or optional raw sysfs dump.

`PROCRANK`'s per-process `ZSwap` is likewise **derived, not measured**: it is a device-wide compression ratio apportioned per process (`zram_compression_ratio = mem_zram_kb / (mem_swap_kb − mem_swap_free_kb)`, `smapinfo.cpp:351-353`; `zswap_ = proportional_swap_ * zram_compression_ratio`, `processrecord.cpp:114`).
Verified on the archive: `1175 … 4K 4K 4K 1K com.android.car.settings` gives `4 K × (24,312 / 57,088) = 1.70 K`, printed as `1K`.
**`ZSwap` carries no per-process truth and every consumer must be told so** via `zswap_estimated: true`.

##### `processes[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer | yes | | `AMS:12661-12665` |
| `process_name` | string | yes | As printed, un-normalised. | `AMS:12663` |
| `user_id` | integer \| null | yes | Only printed when the entry is uid-cached. | `AMS:11905-11912` |
| `pss_total_kib` | integer | yes | | `ss[INDEX_TOTAL_PSS]` row, `AMS:11843-11872` |
| `pss_private_dirty_kib` / `pss_shared_clean_kib` / `rss_total_kib` | integer \| null | yes | | `AMS:11851`, `:12362-12370`; `smapinfo.cpp:247-249` |
| `swap_pss_kib` / `swap_uss_kib` | integer \| null | yes | `(1,128K in swap)` / `PSwap`, `USwap`. | `AMS:11909-11914`; `smapinfo.cpp:250,251` |
| `zswap_kib` | integer \| null | yes | **Always `zswap_estimated: true`.** | `smapinfo.cpp:252-254`; `processrecord.cpp:114` |
| `oom_adj` | integer \| null | yes | | `smapinfo.cpp:238-239`; `AMS:12668-12671` |
| `heap_size_kib` / `heap_alloc_kib` / `heap_free_kib` | integer \| null | yes | | `ActivityThread.dumpMemInfoTable` via `AMS:12366` |
| `source_channel` | enum | yes | `dumpsys_meminfo` / `bugreport_procdump` / `procrank`. **A process may appear in both** — the two dumps are at different instants. | — |
| `provenance` | object | yes | | — |

##### `categories[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `category` | string | yes | `Native`, `.apk mmap`, `Dalvik`, `Dalvik Other`, `Unknown`, … | `AMS:12579-12640` |
| `pss_total_kib` | integer | yes | | `AMS:12674-12679` |
| `swap_pss_kib` | integer \| null | yes | | `AMS:11909-11914` |
| `depth` / `parent_category` | integer / string \| null | yes | Nesting level in the printed tree. | printed indentation |
| `provenance` | object | yes | | — |

##### `procstats[]` — never merged

Three sub-sections, three different datasets, three different windows.
The shape that matters is not the numbers but the mismatch between them:

| Sub-section header | What its start time actually is | `Aggregated over` |
| --- | --- | --- |
| `CURRENT STATS:` | inside the capture window | 1 |
| `AGGREGATED OVER LAST 24 HOURS:` | **months before this boot** | N |
| `AGGREGATED OVER LAST 3 HOURS:` | **the same value as the 24-hour block** | N |

> **The window named in the header is a label, not a fact.**
> In the reference archive the 3-hour and 24-hour blocks resolve to an identical start time that predates the capture by months.
> Stale aggregate averages cannot describe the current device.
> `capture_kind` here is **`persisted_aggregate`**, and that is the value that catches it.

The header block is `ProcessStats.java:1597-1633`: `Start time:` (`:1598-1600`), `Total uptime:` (`:1601-1605`), `Total elapsed time:` (`:1606-1609`), the completeness flags (`(shutdown)`/`(sysprops)`/`(complete)`/`(partial)`, `:1610-1628`), and `Aggregated over: N` (`:1632-1633`).

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `sub_section` | enum | yes | `CURRENT` / `AGGREGATED_24H` / `AGGREGATED_3H`. | `ProcessStatsService.java:1359/1374/1378` |
| `scope_label_verbatim` | string | yes | Kept **so a consumer can see the label is not a fact**. | same |
| `period_start_clock` | string | yes | | `ProcessStats.java:1598-1600` |
| `period_total_uptime` / `period_total_elapsed` | string \| null | yes | | `ProcessStats.java:1601-1609` |
| `period_complete` / `period_flags` | boolean / array | yes | The `(partial)` suffix. | `ProcessStats.java:1610-1628` |
| `aggregated_over_n` | integer \| null | yes | | `ProcessStats.java:1632-1633` |
| `backing_state_file` | string \| null | yes | The `mFile=` path the aggregate block was read from, verbatim. It carries a timestamp in its name, which is a **filename convention, not a measured clock**. | `ProcessStatsService.java:1362` |
| `sys_mem_usage[]` | array | yes | `{label, sample_count, cached_*, free_*, zram_*, kernel_*, native_*}` min/avg/max. | `SysMemUsageTable.java:142-171` |
| `checkin_period` | object \| null | yes | `{start_clock, start_realtime_ms, end_realtime_ms, complete, window_ms}` parsed from the `period,…` line. | `ProcessStats.java:2149-2166` |
| `provenance` | object | yes | `capture_kind: persisted_aggregate`, `freshness: stale` for both `AGGREGATED` entries. | — |

The checkin twin carries the window in machine-readable form:

```text
period,<YYYY-MM-DD-HH-mm-ss>,<realtime_ms>,<realtime_ms>,partial
```

⇒ start clock, `mTimePeriodStartRealtime` ms, `elapsedRealtime` ms, **partial** (the shutdown / sysprops / complete flags all clear), and a derived window length.

##### `pressure_indicators`, `kills[]`, `kill_evidence`, `io`, `kernel_memory`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `pressure_indicators` | `psi_memory` / `psi_io` | object \| null | yes | `{some:{avg10,avg60,avg300,total_us}, full:{…}}`. `avg*` are exponentially weighted moving averages; `total` is µs. | `dumpstate.cpp:1979-1981`; `system/memory/lmkd/libpsi/psi.cpp` |
| | `vmstat_oom_kill` | integer \| null | yes | `oom_kill 0`. | `dumpstate.cpp:1698` |
| | `vmstat_pswpin` / `vmstat_pswpout` | integer \| null | yes | Cumulative since boot. | `dumpstate.cpp:1698` |
| `kills[]` | `timestamp`, `process_name`, `pid`, `uid`, `oom_score_adj`, `rss_kib`, `swap_kib`, `reason_code`, `reason_text` | — | yes | **Empty is a valid, meaningful result.** `reason_text` comes from lmkd's own `ALOGI`, not from the event log. | `lmkd.cpp:2481-2488`; `statslog.h:57-70` |
| | `killinfo_event` | object \| null | yes | The 41-field `killinfo` payload when the events buffer retains it. | `lmkd.cpp:2136-2180`; `event.logtags:39` |
| `kill_evidence` | `lmkd_alive` / `lmkd_using_psi` | boolean \| null | yes | | `lmkd.cpp:17`, `:3597` |
| | `vmstat_oom_kill` / `am_kill_count` | integer | yes | Both `0`. | `dumpstate.cpp:1698`; `EventLogTags.logtags:34` |
| | `am_low_memory_count` / `am_low_memory_value` / `am_low_memory_meaning` | integer / string | yes | `1` — and this is **not** memory exhaustion. `am_low_memory` fires when `!haveBackgroundProcessLOSP()`, i.e. when the *lowest* oom_adj band was already emptied. A `1` with `status normal` is a lowest-priority-caches-exhausted signal, not a kill trigger. The meaning is carried as mandatory prose so the interpretation is visible rather than implicit. | `EventLogTags.logtags:31`; writer `AppProfiler.java:1607-1624` |
| | `am_meminfo_count` / `am_pss_count` | integer | yes | Both measured as `0` in this fixture; not a platform invariant. | `EventLogTags.logtags:67,69`; `am_meminfo` writer `AMS:12691` |
| `io` | `iotop` | object \| null | yes | `{window_ms, row_granularity, sort_column, max_io_delay_pct, max_swap_delay_pct, max_sched_delay_pct, max_mem_delay_pct, resolution_floor_note}`. | `iotop.cpp:36-39,234,247-265` |
| | `storaged` | object \| null | yes | **Cumulative since process start — no window.** | `dumpstate.cpp:1801` |
| | `zram_block_io` | object \| null | yes | Same counters as `swap.zram_block_io`. | `dumpstate.cpp:1688` |
| `kernel_memory` | `slab` / `pagetables` / `stacks` / `cma` | object \| null | yes | From `ZONEINFO` / `BUDDYINFO` / `SLAB INFO`. | `dumpstate.cpp:1700-1704` |

#### Zero memory-event counts are bounded observations

The fixture retains no `am_meminfo` or `am_pss` entries in the searched event-log region.
That does not prove these tags could not have fired before collection.
`AppProfiler.java:668,808` writes `am_meminfo` during background profiling; `:917,983` writes `am_pss`.
Meminfo dumps also write `am_meminfo` (`ActivityManagerService.java:12691,13295`).
Ring-buffer retention, sampling and collection order all limit interpretation.
Publish the count and searched window; do not add an `am_meminfo_absence_reason` asserting a cause that was not observed.

#### Provenance and freshness

Three distinct clock families, each with its own `capture_kind`:

| Source | `capture_kind` | `captured_at_time_base` | Freshness |
| --- | --- | --- | --- |
| `dumpsys meminfo` summary block | `point_in_time_snapshot` | `dumpsys_section_snapshot` (from the section footer's `ending at:` stamp) | `in_window` |
| `PROCRANK` rows and footer | `point_in_time_snapshot` | `undetermined` | `undated` — raw smaps print no clock |
| `MEMORY INFO` (`/proc/meminfo`) | `point_in_time_snapshot` | `undetermined` | `undated` — **deliberately unstamped**; dumpstate suppresses the mtime for `/proc`, `/sys`, and `/d` paths |
| `dumpsys procstats` `CURRENT` | `background_sampler` | `dumpsys_section_snapshot` | `in_window` |
| `dumpsys procstats` `AGGREGATED_*` | **`persisted_aggregate`** | `dumpsys_section_snapshot` | **`stale`** — months |
| `/proc/vmstat` counters | `command_output` | `undetermined` | `undated` — cumulative since boot, but no clock printed |
| IOTOP | `command_output` | `undetermined` | `undated` |
| lmkd kill log line | `live_log` | `logcat_line` | `in_window` |

**The `undated` rows are the honest ones.**
A genuine instantaneous reading with **no printed clock at all** is not `stale`; it is unlabelled.
Neither `stale` nor `now` can replace `undated`: unknown clocks prove neither historical nor current state.
Use `line_start` for output order only; dumpstate appends awaited futures independently of completion order.
Any field claiming to recover statement order is unproducible from any archive.

**IOTOP's `0.00%` columns are a resolution floor, not a zero.**
`TimeToTgidPercent` (`iotop.cpp:36-39`) computes a percentage per thread over the sample window and clamps at the top of the range, printed with two decimal places (`:255-265`).
A `0.00%` therefore means *below the print resolution*, **not** "no I/O".
The same rows carry non-zero throughput:

```text
   <n> <thread_name>       <read>   <write>   <read>   <write>  <n>  0.00%  0.00% <sched_delay>  0.00% <sched_delay>
  <n> Blocking Thread      0   2308   2308      0     99  0.00%  0.00% 24.65%  0.00% 24.65%
    <n> <kernel_thread>         0      0      0      0      0  0.00%  0.00%  0.72%  0.00%  0.72%
```

The non-zero figure on `Blocking Thread` is **run-queue delay, not I/O** — column 3 of the `delayed on` group is `sched` (`iotop.cpp:234`).
This section answers "was anything *waiting*".
Its own duration footer exceeds the nominal sampling window; the excess is process setup and the sampling loop's first iteration (`iotop.cpp:226-273`).

