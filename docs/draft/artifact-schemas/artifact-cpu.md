# `cpu.json` — utilisation and contention, never merged

> **Draft.** See [`index.md`](./index.md). Requirement: `DRAFT-REQ-ART-08`.

Original §4.7 of `artifact-schemas.md`, up to the worked example. Answers how much CPU was used, by
whom, over what window, and separately whether anything was blocked. Folding those into one "CPU
problem" number is prohibited.#

### 4.7 `cpu.json`

#### Purpose

`cpu.json` answers: **how much CPU was used, by whom, over what window** — and, separately,
**was anything blocked**, which is a different failure mode with different evidence and must
not be folded into a single "CPU problem" number.

#### Top-level keys

The thirteen envelope keys, plus:

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
| `power_history` | object \| null | yes | batterystats / powerstats — **quarantined, always stale.** |

**Four disjoint arrays, and the utilization/contention split is the substantive decision.** In
this archive the two point in opposite directions: a middling `TOTAL` with a dominant
`system_server` share looks like a load problem, while a high `irq` with `steal = 0` on every
core says the guest is being *interrupted*, not busy. A single score would say "CPU problem".
Two arrays say "compute-bound in `system_server`, host interrupt injection, and no lock
pathology worth reporting".

The other two separations are about **units**, not judgement:

- `utilization` is a **windowed ratio** whose window is arbitrary and must travel with it.
- `per_cpu_core` is **cumulative milliseconds since boot**, read at one `read_at_uptime_ms`.
  It is *not* a percentage and *not* the same window. Merging them produces a number that means
  nothing.
- `processes[]` shares `utilization`'s window exactly (one tracker delta), so it is
  attributable and gets a share field.
- `contention` is measured in **blocking events**, sampled at an unknown rate, over the **log
  window** — not the CPU window.

#### The `dumpsys cpuinfo` window — the finding that makes the CPU side provably clean

`AppProfiler.updateCpuStatsNow()` re-samples `/proc/stat` **only if ≥ 5 s have elapsed** since
the last sample (`AppProfiler.java:142`, `MONITOR_CPU_MIN_TIME = 5 * 1000`;
gate at `:2156-2160`), driven by a thread that has run since `system_server` started
(`:2358`, `new ProcessCpuThread("CpuTracker")`, loop at `:2285-2313`).
`CpuBinder.dumpCritical` merely prints **the last two samples it already has** — it does not
take a fresh one (`:2332-2335`).

The header carries the window explicitly (`ProcessCpuTracker.java:854-889`), in the form:

```
CPU usage from <n>ms to <n>ms ago (<start> to <end>):
```

From those printed numbers alone a tool can derive three facts, and all three are load-bearing:

- **window length** = `end − start`
- **the window ended before the dump printed it** — the section's own duration footer gives
  the print instant, so the gap is computable rather than asserted
- **whether that gap reaches back to the start of the dump** — compare it against
  `capture.captured_at` in `system.json`

**The CPU numbers therefore cannot be inflated by the collection, and this is checkable rather
than assumed.** `collector_perturbed: false` is only published when the measured window closed
*before* dumpstate began; otherwise the field is `true`. `capture_kind: background_sampler` is
what records why the gap exists — this channel prints pre-existing samples from a background
thread rather than taking a fresh one during the dump.

`top` (`dumpstate.cpp:1690`) is the opposite: it runs mid-dump, and its own `Swap:` line already
shows the collector's effect against the immediately preceding `/proc/meminfo` read.

#### `steal` is structurally unavailable in the global ratio

`dumpsys cpuinfo`'s `TOTAL` line is
`mRelUserTime + mRelSystemTime + mRelIoWaitTime + mRelIrqTime + mRelSoftIrqTime + mRelIdleTime`
(`ProcessCpuTracker.java:890-892` for the denominator, `:914` for the row) — it **excludes
steal**, because the aggregate `/proc/stat` read uses `SYSTEM_CPU_FORMAT` (`:126-134`) which
stops at `softirq`. Steal is available only per-CPU, via the `cpu_monitor` binder service
(`CpuInfoReader.java:69`, `:859-880`). **That asymmetry is why `cpu.json` needs both channels**,
and it is why `utilization[].pct_steal` is `null` for `dumpsys_cpuinfo` with a stated reason
rather than `0`.

