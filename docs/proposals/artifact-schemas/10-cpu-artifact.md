# `cpu.json` — utilisation and contention, never merged

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md).
> Requirement: `REQ-INSIGHT-04`.

Answers how much CPU was used, by whom, over what window, and separately whether anything was blocked.
Folding those into one "CPU problem" number is prohibited.

### 4.7 `cpu.json`

#### Purpose

`cpu.json` separates **CPU use, owners and window** from **blocking**; never combine distinct evidence into one "CPU problem" number.

#### Top-level keys

The fourteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | Per-channel presence/absence. |
| `collection` | object | yes | The shared ledger, **plus** `cpu_clean_channels` / `cpu_perturbed_channels` / `mechanism`. |
| `utilization` | array of object | yes | Global CPU ratios, **one entry per measurement, each with its own window.** |
| `per_cpu_core` | array of object | yes | Per-core cumulative milliseconds, including **steal**. |
| `processes` | array of object | yes | Per-process CPU over `utilization`'s window. |
| `contention` | object | yes | Lock/blocking evidence — **never merged into `utilization`.** |
| `load` | object | yes | Load averages **with uptime context**. |
| `stall_signals` | object | yes | PSI, IOTOP, process churn. |
| `graphics` | object \| null | yes | Per-process frame stats. |
| `power_history` | object \| null | yes | batterystats / powerstats with their own measured intervals; the fixture includes history predating this boot. |

**Four disjoint arrays separate utilization and contention.**
In this archive the channels describe CPU utilization and contention over different intervals.
High IRQ time and zero reported steal do not establish the source of interrupts or exclude host scheduling delays.
Keep the observations separate from a proposed diagnosis.

Further separations preserve **units**:

- `utilization` is a **windowed ratio** whose window is arbitrary and must travel with it.
- `per_cpu_core` is **cumulative milliseconds since boot**, read at one `read_at_uptime_ms`.
  Use only the block labelled `Latest cumulative CPU usage stats by CPU core`.
  The separate `Last read CPU infos` block contains `latestCpuUsageStats` deltas since the prior read (`CpuInfoReader.java:324-336,681-695`), with the first read falling back to cumulative.
  These blocks must not be parsed as interchangeable.
- `processes[]` shares `utilization`'s window exactly (one tracker delta), so it is attributable and gets a share field.
- `contention` is measured in **blocking events**, sampled at an unknown rate, over the **log window** — not the CPU window.

#### The `dumpsys cpuinfo` window — the finding that makes the CPU side provably clean

`AppProfiler.updateCpuStatsNow()` re-samples `/proc/stat` **only if ≥ 5 s have elapsed** since the last sample (`AppProfiler.java:142`, `MONITOR_CPU_MIN_TIME = 5 * 1000`; gate at `:2156-2160`), driven by a thread that has run since `system_server` started (`:2358`, `new ProcessCpuThread("CpuTracker")`, loop at `:2285-2313`).
`CpuBinder.dumpCritical` merely prints **the last two samples it already has** — it does not take a fresh one (`:2332-2335`).

The header carries the window explicitly (`ProcessCpuTracker.java:854-889`), in the form:

```text
CPU usage from <n>ms to <n>ms ago (<start> to <end>):
```

Printed numbers establish:

- **window length** = `end − start`
- **the window ended before the dump printed it** — the section's own duration footer gives the print instant, so the gap is computable rather than asserted
- **whether that gap reaches back to the start of the dump** — compare it against `capture.captured_at` in `system.json`

**The CPU numbers therefore cannot be inflated by the collection, and this is checkable rather than assumed.**
`collector_perturbed: false` is only published when the measured window closed *before* dumpstate began.
A window overlapping collection permits perturbation but does not prove it; use `null` when the cause cannot be established from the evidence.
`capture_kind: background_sampler` identifies pre-existing samples, not fresh dump-time measurements.

`top` (`dumpstate.cpp:1690`) is the opposite: it runs mid-dump, and its own `Swap:` line already shows the collector's effect against the immediately preceding `/proc/meminfo` read.

#### `steal` is structurally unavailable in the global ratio

