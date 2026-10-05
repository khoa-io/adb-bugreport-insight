# `memory.json` — and whether the collector perturbed the reading

> **Draft.** See [`index.md`](./index.md). Requirement: `DRAFT-REQ-ART-07`.

Original §4.6 of `artifact-schemas.md`, up to the worked example. Answers three separable questions
— was the device under pressure, who held the memory, and what kind it was — and carries an explicit
verdict on whether the collection itself perturbed the measurement. In the reference archive it did.#

### 4.6 `memory.json`

#### Purpose

`memory.json` answers three separable questions and refuses to let one answer stand in for
another: **(1)** was the device under memory pressure at capture, and was it *reacting* (kills,
OOM); **(2)** who was holding the memory; **(3)** what *kind* of memory it was. It also carries
an explicit verdict on whether the collection itself perturbed the measurement — because in
the reference archive it did, by 207 MiB.

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | Per-channel presence/absence with reason. |
| `collection` | object | yes | The self-perturbation ledger, per [§2.1.5](./conventions-envelope.md#215-collection). |
| `system` | object | yes | Global pressure verdict. **Only this object may carry a `status` or `free_pct`.** |
| `swap` | object | yes | Swap and ZRAM, snapshot-indexed. |
| `processes` | array of object | yes | Per-process memory — the **cause**. |
| `categories` | array of object | yes | PSS by category — the **nature**. |
| `kernel_memory` | object | yes | Slab, pagetables, stacks, CMA. |
| `procstats` | array of object | yes | **One entry per sub-section, each with its own window.** Never merged. |
| `pressure_indicators` | object | yes | PSI plus `/proc/vmstat` OOM counters. |
| `kills` | array of object | yes | lmkd kills. **Empty is a finding, not an absence.** |
| `kill_evidence` | object | yes | Structured proof of the *absence* of kills and OOM. |
| `io` | object | yes | IOTOP, storaged, zram block stats. |

**Why four arrays rather than one.** `system`, `processes[]`, `categories[]`, and `kills[]` are
emitted by *one* `dumpsys meminfo` call at *one* instant (the block prints a single uptime
figure for the whole dump), so they are mutually consistent and belong in one artifact — but
they answer different questions and aggregate over different keys. A flat per-process list
would force a choice of granularity and would drop either the verdict or the category split. A
single global object would drop attribution entirely, and a process implicated by a crash loop
elsewhere in the output set is only findable from `processes[]`.

#### The self-perturbation design

**The problem.** The two page-scanning sections — an all-process PSS dump and the
`bugreport_procdump` walk — both traverse every process's page tables. A naive schema that
diffs `swap_used` across the archive reports "swap grew ~200 MiB, the device is thrashing" — a
pure artifact of the collector.

There are **four** swap readings in the reference archive, not two, and the growth tracks the
dump's own page-scanning work. The two steps that scan every page table sit exactly where the
number moves, and the collector's own processes appear in its own PSS table.

Three design rules follow:

1. **Never diff snapshots implicitly.** Swap lives in `collection.swap_timeline[]`, an explicit
   ordered list, and every delta is an *edge* between two identified nodes. There is **no**
   `swap.used_kb` scalar a consumer can compare against another artifact's scalar.
2. **Make the collector's own cost a measured quantity.** `collection.page_scanning_sections[]`
   is built from the section footers dumpstate prints unconditionally. Membership is decided by
   **what the section prints** — a per-PID row set carrying a swap column — never by AOSP
   invocation-site knowledge, so the same rule works on a build whose source you cannot read.
   One section's own footer is *inside* its parent's span and is **not** added again.
3. **A consumer that only reads `system` cannot be misled.** `system.provenance.collector_perturbed`
   is `true` for any reading taken inside such a scan. The same pair, in the same place,
   expresses "clean" in `cpu.json` — which is why it is in the shared convention rather than in
   one artifact.

The classification rule is in [§2.1.5](./conventions-envelope.md#215-collection). Applied here: the entire measured
growth falls between the last pre-scan reading and the post-scan reading, and over that
interval there are zero kills and `oom_kill 0` — the shape a reader must be able to see before
concluding anything. **Whether that overlap caused the growth is not a field**, because the
archive contains no per-process page-fault accounting that would settle it.

#### Field tables

Condensed. The owning file's exhaustive row-level table is preserved at
`tmp/aosp-team/03-memory-and-cpu.md` §4.4 for the detail that did not survive merging.

##### `collection.swap_timeline[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `proc_meminfo` / `top` / `bugreport_procdump` / `dumpsys_meminfo`. | — |
| `line_start` | integer | yes | Archive locator, **and the ordering key** per [§2.1.5](./conventions-envelope.md#215-collection). **There is deliberately no statement-order field**: dumpstate runs sections on a thread pool, so archive order is *completion* order and no archive recovers the intended order. | measured |
| `swap_total_kb` | integer | yes | `SwapTotal`. | `android_os_Debug.cpp:391` |
| `swap_used_kb` | integer \| null | yes | `SwapTotal − SwapFree`; `null` if either absent. **Carries `swap_used_kb_derived`.** | `android_os_Debug.cpp:392` |
| `zram_physical_kb` | integer \| null | yes | `mem_used_total` summed over zram devices. | `sysmeminfo.cpp:178-206`; `AMS:12812-12819`; `smapinfo.cpp:289-291` |
| `section_duration_ms` | integer \| null | yes | The collector's own duration for this reading. | `smapinfo.cpp:1201-1203`; `dumpsys.cpp:563-567` |
| `provenance` | object | yes | | — |

##### `system`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `total_ram_kb` | integer \| null | yes | `Total RAM: 3,047,508K`. | `AMS:12699` → `MEMINFO_TOTAL` `android_os_Debug.cpp:383` |
| `memory_level` | string \| null | yes | `(status normal)` — AMS' own verdict. **Only this object may carry a status.** | `AMS:12700-12701` |
| `free_ram_kb` / `free_ram_components_kb` | integer / object | yes | `{cached_pss, cached_kernel, free}`. | `AMS:12702-12709` |
| `free_pct` | number \| null | yes | `free_ram_kb / total_ram_kb`, 4 dp. **Derived.** | derived |
| `used_ram_kb` / `used_pss_kb` / `kernel_used_kb` | integer \| null | yes | | `AMS:12803-12804` |
| `total_pss_kb` | integer \| null | yes | **Derived** = `used_pss_kb + cached_pss` = 2,007,345. **The literal string `2,007,345` does not appear in the archive**, so this field **must** carry `total_pss_kb_derivation`. | `AMS:12803` + `AMS:12703` |
| `lost_ram_kb` | integer \| null | yes | The accounting residual. | `AMS:12796-12798` |
| `tuning` | object \| null | yes | `{memory_class_mb, large_memory_class_mb, oom_adj_score_kb, restore_limit_kb, low_ram, high_end_gfx}`. | `AMS:12839-12850` |
| `gpu_kb`, `dma_buf` | integer / object | yes | | `AMS:12771-12791`, `AMS:12715-12733` |
| `provenance` | object | yes | | — |

##### `swap`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `swap_total_kb` | integer \| null | yes | | `AMS:12818-12819` |
| `swap_used_kb` | integer \| null | yes | | `AMS:12814-12816` |
| `swap_used_pct` | number \| null | yes | **Derived.** | derived |
| `zram_physical_kb` | integer \| null | yes | | `AMS:12812-12814` → `MEMINFO_ZRAM_TOTAL` `android_os_Debug.cpp:393` |
| `zram_physical_pct_of_ram` | number \| null | yes | **Derived.** | derived |
| `compression_ratio` | number \| null | yes | **Derived** = `swap_used_kb / zram_physical_kb`. Carries `compression_ratio_derived: true`. | derived |
| `zram_block_io` | object \| null | yes | Raw `/sys/block/zram0/stat` counters, **cumulative since boot**. | `dumpstate.cpp:1688,1230-1244` |
| `provenance` | object | yes | | — |

**There is deliberately no `zram_orig_size_kb` and no `zram_compressed_size_kb`.**
`SysMemInfo::MemZramDevice` deliberately skips `mm_stat` fields 1 and 2 and takes field 3,
`mem_used_total` (`sysmeminfo.cpp:178-206`). No bugreport output exposes `orig_data_size` or
`compr_data_size` anywhere, so a field so named would be unpopulatable.

`PROCRANK`'s per-process `ZSwap` is likewise **derived, not measured**: it is a
device-wide compression ratio apportioned per process
(`zram_compression_ratio = mem_zram_kb / (mem_swap_kb − mem_swap_free_kb)`,
`smapinfo.cpp:351-353`; `zswap_ = proportional_swap_ * zram_compression_ratio`,
`processrecord.cpp:114`). Verified on the archive: `1175 … 4K 4K 4K 1K
com.android.car.settings` gives `4 K × (24,312 / 57,088) = 1.70 K`, printed as `1K`. **`ZSwap`
carries no per-process truth and every consumer must be told so** via
`zswap_estimated: true`.

##### `processes[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer | yes | | `AMS:12661-12665` |
| `process_name` | string | yes | As printed, un-normalised. | `AMS:12663` |
| `user_id` | integer \| null | yes | Only printed when the entry is uid-cached. | `AMS:11905-11912` |
| `pss_total_kb` | integer | yes | | `ss[INDEX_TOTAL_PSS]` row, `AMS:11843-11872` |
| `pss_private_dirty_kb` / `pss_shared_clean_kb` / `rss_total_kb` | integer \| null | yes | | `AMS:11851`, `:12362-12370`; `smapinfo.cpp:247-249` |
| `swap_pss_kb` / `swap_uss_kb` | integer \| null | yes | `(1,128K in swap)` / `PSwap`, `USwap`. | `AMS:11909-11914`; `smapinfo.cpp:250,251` |
| `zswap_kb` | integer \| null | yes | **Always `zswap_estimated: true`.** | `smapinfo.cpp:252-254`; `processrecord.cpp:114` |
| `oom_adj` | integer \| null | yes | | `smapinfo.cpp:238-239`; `AMS:12668-12671` |
| `heap_size_kb` / `heap_alloc_kb` / `heap_free_kb` | integer \| null | yes | | `ActivityThread.dumpMemInfoTable` via `AMS:12366` |
| `source_channel` | enum | yes | `dumpsys_meminfo` / `bugreport_procdump`. **A process may appear in both** — the two dumps are at different instants. | — |
| `provenance` | object | yes | | — |

##### `categories[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `category` | string | yes | `Native`, `.apk mmap`, `Dalvik`, `Dalvik Other`, `Unknown`, … | `AMS:12579-12640` |
| `pss_total_kb` | integer | yes | | `AMS:12674-12679` |
| `swap_pss_kb` | integer \| null | yes | | `AMS:11909-11914` |
| `depth` / `parent_category` | integer / string \| null | yes | Nesting level in the printed tree. | printed indentation |
| `provenance` | object | yes | | — |

##### `procstats[]` — never merged

Three sub-sections, three different datasets, three different windows. The shape that matters
is not the numbers but the mismatch between them:

| Sub-section header | What its start time actually is | `Aggregated over` |
| --- | --- | --- |
| `CURRENT STATS:` | inside the capture window | 1 |
| `AGGREGATED OVER LAST 24 HOURS:` | **months before this boot** | N |
| `AGGREGATED OVER LAST 3 HOURS:` | **the same value as the 24-hour block** | N |

> **The window named in the header is a label, not a fact.** In the reference archive the
> 3-hour and 24-hour blocks resolve to an identical start time that predates the capture by
> months. A schema that reads an average out of the aggregate blocks and reports it as a device
> property is wrong by the staleness of that data. `capture_kind` here is
> **`persisted_aggregate`**, and that is the value that catches it.

The header block is `ProcessStats.java:1597-1633`: `Start time:` (`:1598-1600`), `Total uptime:`
(`:1601-1605`), `Total elapsed time:` (`:1606-1609`), the completeness flags
(`(shutdown)`/`(sysprops)`/`(complete)`/`(partial)`, `:1610-1628`), and `Aggregated over: N`
(`:1632-1633`).

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

```
period,<YYYY-MM-DD-HH-mm-ss>,<realtime_ms>,<realtime_ms>,partial
```

⇒ start clock, `mTimePeriodStartRealtime` ms, `elapsedRealtime` ms, **partial** (the
shutdown / sysprops / complete flags all clear), and a derived window length.

##### `pressure_indicators`, `kills[]`, `kill_evidence`, `io`, `kernel_memory`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `pressure_indicators` | `psi_memory` / `psi_io` | object \| null | yes | `{some:{avg10,avg60,avg300,total_us}, full:{…}}`. `avg*` are exponentially weighted moving averages; `total` is µs. | `dumpstate.cpp:1979-1981`; `system/memory/lmkd/libpsi/psi.cpp` |
| | `vmstat_oom_kill` | integer \| null | yes | `oom_kill 0`. | `dumpstate.cpp:1698` |
| | `vmstat_pswpin` / `vmstat_pswpout` | integer \| null | yes | Cumulative since boot. | `dumpstate.cpp:1698` |
| `kills[]` | `timestamp`, `process_name`, `pid`, `uid`, `oom_score_adj`, `rss_kb`, `swap_kb`, `reason_code`, `reason_text` | — | yes | **Empty is a valid, meaningful result.** `reason_text` comes from lmkd's own `ALOGI`, not from the event log. | `lmkd.cpp:2481-2488`; `statslog.h:57-70` |
| | `killinfo_event` | object \| null | yes | The 41-field `killinfo` payload when the events buffer retains it. | `lmkd.cpp:2136-2180`; `event.logtags:39` |
| `kill_evidence` | `lmkd_alive` / `lmkd_using_psi` | boolean \| null | yes | | `lmkd.cpp:17`, `:3597` |
| | `vmstat_oom_kill` / `am_kill_count` | integer | yes | Both `0`. | `dumpstate.cpp:1698`; `EventLogTags.logtags:34` |
| | `am_low_memory_count` / `am_low_memory_value` / `am_low_memory_meaning` | integer / string | yes | `1` — and this is **not** memory exhaustion. `am_low_memory` fires when `!haveBackgroundProcessLOSP()`, i.e. when the *lowest* oom_adj band was already emptied. A `1` with `status normal` is a lowest-priority-caches-exhausted signal, not a kill trigger. The meaning is carried as mandatory prose so the interpretation is visible rather than implicit. | `EventLogTags.logtags:31`; writer `AppProfiler.java:1607-1624` |
| | `am_meminfo_count` / `am_pss_count` | integer | yes | Both `0` — **structurally**. | `EventLogTags.logtags:67,69`; `am_meminfo` writer `AMS:12691` |
| | `am_meminfo_absence_reason` | string | yes | **Required** when `am_meminfo_count == 0`. | — |
| `io` | `iotop` | object \| null | yes | `{window_ms, row_granularity, sort_column, max_io_delay_pct, max_swap_delay_pct, max_sched_delay_pct, max_mem_delay_pct, resolution_floor_note}`. | `iotop.cpp:36-39,234,247-265` |
| | `storaged` | object \| null | yes | **Cumulative since process start — no window.** | `dumpstate.cpp:1801` |
| | `zram_block_io` | object \| null | yes | Same counters as `swap.zram_block_io`. | `dumpstate.cpp:1688` |
| `kernel_memory` | `slab` / `pagetables` / `stacks` / `cma` | object \| null | yes | From `ZONEINFO` / `BUDDYINFO` / `SLAB INFO`. | `dumpstate.cpp:1700-1704` |

#### Why `am_meminfo` and `am_pss` are structurally zero

This matters because **"0 events" must be distinguishable from "channel broken"**. dumpstate
captures `EVENT LOG` early in the dump sequence, **before** the memory dumps that write these
tags. The `am_meminfo` tag is written *by the meminfo dump itself*, so in **every** bugreport
whose event log is captured before its memory sections — i.e. every bugreport 2.0 archive —
`am_meminfo` is structurally expected to be `0`. Measured alongside it, the event log does
carry other process-lifecycle tags at non-zero counts, which is what distinguishes "the log was
read" from "this one tag could not fire yet".

**A zero here is not evidence of a working memory-telemetry path**, and the schema says so via
`am_meminfo_absence_reason`.

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

**The `undated` rows are the honest ones.** A genuine instantaneous reading with **no printed
clock at all** is not `stale`; it is unlabelled. Collapsing `undated` into `stale` would make a
whole class of legitimate evidence look like historical residue and train consumers to ignore
the field. Collapsing it into `now` is the exact bug this convention exists to prevent. Where a
snapshot can only be ordered, the schema records `line_start` — and says nothing about the
collector's intended order, because dumpstate runs sections on a thread pool and archive order
is **completion** order. Any field claiming to recover statement order is unproducible from any
archive.

**IOTOP's `0.00%` columns are a resolution floor, not a zero.**
`TimeToTgidPercent` (`iotop.cpp:36-39`) computes a percentage per thread over the sample window
and clamps at the top of the range, printed with two decimal places (`:255-265`). A `0.00%`
therefore means *below the print resolution*, **not** "no I/O". The same rows carry non-zero
throughput:

```
   <n> <thread_name>       <read>   <write>   <read>   <write>  <n>  0.00%  0.00% <sched_delay>  0.00% <sched_delay>
  <n> Blocking Thread      0   2308   2308      0     99  0.00%  0.00% 24.65%  0.00% 24.65%
    <n> <kernel_thread>         0      0      0      0      0  0.00%  0.00%  0.72%  0.00%  0.72%
```

The non-zero figure on `Blocking Thread` is **run-queue delay, not I/O** — column 3 of the
`delayed on` group is `sched` (`iotop.cpp:234`). This section answers "was anything *waiting*".
Its own duration footer exceeds the nominal sampling window; the excess is process setup and
the sampling loop's first iteration (`iotop.cpp:226-273`).

