# Schema decisions and known limitations

> **Promoted contract limitations for `UC-03`.**
> See [`index.md`](index.md).


## 5. Schema decisions

Shared schema choices preserve evidence identity, clock semantics and comparable outputs.

### 5.1 The D1–D5 decisions

| # | Question | Decision | Why |
| --- | --- | --- | --- |
| **D1** | Capture-instant key name | **`provenance.captured_at`**, RFC 3339 with an explicit offset (`Z` qualifies). Either the `== dumpstate:` banner or the archive filename may supply it; record which in `captured_at_time_base`. | The banner and filename both derive from dumpstate capture time; the time-base field records which source supplied it. |
| **D2** | Freshness threshold | **3,600 s (1 hour)**, and the cross-boot escalation is **mandatory, not advisory**. | A one-hour cutoff and mandatory cross-boot check separate recent evidence from retained prior-boot data; the reference contains 126-day-old traces and an 8 h 39 m prior-boot uptime. |
| **D3** | Process identity | Proposed `<boot_epoch>/<start_seq>` is restricted to one demonstrated AMS lifetime; cross-restart identity remains unresolved. | `ProcessList.java:463,2156` scopes start sequence to one AMS instance; system_server restarts do not reset kernel boot identity. See [identity limits](02-evidence-conventions.md#28-identity). |
| **D4** | Provenance nesting | **Flat `provenance` object on every finding**, not nested under a `capture` wrapper. | Per-finding provenance keeps evidence clocks distinct from canonical capture context and avoids independently derived copies. |
| **D5** | Time-base enum | **Keep BOTH**: `captured_at_time_base` (the 11-value enum) *and* `capture_kind` (the measurement-kind enum). | Time base names the clock; capture kind names the measurement process, such as a persisted aggregate or background sampler. |

### 5.2 Reference facts

These measured reference facts explain the required distinctions in artifact output.

| Fact | Reference value and interpretation |
| --- | --- |
| Crash count | **271 distinct crashes.** The 3 extras are dropbox-inlined copies of logcat (`ActivityManagerService.java:9890-9900`). Decomposition: 38 in the first `SYSTEM LOG`, **3** in the broader `DUMPSYS` block (zero in the explicit Dropbox crash section), 233 in the trailing `SYSTEM LOG`. |
| PID recycling | 271 crashes → **271 distinct monotonic PIDs** (1116 → 9985), no recycling *within* the loop. |
| `dumpsys procrank` | **Present and populated** — archive line 192650, 178 rows. It is a `bugreport_procdump` sub-section, **not** a dumpsys section (`dumpstate.cpp:1693-1694` → `smapinfo.cpp:1233-1242`). |
| CPU breakdown | `17% **kernel**`. Nice is `12%` in the `top` line (archive line 57472). |
| Swap snapshots | **4:** 18,688 / 18,176 / 57,088 / 230,912 K. The two intermediate readings are what make the collector-induced growth visible. |
| `Cmd line:` | from `art/runtime/signal_catcher.cc:55-69`, chosen to match debuggerd's format. **All 251 hits are inside VM traces** (184 in `VM TRACES JUST NOW`, 67 in `VM TRACES AT LAST ANR`) — trace evidence rather than package inventory. |
| `Current user ids: [10]` | A **SystemUI app-op monitor** string at archive line 1957. The user list is `dumpsys user`: `Current user: 10` at line 613374, `UserInfo{0:null:813}` / `UserInfo{10:Driver:412}` at 613377 / 613416. |
| Car packages | 13 is the count of `com.android.car.*` **processes** (`ProcessRecord.java:1396`); the package dump has **100+** distinct `com.android.car.*` packages. Both are stored, as `car_service_process_names` and `car_service_packages`. |
| Per-package entries | **282 `Package [` blocks** + **594 per-user state blocks** measured in the `DUMP OF SERVICE package:` section. The expected arithmetic `282×2 + 14×2 = 592` falls two short — see [`system.json`](05-system-artifact.md#42-systemjson) open questions. |
| `DumpState.java` | No central android.os registry by that name. Package/overlay-manager DumpState classes exist for bookkeeping. AMS registers several services; other registration sites contribute to the runtime registry. |
| `am/ProcessStats.java` | The service is am/ProcessStatsService.java; the data model ProcessStats still exists in core/java/com/android/internal/app/procstats. |
| `anrd` | A **CPU-starvation ftrace collector** (`system/extras/ANRdaemon/README:1-14`; `ANRdaemon.cpp:236-260`). It declares no ANR, writes no `/data/anr/` entry, **and was not found in the inspected Lima phone product output**; that does not establish its presence on the reference automotive device. Contributes no field. |
| ANR trace timestamps | The trace **does** carry them — `backtrace.cpp:49`, ns + tz, 31 `----- pid N at <ts> -----` lines. **Staleness here is provable, not inferred.** |

### 5.3 Domain design choices

These choices keep artifact interpretation explicit:

| # | Decision | Rationale |
| --- | --- | --- |
| M1 | **`source` carries no time.** The capture instant lives in `capture`; evidence times live in `provenance`. | Separating these fields keeps *archive identity* and *capture instant* separable, which is the same separation as `provenance.captured_at` vs `source.captured_at`. |
| M2 | **`collection` is a top-level envelope key with one schema in all six JSON artifacts.** | It is the only mechanism that answers "is this number self-inflicted?" from one artifact read in isolation, and it expresses *both* directions with the same `collector_perturbed` / `collector_perturbed_basis` pair. |
| M3 | `anr.json` has no generic `timeout_ms`. | InputDispatcher can print `Waited Nms` (`InputDispatcher.cpp:6511-6517`); observed wait, configured timeout and generic ANR budget are different quantities. Preserve the reason text. |
| M4 | Omit original/compressed ZRAM sizes from standard meminfo-derived fields. | `sysmeminfo.cpp:178-206` reads physical mem_used_total, skipping the first two mm_stat values. This does not rule out optional raw sysfs or OEM output. |
| M5 | **`cpu.json` keeps `utilization` and `contention` separate, and `utilization` and `per_cpu_core` separate.** | They differ by *units* and *window*, not by judgement. A `windowed ratio`, a `cumulative-since-boot millisecond counter`, and a `blocking-event count over the log window` are three different quantities. Merging any two produces a number that means nothing. |
| M6 | **`dvm_lock_sample` is `sampled_event_count`, not `event_count`.** | `Monitor::LogContentionEvent` fires with probability proportional to wait duration (`monitor.cc:529-536`). The name is the warning: a consumer who sees `sampled_event_count` cannot mistake 168 for a contention count. |
| M7 | **`contention.art_rate_limiting.counting_caveat` and `kill_evidence.am_low_memory_meaning` are mandatory prose fields.** | Two values that a reader will otherwise interpret the wrong way round. Carrying the interpretation as data makes it visible instead of implicit. |
| M8 | **`README.md` has no envelope; the seven-artifact list includes it.** | `02-requirements.md` lists it as an artifact. It is Markdown, references the JSON structures, and its "top-level keys" are its nine numbered sections. |

---

## 6. Known Limitations

Deliberately unspecified scope, with reasons.

### 6.1 Deliberately unspecified

| Limitation | Why |
| --- | --- |
| **A dumpsys section allowlist.** | Sections are resolved at runtime by binder-service enumeration (`dumpsys.cpp:261`, `dumpsys.cpp:250-274`). The reference archive holds ~250 distinct `dumpsys <name>` lines. **Any schema with a fixed section table is wrong by construction** — see [§3.4](03-build-variants.md#34-dumpsys-sections-are-an-open-set). |
| **A `userdebug` vs `eng` distinction.** | `PropertiesHelper::IsUserBuild()` is a literal compare against `"user"` and **both take the same branch**. There is no such distinction anywhere in dumpstate. |
| **A "capture succeeded" guarantee.** | Collection can fail or return partial evidence. The normal privilege-drop failure returns RunStatus::ERROR (`dumpstate.cpp:1999-2000`); telephony/Wi-Fi paths return early. ZIP readability alone does not prove success. |
| **Per-UID CPU scope.** | Android has per-UID accounting: `KernelCpuUidTimeReader.java:196-208` reads uid_cputime, and `BatteryStats.java:6688-6698` prints UID CPU totals. Those battery intervals differ from cpuinfo. No per_uid array is specified in this draft; that is scope, not absence of platform support. |
| **An ANR `timeout_ms`.** | No generic field: a reason may include an observed wait, which is not necessarily the configured trigger budget. |
| **ZRAM compressed/original sizes.** | Not printed by the inspected standard meminfo/procrank summaries; optional/raw sources remain possible. |
| **A calendar or locale library.** | Timestamps are RFC 3339 with an explicit offset; anything locale-formatted is `*_raw` plus a parsed sibling plus `parsed_confidence`. The spec does not define the locale. |
| **Severity thresholds as tunable parameters.** | The five levels and their definitions are fixed in [§2.6](02-evidence-conventions.md#26-severity). Making them per-run configurable would make artifacts incomparable, which defeats the point of a shared convention. |
| **The 3600 s threshold as a configurable value.** | Same reason. Fixed so artifacts are comparable across runs and tools. |
| **A per-line `evidence` for every field of every domain object.** | Evidence is one locator per finding, plus `corroborating_evidence[]`. Making it per-field would multiply the artifact size by two orders of magnitude for no additional traceability — the finding's `line_start`/`line_end` already spans the record. |
| **`README.md` byte-level layout.** | Its nine sections, their order, their mandatory/optional status, and the cold-boot and cross-boot banners are specified. Exact line breaks and table column widths are not; they are presentation, not contract. |

### 6.2 Known gaps in the evidence base

These are limits of what the reference archive and this AOSP tree can prove.
They are listed per artifact in [§4](04-readme-artifact.md#4-the-seven-artifacts) and collected here.

| Gap | Consequence |
| --- | --- |
| **top/procrank sources.** | `external/toybox/toys/posix/ps.c` implements top; `system/memory/libmeminfo/tools/procrank.cpp` and `libsmapinfo/smapinfo.cpp` implement procrank. Both binaries are present in the inspected Lima phone output. OEM binary equivalence remains unverified. |
| **am_crash emitter.** | `ActivityManagerService.java:9271-9279` calls generated `writeAmCrash`; actual arguments start PID, user ID, contrary to the tag declaration names. |
| **No `dumpstate` section-size budget.** | None was found in `dumpstate.cpp`, `dumpstate.h`, `DumpstateUtil.{h,cpp}`, or `bugreportz.{cpp,h}`. The 64 MB main entry is **not** evidence of a cap. Timeouts are the only verified truncation mechanism. |
| **`dumpstate.strict_run` / `parallel_run` / `dry_run` / `unroot` were not verified on the device.** | Defaults are `dry_run=false`, `unroot=false`, `parallel_run=true`, `strict_run=true`. If any is overridden on the target, section presence and shape change for reasons unrelated to build type. |
| **~60 AIDL dumpsys sections unresolved.** | Each is a per-HAL `publishBinderService` in the owning HAL's source, and this product uses *example/mock* implementations. They are treated as open-set, as required. |
| **The archive's timezone offset is recoverable only indirectly.** | `localtime()` is used at `dumpstate.cpp:802`, `:2744`, and `DumpstateInternal.cpp:148`. The offset appears only in the `----- pid N at <ts>+0700 -----` lines. `ro.runtime.firstboot` disagrees with the documented solved boot instant by about 19.1 s and is recorded but not preferred. |
| **Logcat has no year in the observed threadtime format.** | Use the heuristic specified in §2.4 when a capture year exists, with an uncertainty note for wrap/clock changes. The old claim that every Dec–Jan line receives one year contradicted that rule. |
| **Uptime at capture is a derived figure, not a printed one.** | It resolves to **30 s ± 0.6 s** from the solved boot instant `B ≈ 00:26:47.33` ± 0.10 s. No archive-internal source states seconds-level uptime *directly*: the `== dumpstate:` banner is second-truncated, and `header_raw` / `section_raw` are minute-granular. The figure therefore rests on the derived bracket, and its ± 0.6 s uncertainty is dominated by the 1-second truncation of those wall clocks..) |
| **`am_kill` `Reason` vocabulary and `Rss\|2\|2` decoding are spec-derived, not observation-derived.** | `am_kill` is zero on this archive, so neither could be validated against a real sample. |
| **Upstream usagestats user scope.** | `UsageStatsService.java:2038-2076` iterates mUserState and prints each user ID. Unlocked users receive normal dumps; locked users may have pending events. Preserve the enclosing user ID; absence is not proof of no usage. |
| **`ProcessStats` per-PSS-bucket window widths are unlocated.** | `aggregation_window` is built from printed fields, not a declared bucket width. |
| **Trace pruning history is unknown.** | `StackTracesDumpHelper.java:516-521` prunes count and files older than one day. With default 64, `i > max` can retain 65 recent files; it does not explain 126-day retention. Count limits alone cannot establish whether pruning ran or failed. |
| **SharedUser header source.** | `Settings.java:5766-5773` prints the shared-user name, identity hash and appId. |

### 6.3 Where the detail lives

Field definitions and worked examples are owned by this schema directory.
The [shared envelope](01-envelope.md), [evidence conventions](02-evidence-conventions.md), [generation rules](12-generation-rules.md) and each artifact page define the promoted contract together.
The [source provenance lookup](13-source-provenance.md) retains module paths, installed-name distinctions, evidence emitters and revision limits.
The [fact-check](../../../verification/results/aosp-source-verification.md) owns corrected platform claims and remaining source/design questions.
The shared conventions, domain definitions and generation rules form one contract; independent research notes do not supply additional required keys.