`dumpsys cpuinfo`'s `TOTAL` line is `mRelUserTime + mRelSystemTime + mRelIoWaitTime + mRelIrqTime + mRelSoftIrqTime + mRelIdleTime` (`ProcessCpuTracker.java:890-892` for the denominator, `:914` for the row) — it **excludes steal**, because the aggregate `/proc/stat` read uses `SYSTEM_CPU_FORMAT` (`:126-134`) which stops at `softirq`.
Steal is also printed by toybox `top` as `host`; per-core cumulative values are available via the `cpu_monitor` binder service (`CpuInfoReader.java:69`, `:859-880`).
**Keep both channels**; `utilization[].pct_steal` for `dumpsys_cpuinfo` is explained `null`, never `0`.

#### Contention: three distinct channels, all real, none a count

A search for `Monitor contention with tid` returns **0**, while a case-insensitive search for `Long monitor contention` returns **33**.
**Different producers/thresholds** explain silent channels without implying failure.
All three are published; none is a count of stalls.

**ART Java monitor.**
Line shape (`art/runtime/monitor.cc:592-598` + `402-420`):

```text
Long monitor contention with owner <owner_name> (<owner_tid>) at <owner_method>(<file>:<line>) waiters=<N> in <waiter_method> for <PrettyDuration>
```

