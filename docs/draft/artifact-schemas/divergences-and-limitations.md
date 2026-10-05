# Divergences resolved and known limitations

> **Draft.** See [`index.md`](./index.md).

Original §5–§6 of `artifact-schemas.md`: the D1–D5 rulings, the facts corrected while writing, and
what this specification still cannot tell a reader.

## 5. Divergences Resolved

The five team deliverables were written in parallel and could not see each other. This section
records what was decided and **why**, so a future reader can see the reasoning rather than just
the outcome.

### 5.1 The D1–D5 decisions

| # | Question | Decision | Why |
| --- | --- | --- | --- |
| **D1** | Capture-instant key name | **`provenance.captured_at`**, RFC 3339 with an explicit offset (`Z` qualifies). Either the `== dumpstate:` banner or the archive filename may supply it; record which in `captured_at_time_base`. | The Technical Lead wrote `capture.captured_at_utc` derived from the **archive filename**; the Architect wrote `provenance.captured_at` derived from the **`== dumpstate:` banner**. Both are `ds.now_` from `dumpstate.cpp:3805` — **the same instant**. Renaming buys nothing, and `captured_at_time_base` exists precisely to record which source was used. The Tech Lead's nested `capture` wrapper was also flattened to `provenance` (that is D4). |
| **D2** | Freshness threshold | **3,600 s (1 hour)**, and the cross-boot escalation is **mandatory, not advisory**. | The Platform Developer proposed 24 h. On the reference archive both give the same answer — the gap is 126 days, so anything from 1 min to 1 day separates them — so neither is *wrong* on this evidence. They diverge on a 3-hour-old artifact. **The only distinction that must be made is "same boot" vs "previous boot", and the previous boot here is 8 h 39 m of uptime away** (`cross_boot_reference = 31190738`). A 1 h threshold catches that; 24 h would not and would have labelled the stale ANR set current. |
| **D3** | Process identity | **`process_key = "<boot_epoch>/<start_seq>"`** as primary. Fall back to the Architect's `"<process_name>#<uid>@<start_ordinal>"` **only** when `start_seq` is absent (a `dumpsys`-only sighting). `pid` is an observation attribute, never a key. | The App Developer's form is strictly stronger where available. `start_seq` is **AMS's own monotonic counter** (`ProcessList.java:2155-2157`), so it is globally unique for the life of the AMS process — and **AOSP itself uses it as the instance identity** (`AMS:930`, `:4475`, `:4814`). The Architect's ordinal is computed by us, is window-local, and is `null` for a process that started before the window. `start_seq` must be paired with `boot_epoch` because it resets to 0 on a `system_server` restart (`ProcessList.java:1858`) — which is exactly what `system.json#/capture/boot/at` supplies. |
| **D4** | Provenance nesting | **Flat `provenance` object on every finding**, not nested under a `capture` wrapper. | The Technical Lead nested it under `capture.*`. Per-finding provenance is the whole point; a wrapper that repeats capture context on every finding invites exactly the "one derived value, two answers" problem rule 6 of [§2.9](./conventions-evidence.md#29-null-absent-and-raw) forbids. Every other key in the wrapper has a 1:1 counterpart in the flat block, so **no data is lost** by flattening. |
| **D5** | Time-base enum | **Keep BOTH**: `captured_at_time_base` (the 11-value enum) *and* `capture_kind` (the measurement-kind enum). | The Architect's D5 recommendation was to emit one alongside the other, and the Platform Developer's proposed-changes section made the same argument independently. **They answer different questions.** The 11-value enum is what makes the year-less-logcat and cross-boot cases nameable; `capture_kind` is what distinguishes `persisted_aggregate` (which catches the 128-day-old `procstats` blocks) from `background_sampler` (which rescues `dumpsys cpuinfo`). Collapsing either into the other loses a distinction that turned out to be load-bearing in practice. |

### 5.2 Corrected facts

Several team files and one earlier brief carried wrong versions of these. All are measured, not
inferred, and each is carried into the spec above.

| Fact | Earlier claim | Correct value |
| --- | --- | --- |
| Crash count | 274 | **271 distinct crashes.** The 3 extras are dropbox-inlined copies of logcat (`ActivityManagerService.java:9890-9900`). Decomposition: 38 in the first `SYSTEM LOG`, **3** in `DROPBOX SYSTEM APP CRASHES`, 233 in the trailing `SYSTEM LOG`. |
| PID recycling | ~3 cycles in 274 crashes | 271 crashes → **271 distinct monotonic PIDs** (1116 → 9985), no recycling *within* the loop. The earlier reading conflated the PID range span with the crash count and mistook the dropbox duplicates for kernel recycling. |
| `dumpsys procrank` | absent | **Present and populated** — archive line 192650, 178 rows. It is a `bugreport_procdump` sub-section, **not** a dumpsys section (`dumpstate.cpp:1693-1694` → `smapinfo.cpp:1233-1242`). |
| CPU breakdown | 9.2 user / 17k *nice* | `17% **kernel**`. Nice is `12%` in the `top` line (archive line 57472). |
| Swap snapshots | 2 (18,688 K → 230,912 K) | **4:** 18,688 / 18,176 / 57,088 / 230,912 K. The two intermediate readings are what make the collector-induced growth visible. |
| `Cmd line:` | from AMS | from `art/runtime/signal_catcher.cc:55-69`, chosen to match debuggerd's format. **All 251 hits are inside VM traces** (184 in `VM TRACES JUST NOW`, 67 in `VM TRACES AT LAST ANR`) — the ANR team's evidence, not `apps.json`'s. |
| `Current user ids: [10]` | the device user list | A **SystemUI app-op monitor** string at archive line 1957. The user list is `dumpsys user`: `Current user: 10` at line 613374, `UserInfo{0:null:813}` / `UserInfo{10:Driver:412}` at 613377 / 613416. |
| Car packages | 13 | 13 is the count of `com.android.car.*` **processes** (`ProcessRecord.java:1396`); the package dump has **100+** distinct `com.android.car.*` packages. Both are stored, as `car_service_process_names` and `car_service_packages`. |
| Per-package entries | 660 | **282 `Package [` blocks** + **594 per-user state blocks** measured in the `DUMP OF SERVICE package:` section. The expected arithmetic `282×2 + 14×2 = 592` falls two short — see [`system.json`](./artifact-system.md#42-systemjson) open questions. |
| `DumpState.java` | the section registry | **Does not exist in Android 15.** The registry moved to `ServiceManager.addService(...)` in `ActivityManagerService.setSystemProcess()` (`:1926-1937`); the native `dumpsys` resolves names by asking the service manager (`dumpsys.cpp:261`). |
| `am/ProcessStats.java` | — | is now **`am/ProcessStatsService.java`**. Any citation of `am/ProcessStats.java` is stale. |
| `anrd` | the ANR detector | A **CPU-starvation ftrace collector** (`system/extras/ANRdaemon/README:1-14`; `ANRdaemon.cpp:236-260`). It declares no ANR, writes no `/data/anr/` entry, **and is not built for this product**. Contributes no field. |
| ANR trace timestamps | "none inside the trace" | The trace **does** carry them — `backtrace.cpp:49`, ns + tz, 31 `----- pid N at <ts> -----` lines. **Staleness here is provable, not inferred.** |

### 5.3 Design decisions this document made beyond D1–D5

Places where merging required a judgement the contract did not dictate:

| # | Decision | Rationale |
| --- | --- | --- |
| M1 | **`source` carries no time.** The capture instant lives in `capture`; evidence times live in `provenance`. | The Architect's `source` block held `captured_at`, `uptime_at_capture_seconds`, and `captured_at_time_base` alongside `archive_sha256`. Splitting them keeps *archive identity* and *capture instant* separable, which is the same separation as `provenance.captured_at` vs `source.captured_at`. |
| M2 | **`collection` is a top-level envelope key with one schema in all six JSON artifacts.** | The Platform Developer recommended a cross-artifact ledger and the contract adopted it. It is the only mechanism that answers "is this number self-inflicted?" from one artifact read in isolation, and it expresses *both* directions with the same `collector_perturbed` / `collector_perturbed_basis` pair. |
| M3 | **`anr.json` drops the `timeout_ms` field.** | `DEFAULT_ANR_TIMEOUT` and `DEFAULT_INPUT_ANR_TIMEOUT` do not exist anywhere in `android-15.0.0_r17`. The input-dispatch timeout is a reason *string* only (`InputManagerCallback.java:350-351`). A field that cannot be sourced does not go in the spec. |
| M4 | **`memory.json` drops `zram_orig_size_kb` and `zram_compressed_size_kb`.** | `SysMemInfo::MemZramDevice` skips `mm_stat` fields 1 and 2 and takes `mem_used_total` (`sysmeminfo.cpp:178-206`). No bugreport output exposes the compressed/original sizes, so the fields would be permanently unpopulatable. A derived `compression_ratio` with `*_derived: true` is legitimate. |
| M5 | **`cpu.json` keeps `utilization` and `contention` separate, and `utilization` and `per_cpu_core` separate.** | They differ by *units* and *window*, not by judgement. A `windowed ratio`, a `cumulative-since-boot millisecond counter`, and a `blocking-event count over the log window` are three different quantities. Merging any two produces a number that means nothing. |
| M6 | **`dvm_lock_sample` is `sampled_event_count`, not `event_count`.** | `Monitor::LogContentionEvent` fires with probability proportional to wait duration (`monitor.cc:529-536`). The name is the warning: a consumer who sees `sampled_event_count` cannot mistake 168 for a contention count. |
| M7 | **`contention.art_rate_limiting.counting_caveat` and `kill_evidence.am_low_memory_meaning` are mandatory prose fields.** | Two values that a reader will otherwise interpret the wrong way round. Carrying the interpretation as data makes it visible instead of implicit. |
| M8 | **`README.md` has no envelope; the seven-artifact list includes it.** | `requirements.md` lists it as an artifact. It is Markdown, references the JSON structures, and its "top-level keys" are its nine numbered sections. |

---

## 6. Known Limitations

What this specification deliberately does **not** specify, and why. Each entry is a decision,
not an omission.

### 6.1 Deliberately unspecified

| Limitation | Why |
| --- | --- |
| **A dumpsys section allowlist.** | Sections are resolved at runtime by binder-service enumeration (`dumpsys.cpp:261`, `dumpsys.cpp:250-274`). The reference archive holds ~250 distinct `dumpsys <name>` lines. **Any schema with a fixed section table is wrong by construction** — see [§3.4](./build-variant.md#34-dumpsys-sections-are-an-open-set). |
| **A `userdebug` vs `eng` distinction.** | `PropertiesHelper::IsUserBuild()` is a literal compare against `"user"` and **both take the same branch**. There is no such distinction anywhere in dumpstate. |
| **A "capture succeeded" guarantee.** | `DropRootUser()` failure returns early from the whole dump (`dumpstate.cpp:1999`, `:2068`, `:2179`) and the archive still looks well-formed. The spec requires truncation to be *detectable*; it cannot make it impossible. |
| **Per-UID CPU accounting.** | `/proc/uid_stat` does not exist in Android 15. `ProcessCpuTracker` sums per-process deltas instead. There is no separate evidence source to model. |
| **An ANR `timeout_ms`.** | The constants do not exist in this tree and the reason string does not carry a duration (M3). |
| **ZRAM compressed/original sizes.** | Not exposed in any bugreport output (M4). |
| **A calendar or locale library.** | Timestamps are RFC 3339 with an explicit offset; anything locale-formatted is `*_raw` plus a parsed sibling plus `parsed_confidence`. The spec does not define the locale. |
| **Severity thresholds as tunable parameters.** | The five levels and their definitions are fixed in [§2.6](./conventions-evidence.md#26-severity). Making them per-run configurable would make artifacts incomparable, which defeats the point of a shared convention. |
| **The 3600 s threshold as a configurable value.** | Same reason. Fixed so artifacts are comparable across runs and tools. |
| **A per-line `evidence` for every field of every domain object.** | Evidence is one locator per finding, plus `corroborating_evidence[]`. Making it per-field would multiply the artifact size by two orders of magnitude for no additional traceability — the finding's `line_start`/`line_end` already spans the record. |
| **`README.md` byte-level layout.** | Its nine sections, their order, their mandatory/optional status, and the cold-boot and cross-boot banners are specified. Exact line breaks and table column widths are not; they are presentation, not contract. |

### 6.2 Known gaps in the evidence base

These are limits of what the reference archive and this AOSP tree can prove. They are listed
per artifact in [§4](./artifact-readme.md#4-the-seven-artifacts) and collected here.

| Gap | Consequence |
| --- | --- |
| **`top` and `procrank` binaries are not in this source drop.** | Every `top`-derived field's semantics is unverified — whether `%CPU` is scaled by thread count, what `0%host` means, the denominator behind `400%cpu`. Only the *invocation* is citable. |
| **No `AM_CRASH` write site in `frameworks/base`, `system/`, or `packages/modules/`.** | `am_crash` slot order is decoded **by observation**, flagged in `channels[].decode_warning`. The tag is declared and emitted; the emitter was not found. |
| **No `dumpstate` section-size budget.** | None was found in `dumpstate.cpp`, `dumpstate.h`, `DumpstateUtil.{h,cpp}`, or `bugreportz.{cpp,h}`. The 64 MB main entry is **not** evidence of a cap. Timeouts are the only verified truncation mechanism. |
| **`dumpstate.strict_run` / `parallel_run` / `dry_run` / `unroot` were not verified on the device.** | Defaults are `dry_run=false`, `unroot=false`, `parallel_run=true`, `strict_run=true`. If any is overridden on the target, section presence and shape change for reasons unrelated to build type. |
| **~60 AIDL dumpsys sections unresolved.** | Each is a per-HAL `publishBinderService` in the owning HAL's source, and this product uses *example/mock* implementations. They are treated as open-set, as required. |
| **The archive's timezone offset is recoverable only indirectly.** | `localtime()` is used at `dumpstate.cpp:802`, `:2744`, and `DumpstateInternal.cpp:148`. The offset appears only in the `----- pid N at <ts>+0700 -----` lines. `ro.runtime.firstboot` disagrees with the AMS boot derivation by 6.5 s and is recorded but not preferred. |
| **Logcat has no year.** | The year is taken from `capture.captured_at` and the wrap heuristic is undetectable from inside the archive. A bugreport spanning 31 Dec → 1 Jan gets one year on all lines. Mitigated by `notes` `LOGCAT_YEAR_UNRESOLVED`, not solved. |
| **Uptime at capture is a derived figure, not a printed one.** | It resolves to **30 s ± 0.6 s** from the solved boot instant `B ≈ 00:26:47.33` ± 0.10 s. No archive-internal source states seconds-level uptime *directly*: the `== dumpstate:` banner is second-truncated, and `header_raw` / `section_raw` are minute-granular. The figure therefore rests on the derived bracket, and its ± 0.6 s uncertainty is dominated by the 1-second truncation of those wall clocks. (Two earlier figures, 27 s and 17.118 s, were both wrong — 27 s misread a wall clock as an elapsed time and 17.118 s was a process age. See [`system.json`](./artifact-system.md#42-systemjson).) |
| **`am_kill` `Reason` vocabulary and `Rss\|2\|2` decoding are spec-derived, not observation-derived.** | `am_kill` is zero on this archive, so neither could be validated against a real sample. |
| **`dumpsys usagestats` user coverage is unconfirmed.** | If it is user-0-only, every `activity_events[]` row describes the wrong profile on a multi-user device. This must be checked before `activity_events` is trusted. |
| **`ProcessStats` per-PSS-bucket window widths are unlocated.** | `aggregation_window` is built from printed fields, not a declared bucket width. |
| **`maybePruneOldTraces` did not run for 126 days on this device.** | Either the retention cap was raised by device config, or `if (i > max)` (`StackTracesDumpHelper.java:522`) leaves index 64. Unresolved which. It does not change the schema — it is a fact for `README.md` §8. |
| **The `SharedUser [` header printer was not isolated.** | `apps.json#/inventory/shared_uids[]` ids are read from the archive, not from a cited printer. |

### 6.3 Where the detail lives

The per-field tables in this document are **complete in coverage but condensed in rows**. The
owning team's exhaustive row-level field tables are preserved and were not edited:

| Owning file | Preserved detail |
| --- | --- |
| `tmp/aosp-team/01-architect-conventions-and-readme.md` | Part B field tables (§B.1–B.8), the full `captured_at_time_base` mapping table, the closed suppression-reason vocabulary with per-archive-instance counts, the `README.md` section-by-section specification, and Appendix A's AOSP citation index (~200 `path:line` pairs). |
| `tmp/aosp-team/02-anr-and-crashes.md` | §6 and §13 field tables, the six ANR channels and seven crash channels with completeness ratings, the `ANR FILES` listing verbatim, and §15's consolidated citation list. |
| `tmp/aosp-team/03-memory-and-cpu.md` | §4.4 and §5.4 field tables, the twenty memory channels (M1–M20) and eighteen CPU channels (C1–C18) with presence and window/basis, and §6's citation index. |
| `tmp/aosp-team/04-apps-and-system.md` | §1.2 and §2.4 field tables, the full `PackageEntry` / `ProcessEntry` / evidence-channel tables, the build-type dependency table, and §6's citation list. |
| `tmp/aosp-team/05-module-and-provenance-map.md` | §1's module → source path → installed-artifact table (including the three cases where the installed name differs from the module name), §2.2's gated-section table, and §3.2's section → emitting-source table. |

**Where this document and a team file disagree on a value, this document wins** — it is the
merged, binding specification. Where a team file carries a value this document does not repeat,
the team file's Appendix B or its worked example is the authority, and it was verified against
the reference archive during this merge.