#### Contention: three distinct channels, all real, none a count

A search for `Monitor contention with tid` returns **0**, while a case-insensitive search for
`Long monitor contention` returns **33**. These are **different producers with different
thresholds**, and a tool that conflates them reports a channel as broken when it is merely
silent. All three are published; none is a count of stalls.

**ART Java monitor.** Line shape (`art/runtime/monitor.cc:592-598` + `402-420`):

```
Long monitor contention with owner <owner_name> (<owner_tid>) at <owner_method>(<file>:<line>) waiters=<N> in <waiter_method> for <PrettyDuration>
```

`PrettyDuration` (`art/libartbase/base/time_utils.cc:53-75`) chooses ns/µs/ms/**s** and prints
at second granularity with fractions — hence `1.780s`, `181ms`, `12.3us`.

Threshold: `kLongWaitMs = 100 * kDebugThresholdFudgeFactor` (`monitor.cc:54`) and
`kDebugThresholdFudgeFactor = kIsDebugBuild ? 10 : 1` (`monitor.cc:53`) — so the floor is
**1000 ms on a debuggable build and 100 ms on a user build.** That is a real schema caveat, and
it is not knowable from the archive: *a stall well below the floor produces no line here, and
its absence is not evidence of health.*

Measured distribution of the long-contention lines (all wait strings): max **1.780 s**, min
105 ms; **6 events ≥ 1 s**, all inside two processes, all on the **same monitor**, all naming
the same owner method and dex, with `waiters=` climbing `0,1,2,3` on consecutive lines a few
milliseconds apart. A monotonic waiter queue on one monitor is a `MessageQueue.next()` shape.
**Zero of them are in `system_server`** — the `system_server` contention in this archive is the
*other* kind, the sampled channel below.

**`dvm_lock_sample` — the sampled channel.** `Monitor::LogContentionEvent` is emitted only with
probability equal to how far the wait exceeded the configured threshold (`monitor.cc:529-536`):

```cpp
if (wait_ms >= lock_profiling_threshold_) { sample_percent = 100; }
else                                        { sample_percent = 100 * wait_ms / lock_profiling_threshold_; }
if (sample_percent != 0 && (static_cast<uint32_t>(rand() % 100) < sample_percent)) { … LogContentionEvent(…) }
```

Field order (declared in the platform's event-log tag file, which **no bugreport contains** —
see the `evidence_gaps` entry for this channel):

```
1 process_name  2 is_sensitive  3 thread_name  4 wait_ms
5 waiter_file   6 waiter_line   7 waiter_method
8 owner_file    9 owner_line    10 owner_method  11 sample_percent
```

A representative line, verbatim and complete:

```
09-24 <HH:MM:SS.mmm>  <uid>   <pid>   <tid> I dvm_lock_sample: [<process>,<is_sensitive>,<thread>,<wait_ms>,<waiter_file>,<waiter_line>,<waiter_method>,<owner_file>,<owner_line>,<owner_method>,<sample_percent>]
```

⇒ `wait_ms`, `sample_percent`, and an `owner_file` of `-`, which is a "same file as the
waiter, save log space" shorthand (`monitor_android.cc:88-93`).

> **The count is not a count.** A wait logged with a high `sample_percent` implies roughly
> `lines / sample_percent` true events for waits of that length — but a *short* wait at the
> configured threshold logs only a small fraction of the time, so the *distribution* of unseen
> short waits is unbounded. **The schema records `sampled_event_count` and the observed
> `sample_percent` values, never a bare `contention_count`**, and `counting_caveat` is
> mandatory prose to that effect.

`lock_profiling_threshold_` comes from `Opt::LockProfThreshold`
(`art/runtime/runtime.cc:1589`; `runtime_options.def:123`) and is **zero on user builds** — in
which case `log_contention` is `false` and the whole branch is skipped (`monitor.cc:458`).

**ART runtime mutex — the rate-limited channel.** `Mutex::DumpStack`
(`art/runtime/base/mutex.cc:567-571`) is
enabled (`mutex.cc:533` called from `:481-499`) but produced **zero** events: no event satisfied
the 50 ms × `try_times` rate limit (`kIntervalMillis = 50`, `:42`; `IsDumpFrequent()` `:576-591`).
**Presence-with-zero is a result, not an absence**, and `zero_reason` states it.

#### Field tables

Condensed. The owning file's exhaustive row-level table is preserved at
`tmp/aosp-team/03-memory-and-cpu.md` §5.4.

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
| `pct_nice` | number \| null | yes | **`null` for `dumpsys_cpuinfo`** — not in `SYSTEM_CPU_FORMAT`. | `:126-134` |
| `pct_steal` | number \| null | yes | **`null` for `dumpsys_cpuinfo`** — structurally unavailable; only per-CPU via `cpu_monitor`. | `:126-134`; `CpuInfoReader.java:69` |
| `null_reasons` | object | yes | **Required** for every `null` percentage, naming the structural cause. | — |
| `online_cpus` | integer \| null | yes | The `400%` denominator. | `dumpstate.cpp:1690`; `CpuMonitorService.java:205` |
| `threads_total` / `running` / `sleeping` / `stopped` / `zombie` | integer \| null | yes | **`top` only — source not in this drop, semantics unverified.** | `top` |
| `provenance` | object | yes | | — |

##### `per_cpu_core[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `core_id` | integer | yes | 0..3. | `CpuInfoReader.java:63-70` |
| `is_online` | boolean \| null | yes | `isOnline = Yes`. | `CpuInfoReader` `CpuInfo` `toString` |
| `user_time_ms` / `nice_time_ms` / `system_time_ms` / `idle_time_ms` | integer \| null | yes | Cumulative since boot. | `CpuInfoReader.java:859-870` |
| `iowait_time_ms` / `irq_time_ms` / `softirq_time_ms` | integer \| null | yes | Cumulative since boot. | `:865-867` |
| `steal_time_ms` | integer \| null | yes | **The only per-CPU source of steal in a bugreport.** | `:868` |
| `guest_time_ms` / `guest_nice_time_ms` | integer \| null | yes | Cumulative since boot. | `:869-870` |
| `busy_pct_since_boot` | number \| null | yes | **Derived** — busy ÷ sum of all buckets. Carries `busy_pct_derivation`. AOSP publishes only raw milliseconds and never this percentage, so an implementation computing it needs a unit test against a hand-summed row. | derived |
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
| `art_java_monitor.channel_present` | boolean | yes | `true` — the `log_contention` gate was on. | `monitor.cc:458` |
| `art_java_monitor.threshold_ms` | integer \| null | yes | **1000 on a debuggable build.** Carries `threshold_derivation`. | `monitor.cc:53-54` |
| `art_java_monitor.wait_ms_max` / `wait_ms_min` | integer \| null | yes | 1780 / 105. | measured |
| `art_java_monitor.events_in_system_server` | integer | yes | **0.** | measured |
| `art_java_monitor.events[]` | array | yes | `{timestamp, process, pid, tid, owner_name, owner_tid, owner_method, owner_file, owner_line, waiters, waiter_method, wait_ms, wait_verbatim}`. **`wait_verbatim` is kept so a consumer can see the unit ART chose.** `waiters` is a queue depth and the most diagnostic single field. | `monitor.cc:409-418,597`; `time_utils.cc:53-75` |
| `art_runtime_mutex` | object | yes | `{channel_present, event_count, format, short_format, zero_reason}`. `event_count` **0** — present, enabled, silent. | `mutex.cc:42,533,567-571,576-591` |
| `art_rate_limiting` | object | yes | `{java_monitor_is_sampled, java_monitor_sampling_rule, java_monitor_sample_pct_observed, java_monitor_threshold_source, runtime_mutex_is_rate_limited, runtime_mutex_rate_limit, counting_caveat}`. **`counting_caveat` is mandatory prose: no field in `contention` is a contention count.** | `monitor.cc:529-536`; `monitor_android.cc:105`; `runtime.cc:1589` |
| `dvm_lock_sample` | object | yes | `{sampled_event_count: 168, in_system_server: 149, system_server_pct: 0.8869, field_order_source, example}`. Named `sampled_event_count`, **not** `event_count` — the name is the warning. | `monitor_android.cc:35-108` |
| `provenance` | object | yes | `capture_kind: live_log`. **The window is the log buffer, not the CPU window.** | — |

##### `load`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `sources[]` | array of object | yes | `{source: bugreport_header \| uptime_section \| dumpsys_cpuinfo, load_1m, load_5m, load_15m, uptime_ms_at_read, uptime_qualifier, line_start}`. **All three readouts are published, because on this archive they disagree.** | `ProcessCpuTracker.java:829-838`; `dumpstate.cpp:1687` |
| `load.uptime_ms_at_read` | integer \| null | yes | **Required whenever `load_1m` is present.** | header / `uptime` output |
| `online_cpus` | integer \| null | yes | Load is normalised by runnable CPUs, so a 1-minute average of 10.42 on 4 CPUs is 261 % runnable demand. | `ProcessCpuTracker.java:139-144` |
| `load_15m_implies_uptime_ms` | number \| null | yes | `load_15m × 15 × 1000` — the amount of history the 15-minute average implies it has seen. **Published so the contradiction is visible, not resolved.** | derived |
| `load_history_conflict` | boolean | yes | `true` when `load_15m_implies_uptime_ms` exceeds `uptime_ms_at_read`. **`uptime_ms_at_read` is `null` in the reference archive** — the header prints uptime only to minute granularity — so the comparison the boolean depends on is not available there; the value is then `null` and the conflict is recorded as an `evidence_gaps[]` entry with `reason: UNVERIFIABLE` rather than guessed. | derived |
| `provenance` | object | yes | | — |

> **Why `uptime_ms_at_read` is required.** Load averages are EWMAs decayed by the kernel's
> `LOAD_FREQ`. On a device with seconds of uptime the 1-minute average has only ever seen boot
> work, so it is arithmetically meaningless as a load signal. The reference archive is exactly
> that case, and it is *contradictory* in a second way: the header reports sub-minute uptime
> while `load_15m` implies roughly 700 s of history, which cannot exist on a boot that young.
> Nothing in the archive adjudicates why — see the `AMBIGUOUS_SOURCE` entry for
> `load.load_15m_implies_uptime_ms`. Either way the number cannot be a steady-state load, which
> is why this schema publishes the conflict (`load_history_conflict`) instead of a boolean
> verdict about it.

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

Two channels, two aggregate rows, and they disagree. That disagreement is correct:

| Source | Normalisation |
| --- | --- |
| `dumpsys cpuinfo` `TOTAL` row | divided by a time span in which the device may be partly idle |
| `top` aggregate row | normalised across **all** online cores (`400%cpu` on a 4-core device) |

Both are correct over **different windows** (cpuinfo's is a short background sample taken
before the dump; `top`'s is its own mid-dump sample) and against **different denominators**.
Note also that the earlier reading of "9.2 user / 17k nice" was a **misreading**: the archive
says **`17% kernel`**, not nice — nice is the `12%` in the `top` line. **A schema that merges
these two rows into one "CPU %" is unsound**, so the schema keeps them as two entries of the
`utilization` array, each with its own window.

**On the high `irq`.** `stealTimeMillis = 0` on every core and `0%host` in `top` mean the guest
was **not** descheduled by the hypervisor. Combined with the product being an emulator
(`ro.ranchu`, and a fingerprint segment naming an emulator), the elevated `irq` in the
`cpuinfo` window is best explained as **host→guest device interrupt injection being accounted
in the guest's `irq` bucket, not by a guest driver defect**. **This is a labelled inference, not
a schema field** — the schema carries only the measured `irq` and `steal`. A finding may carry
`inference: true` and the supporting observations, and must then carry
`confidence: "inferred"`.