`PrettyDuration` (`art/libartbase/base/time_utils.cc:53-75`) chooses ns/µs/ms/**s** and prints at second granularity with fractions — hence `1.780s`, `181ms`, `12.3us`.

Threshold: `kLongWaitMs = 100 * kDebugThresholdFudgeFactor` (`monitor.cc:54`) and `kDebugThresholdFudgeFactor = kIsDebugBuild ? 10 : 1` (`monitor.cc:53`) — so the floor is **1000 ms in a debug ART library and 100 ms in a release ART library.**
`kIsDebugBuild` is selected by `NDEBUG` (`art/libartbase/base/globals.h:65-69`), not by `ro.build.type` or whether an application is debuggable.
Do not infer the loaded library variant from a `userdebug` fingerprint.
*Sub-threshold stalls produce no line; absent lines cannot establish health.*

Measured distribution of the long-contention lines (all wait strings): max **1.780 s**, min 105 ms; **6 events ≥ 1 s**, all inside two processes, all on the **same monitor**, all naming the same owner method and dex, with `waiters=` climbing `0,1,2,3` on consecutive lines a few milliseconds apart.
A monotonic waiter queue on one monitor is a `MessageQueue.next()` shape.
**17 distinct monitor events are tagged `system_server`, all below one second.**
The full monitor channel has 33 raw lines and 30 distinct events; zero applies only to the `system_server` ≥1-second subset.
The sampled channel below remains separate.

**`dvm_lock_sample` — the sampled channel.**
`Monitor::LogContentionEvent` is emitted only with probability proportional to wait_ms divided by the configured threshold, capped at 100% (`monitor.cc:529-536`):

```cpp
if (wait_ms >= lock_profiling_threshold_) { sample_percent = 100; }
else                                        { sample_percent = 100 * wait_ms / lock_profiling_threshold_; }
if (sample_percent != 0 && (static_cast<uint32_t>(rand() % 100) < sample_percent)) { … LogContentionEvent(…) }
```

Field order for this source baseline (`art/runtime/monitor_android.cc:35-108`); matching an OEM runtime remains a separate validation step:

```text
1 process_name  2 is_sensitive  3 thread_name  4 wait_ms
5 waiter_file   6 waiter_line   7 waiter_method
8 owner_file    9 owner_line    10 owner_method  11 sample_percent
```

A representative line, verbatim and complete:

```text
09-24 <HH:MM:SS.mmm>  <uid>   <pid>   <tid> I dvm_lock_sample: [<process>,<is_sensitive>,<thread>,<wait_ms>,<waiter_file>,<waiter_line>,<waiter_method>,<owner_file>,<owner_line>,<owner_method>,<sample_percent>]
```

⇒ `wait_ms`, `sample_percent`, and an `owner_file` of `-`, which is a "same file as the waiter, save log space" shorthand (`monitor_android.cc:88-93`).

> **The count is not a count.**
> A wait logged with a high `sample_percent` implies roughly `100 × lines / sample_percent` expected events at a fixed sampling percentage for waits of that length — but a wait well below the configured threshold logs only a small fraction of the time, so the *distribution* of unseen short waits is unbounded.
> **The schema records `sampled_event_count` and the observed `sample_percent` values, never a bare `contention_count`**, and `counting_caveat` is mandatory prose to that effect.

`lock_profiling_threshold_` comes from `Opt::LockProfThreshold` (`art/runtime/runtime.cc:1589`; `runtime_options.def:123`) and `dalvik.vm.lockprof.threshold` (`frameworks/base/core/jni/AndroidRuntime.cpp:963-965`), with another runtime flag able to disable profiling (`:758-764`).
A zero threshold disables `log_contention` (`monitor.cc:458`); build type alone does not establish that value.

**ART runtime mutex — the rate-limited channel.**
`Mutex::DumpStack` (`art/runtime/base/mutex.cc:567-571`) and `IsDumpFrequent()` (`:576-591`) impose a reporting limit.
Zero matched records establish only that none were retained in the searched region; they do not prove the runtime path was enabled or that no wait met a threshold.
Carry a bounded negative observation, with unknown enablement represented as `null`.

#### Field tables

Condensed.
Grouped rows below define the fields; [source provenance](13-source-provenance.md#domain-evidence-sources) identifies their producer families.

##### `utilization[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `dumpsys_cpuinfo` / `top` / `cpu_monitor`. | — |
| `line_start` | integer | yes | Archive locator. | measured |
| `window` | object | yes | `{start_utc, end_utc, duration_ms}`. **Mandatory** — a ratio without its window is meaningless. | `ProcessCpuTracker.java:860-872` |
| `window_source` | enum | yes | `section_body` (cpuinfo prints it) / `unknown` (top does not). | `ProcessCpuTracker.java:862-871` |
| `age_at_print_ms` | object \| null | yes | The `22612ms … 20840ms ago` pair. | `ProcessCpuTracker.java:861-865` |
| `total_pct` | number \| null | yes | | `:914` |
| `pct_user` / `pct_kernel` / `pct_iowait` / `pct_irq` / `pct_softirq` | number \| null | yes | | `:947-948`, `:949-950`, `:953-954`, `:958-959`, `:963-964` |
| `pct_nice` | number \| null | yes | **`null` as a separate bucket for `dumpsys_cpuinfo`** — nice is read and added into user (`ProcessCpuTracker.java:356`). | `:126-134` |
| `pct_steal` | number \| null | yes | **`null` for `dumpsys_cpuinfo`** — absent from its aggregate; toybox top calls this bucket `host`, and cpu_monitor also prints it. | `:126-134`; `CpuInfoReader.java:69` |
| `null_reasons` | object | yes | **Required** for every `null` percentage, naming the structural cause. | — |
| `online_cpus` | integer \| null | yes | Use independently observed online status; the top header alone does not supply it. | `CpuInfoReader` online status |
| `configured_cpus` | integer \| null | yes | Top header capacity divided by 100, from `sysconf(_SC_NPROCESSORS_CONF)`; not an online-CPU probe. | `external/toybox/toys/posix/ps.c:1645,1691` |
| `threads_total` / `running` / `sleeping` / `stopped` / `zombie` | integer \| null | yes | **`top` only.** `-H` selects thread rows; source is `external/toybox/toys/posix/ps.c` (`top_main`, `top_common`). | `top` |
| `provenance` | object | yes | | — |

##### `per_cpu_core[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `core_id` | integer | yes | 0..3. | `CpuInfoReader.java:63-70` |
| `is_online` | boolean \| null | yes | `isOnline = Yes`. | `CpuInfoReader` `CpuInfo` `toString` |
| `user_time_ms` / `nice_time_ms` / `system_time_ms` / `idle_time_ms` | integer \| null | yes | Cumulative since boot. | `CpuInfoReader.java:859-870` |
| `iowait_time_ms` / `irq_time_ms` / `softirq_time_ms` | integer \| null | yes | Cumulative since boot. | `:865-867` |
| `steal_time_ms` | integer \| null | yes | Steal time in the observed cpu_monitor cumulative block; optional proc/stat copies can also expose counters. | `:868` |
| `guest_time_ms` / `guest_nice_time_ms` | integer \| null | yes | Cumulative since boot. | `:869-870` |
| `busy_pct_since_boot` | number \| null | yes | **Derived from the cumulative block only** — `(user + nice + system + irq + softirq) / (user + nice + system + idle + iowait + irq + softirq + steal) × 100`. Guest/guest_nice are already included in user/nice; do not add them again. Carries `busy_pct_derivation`. AOSP publishes only raw milliseconds and never this percentage, so an implementation computing it needs a unit test against a hand-summed row. | derived |
| `read_at_uptime_ms` | integer \| null | yes | `mLastReadUptimeMillis = 11192`. | `CpuInfoReader.java:75` |
| `provenance` | object | yes | `capture_kind: command_output`, and **must not be read as a window**. | — |

##### `processes[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer \| null | yes | | `ProcessCpuTracker.java:899-908` |
| `label` | string | yes | | same |
| `pct_total` / `pct_user` / `pct_kernel` | number \| null | yes | | `:939-940`, `:947-948`, `:949-950` |
| `minor_faults` / `major_faults` | integer \| null | yes | | `PROCESS_STAT_MINOR_FAULTS` / `MAJOR_FAULTS` `:75-78` |
| `is_thread_row` | boolean | yes | `true` for the indented rows. | `:899-908` |
| `window_ref` | string | yes | Must point at a `utilization[].window`. | — |
| `provenance` | object | yes | | — |

##### `contention`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `art_java_monitor.event_count` | integer | yes | 33. | measured |
| `art_java_monitor.channel_present` | boolean | yes | Observed log lines establish this channel emitted records; absence of lines alone does not prove the gate was disabled. | `monitor.cc:458` |
| `art_java_monitor.threshold_ms` | integer \| null | yes | **100 or 1000 depending on the loaded ART library's compile-time debug flag.** Use `null` when that variant is not established; a userdebug fingerprint is insufficient. | `monitor.cc:53-54` |
| `art_java_monitor.wait_ms_max` / `wait_ms_min` | integer \| null | yes | 1780 / 105. | measured |
| `art_java_monitor.events_in_system_server` | integer | yes | **17 distinct retained events; none ≥1 s.** | measured |
| `art_java_monitor.events[]` | array | yes | `{timestamp, process, pid, tid, owner_name, owner_tid, owner_method, owner_file, owner_line, waiters, waiter_method, wait_ms, wait_verbatim}`. **`wait_verbatim` is kept so a consumer can see the unit ART chose.** `waiters` is a queue depth and the most diagnostic single field. | `monitor.cc:409-418,597`; `time_utils.cc:53-75` |
| `art_runtime_mutex` | object | yes | `{channel_present, event_count, format, short_format, zero_reason}`. `event_count` **0** in the searched region; enablement is unknown without separate evidence. | `mutex.cc:42,533,567-571,576-591` |
| `art_rate_limiting` | object | yes | `{java_monitor_is_sampled, java_monitor_sampling_rule, java_monitor_sample_pct_observed, java_monitor_threshold_source, runtime_mutex_is_rate_limited, runtime_mutex_rate_limit, counting_caveat}`. **`counting_caveat` is mandatory prose: no field in `contention` is a contention count.** | `monitor.cc:529-536`; `monitor_android.cc:105`; `runtime.cc:1589` |
| `dvm_lock_sample` | object | yes | `{sampled_event_count: 168, in_system_server: 149, system_server_ratio: 0.8869, field_order_source, example}`. Named `sampled_event_count`, **not** `event_count` — the name is the warning. | `monitor_android.cc:35-108` |
| `provenance` | object | yes | `capture_kind: live_log`. **The window is the log buffer, not the CPU window.** | — |

##### `load`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `sources[]` | array of object | yes | `{source: bugreport_header \| uptime_section \| dumpsys_cpuinfo, load_1m, load_5m, load_15m, uptime_ms_at_read, uptime_qualifier, line_start}`. **All three readouts are published, because on this archive they disagree.** | `ProcessCpuTracker.java:829-838`; `dumpstate.cpp:1687` |
| `load.uptime_ms_at_read` | integer \| null | yes | **Required whenever `load_1m` is present.** | header / `uptime` output |
| `online_cpus` | integer \| null | yes | Independently observed online count. Load divided by this count expresses task demand per CPU, including uninterruptible tasks; it is not utilization. | Independent online-status evidence |
| `provenance` | object | yes | Measurement source and time basis. | — |

Do not emit `load_15m_implies_uptime_ms` or `load_history_conflict`; load average cannot establish uptime.
Load average is an exponentially weighted count of runnable and uninterruptible tasks, not a duration.
Multiplying a load value by 15 and 1000 cannot recover uptime or establish cross-boot history.
See Linux's [load-average implementation](https://github.com/torvalds/linux/blob/v6.6/kernel/sched/loadavg.c). Publish measured load and independently observed uptime, with uncertainty.
A short boot means the averages reflect startup rather than a settled workload; it does not make a high 15-minute value mathematically impossible.
Normalizing load by CPUs gives demand per CPU, including uninterruptible tasks, rather than CPU utilization.

##### `stall_signals`, `graphics`, `power_history`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `stall_signals` | `psi_cpu` / `psi_io` | object \| null | yes | `{some:{avg10,avg60,avg300,total_us}, full:{…}}`. | `dumpstate.cpp:1979,1981` |
| | `iotop` | object \| null | yes | Same shape as `memory.json#/io/iotop`, **with** its own `provenance`. | `iotop.cpp:36-39,234,247-265` |
| | `proc_churn` | object | yes | `{am_proc_start_count, am_proc_died_count, am_proc_bound_count, am_uid_running_count, am_uid_active_count, window, rate_per_s, rate_valid, rate_suppressed_reason}`. **The window is the event-log window, not the CPU window.** | `EventLogTags.logtags:25,22`; `dumpstate.cpp:1028` |
| | `proc_churn.rate_valid` | boolean | yes | **Gates `rate_per_s`.** A per-boot churn rate computed over a window shorter than 120 s is dominated by the single burst of zygote/ART initialisation and means nothing. | derived |
| `graphics` | `per_process[]` | array \| null | yes | `{pid, process_name, stats_since_ns, total_frames, janky_frames, janky_pct, p50_ms, p90_ms, p95_ms, p99_ms}`. **gfxinfo windows are per-process and independent of `utilization[].window`.** | `AMS:1931`; `frameworks/base/libs/hwui/ProfileData.cpp:114` |
| | `window_note` | string | yes | | — |
| `power_history` | `batterystats_reset_time` / `batterystats_duration_s` / `powerstats_earliest_data_time` | string / integer | yes | | `dumpstate.cpp:2130,2134` |
| | `quarantine_reason` | string | yes | **These sections describe a *previous boot*.** They are stored here and **never** joined to `utilization` or to any uptime-derived field. `powerstats` reports `Earliest data time : Wed May 20 02:34:12 GMT+07:00 2026`; batterystats resets at `2026-05-19-00-16-18`. | derived |

#### The two aggregate CPU rows disagree — correctly

Two channels correctly report different aggregates:

| Source | Normalisation |
| --- | --- |
| `dumpsys cpuinfo` `TOTAL` row | divided by a time span in which the device may be partly idle |
| `top` aggregate row | one CPU is 100%; the header capacity is 100 × configured CPU count (`400%cpu` when configured count is 4) |

Both are correct over **different windows** (cpuinfo's is a short background sample taken before the dump; `top`'s is its own mid-dump sample) and against **different denominators**.
The reference top line reports `17% kernel` and `12% nice`; keep those CPU buckets separate.
**Never merge into one "CPU %"**; retain separate `utilization` entries/windows.

**On the high IRQ reading.**
Zero steal counters and `0%host` report no accounted steal in those channels/windows.
They do not prove the VM was never descheduled or identify the origin of interrupts.
Host interrupt injection and guest driver activity require additional evidence; neither is established by this archive alone.
See the kernel's [proc/stat counter definitions](https://docs.kernel.org/filesystems/proc.html#miscellaneous-kernel-statistics-in-proc-stat).
