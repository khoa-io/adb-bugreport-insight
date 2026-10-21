# AOSP documentation fact-check — 2026-10-15

## Scope and verdict

**Verdict: technical corrections applied; draft schema and software conformance remain open.**
The user requested a second check of `docs/` using Lima MCP and the aosp-mac skill.
Reviewed Android producer/collector claims in deferred schemas, examples and requirements.
Spot-checked archive/MCP design against Rust sources; its discrepancy register remains authoritative and unresolved.

Affected deferred scope: DRAFT-REQ-GEN-04, DRAFT-REQ-ART-02 through DRAFT-REQ-ART-08 and DRAFT-ARCH-CORE-ART.
No ID, requirement allocation, product code or committed obligation changed.
Corrections do not promote analyzers/artifacts into shipped scope.
Unchanged element ownership preserves traceability mappings.

Repository baseline: `8d38208b835932d248071c7a8ad13434734b7923` plus the existing uncommitted TASK011/TASK012 documentation, diagram and working context work.
That work was retained.
This review adds corrections to proposal pages, this record and navigation/working-context notes.
No staging, commit, VM provisioning, AOSP synchronization or build was performed.

## Evidence environment

Lima MCP `run_shell_command` was used with argv arrays and absolute guest directories.
The aosp-mac quick checks confirmed both mounted disks and an existing repo checkout:

| Item | Observed |
| --- | --- |
| Source mount | `/mnt/lima-android-src`, ext4 disk mounted, 314G capacity |
| Output mount | `/mnt/lima-android-out`, ext4 disk mounted, 196G capacity |
| Manifest default revision | `refs/tags/android-15.0.0_r17` |
| Installed output | `target/product/emu64a` |
| Product definition | `sdk_phone64_arm64`, `PRODUCT_DEVICE=emu64a` |
| Installed system properties | Android 15, SDK 35, `AP4A.250205.002.C1`, `eng` |
| Installed tools found | `system/bin/top`, `system/bin/procrank`, `system/bin/bugreport_procdump` |
| Optional tools not found in installed partitions inspected | `anrd`, `carwatchdogd` |
| Container state | No active `aosp-build` development container; source/output inspected directly through guest mounts |

This phone build differs from the reference automotive userdebug device; absent installed binaries here cannot prove absence there.
Existing files prove neither fresh build success nor current build-chain health.

Relevant project revisions were read with `git -C <project> rev-parse HEAD`; tracked source content in all eight projects was clean against HEAD (`git diff --quiet HEAD --`).

| Project | HEAD |
| --- | --- |
| frameworks/base | `d41da232c75a51b4a9023d8d418d7c0a5120c0e2` |
| frameworks/native | `793ee6480f4f491166c775d350439b9d86701f66` |
| system/logging | `22584447e927944ebcc93b964549a29f91d7756b` |
| system/memory/libmeminfo | `c4a78c8e4e63b13168df85b2b172f8db737ba926` |
| external/toybox | `1e6e196a1a7e269019f6a31903c159486097f8c4` |
| art | `814cc9385f8f8eaba6f4bfd1d723160c2132c76e` |
| build/make | `75bb8bac25fbc6ae084304c8b2484d47d0746d2b` |
| device/generic/goldfish | `157a6de4150e70fb8031c89a76bd2335bdab8a35` |

## Findings and dispositions

Paths below are relative to that pinned AOSP checkout.
Line numbers refer to the inspected revisions, not a moving branch.
Historical notebooks remain context; corrected proposals supersede their earlier conclusions.

| Finding | Source evidence and correction |
| --- | --- |
| Full user reports wrongly described as omitting getprop and sensitive sections | `frameworks/native/cmds/dumpstate/dumpstate.cpp:1787` invokes getprop without a build gate. `include_sensitive_info` at `:2065` belongs to telephony-only reports. Rewrote build/report-mode guidance and corresponding system fields/requirements. |
| Userdebug treated as guaranteeing all optional sections | File access, mode, runtime properties and timeouts still control collection. Non-user profile/Dropbox ZIP gates are at `dumpstate.cpp:1944-1961`; system trace `:1091-1108` is added when present on either build type. |
| Privilege-drop failure described as universally silent success | Normal collection returns `RunStatus::ERROR` at `dumpstate.cpp:1999-2000`; telephony/Wi-Fi paths return early. Readable partial output is not a success guarantee. |
| NO ANRs/TOMBSTONES markers treated as proving readable empty directories | `GetDumpFds`, `dumpstate.cpp:395-436`, can return an empty vector after open failures. Markers describe the collector's set; associated diagnostics are needed to exclude inaccessible evidence. |
| Missing top/procrank sources | Top is in `external/toybox/toys/posix/ps.c`; procrank is in `system/memory/libmeminfo/tools/procrank.cpp`, sharing `libsmapinfo/smapinfo.cpp`. Corrected source locations and unavailable-source claims. |
| Top denominator and host column unresolved | `ps.c:1544-1582,1645,1691-1705,1843-1857`: -H selects threads, the header uses configured CPU count, one CPU is 100%, and host maps to the eighth proc/stat field, steal. OEM binary equivalence remains unverified. |
| am_crash emitter claimed missing | `frameworks/base/services/core/java/com/android/server/am/ActivityManagerService.java:9271-9279` calls generated writeAmCrash. Actual first arguments are PID then user ID despite reversed declaration names at EventLogTags.logtags:56. Versioned decoding is source-backed, not observation-only. |
| am_anr emitter claimed missing | `ProcessErrorStateRecord.java:374-375` emits user ID, PID, process name, flags and annotation, matching EventLogTags.logtags:17. Corrected the missing-emitter caveat. |
| Memory tags claimed structurally zero before meminfo collection | `AppProfiler.java:668,808` emits am_meminfo; `:917,983` emits am_pss during profiling. AMS also emits am_meminfo in dumps. Zero retained fixture rows do not establish an emission impossibility or absence cause. |
| am_kill writer claimed unavailable | `ProcessRecord.java:1296-1297` emits user ID, PID, process name, adj, caller reason and RSS. Reasons are open strings; the writer exists. |
| Boot/start_seq pair claimed unique across AMS restarts | `ProcessList.java:463,2156` initializes/increments the counter per AMS instance. Kernel boot time can survive system_server restart. Restricted the proposed pair to a demonstrated AMS lifetime and recorded unresolved cross-restart identity. Removed the invalid boot-plus-relative-age lifetime join. |
| Cpuinfo nice bucket claimed absent from input | `core/java/com/android/internal/os/ProcessCpuTracker.java:126-134,356` reads nice and adds it to user. Only the separately printed nice value is unavailable. Steal is absent from this tracker, but available in top/cpu_monitor. |
| Cumulative and delta cpu_monitor blocks conflated | `services/core/java/com/android/server/cpu/CpuInfoReader.java:324-336,681-695` prints both. Only the labelled cumulative block supports since-boot counters; latestCpuUsageStats contains read deltas after the first read. |
| ART threshold equated with userdebug or app debuggability | `art/runtime/monitor.cc:53-54` uses kIsDebugBuild; `libartbase/base/globals.h:65-69` selects it through NDEBUG. `AndroidRuntime.cpp:758-764,963-965` separately configures profiling. Loaded variant cannot be inferred from ro.build.type. |
| Input ANR reasons claimed unable to carry durations | `frameworks/native/services/inputflinger/dispatcher/InputDispatcher.cpp:6511-6517` formats Waited Nms; `InputManagerCallback.java:350-351` wraps the reason. Observed wait is not a universal configured ANR budget, so no generic timeout field was added. |
| Load value multiplied into purported uptime | Linux loadavg computes an exponentially weighted task count. Withdrew invalid load-derived duration/conflict fields and their illustrative finding. High load soon after boot is not mathematically impossible. |
| Parallel archive order called completion order | `frameworks/native/cmds/dumpstate/DumpPool.cpp:37-48` appends a particular future's temporary file when awaited. Wait order, completion order and execution timing can differ. Line overlap does not prove time overlap. |
| Swap growth and high IRQ assigned a cause from correlation | Retained observations and timing; removed claims proving collector-caused swap growth, host interrupt injection, or absence of host descheduling from zero steal. |
| Per-UID CPU accounting claimed nonexistent | `KernelCpuUidTimeReader.java:196-208` reads uid_cputime; `core/java/android/os/BatteryStats.java:6688-6698` prints UID CPU totals. These are different accounting intervals; omission from the draft is scope, not lack of platform support. |
| lshal R interpreted as registered | `frameworks/native/cmds/lshal/TableEntry.cpp:63,108-119` maps R to release status from the interface hash. Replaced registered with released/interface_hash in the proposal and example. |
| Missing SharedUser printer | `Settings.java:5766-5773` prints the name, identity hash and appId. Resolved the missing-printer claim. |
| 64-file limit used to explain 126-day-old ANRs | `StackTracesDumpHelper.java:516-521` checks both count and one-day age. i > max can retain 65 recent files; it cannot explain old-file retention. Pruning history remains unknown. |
| Envelope and units inconsistent | The enumerated common envelope has 14 keys, not 13. Corrected count, removed the required-but-removed title row, reconciled empty negative-evidence/corroboration rules, renamed binary memory field suffixes to kib/mib and corrected fractional/above-100 CPU percentage guidance. Actual AOSP function names retain their original spelling. |

Linux cross-checks used primary sources: the [v6.6 load-average implementation](https://github.com/torvalds/linux/blob/v6.6/kernel/sched/loadavg.c) and [proc/stat counter documentation](https://docs.kernel.org/filesystems/proc.html#miscellaneous-kernel-statistics-in-proc-stat). They support the interpretation limits; this is not a verification of the reference device's kernel configuration or hypervisor accounting.
Guest counters must not be added again to user/nice when deriving executed CPU share; the draft formula now states its accounting policy explicitly.

## Reference fixture remeasurement

The local private fixture was read without extraction or modification.
ZIP names, device identifiers, log payloads and capture dates are omitted here.
SHA-256 matched the expected fixture: `10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92`.
Python zipfile and byte/line counts produced:

| Property | Actual |
| --- | --- |
| Stored ZIP size | 14,710,126 bytes |
| Entries | 398 |
| Main text size | 64,645,191 bytes |
| Main text newline count | 682,951 |
| Main text NUL bytes / affected split lines | 4 / 1 |
| All-entry NUL bytes, including binary entries | 142,123; this is not a text-corruption count |
| FATAL EXCEPTION raw lines | 274 |
| First SYSTEM LOG, lines 9116–53709 | 38 |
| Broader DUMPSYS block, lines 365371–617121 | 3 |
| Trailing SYSTEM LOG, lines 653373–682951 | 233 |
| Explicit DROPBOX SYSTEM APP CRASHES, lines 652155–652449 | 0 |
| Retained am_meminfo / am_pss main-text rows | 0 / 0 |

The record preserves the historical 271 distinct-crash interpretation, with a warning that PID/second alone is a fixture shortcut, not a universally safe de-duplication key.
Fresh checks cover raw region counts, not every crash reconstruction.
Archive-wide line metrics/boot uncertainty were not exhaustively rederived.
Against the documented boot bracket and the observed +07:00 offset, ro.runtime.firstboot differs by about 19.1 seconds, not 6.5 seconds.
The redacted same-day age table also had arithmetic errors: RT_TABLES is 7 seconds before the banner, BINDER is 25 seconds before, and VM TRACES JUST NOW is 8 seconds after collection began.

## Verification and remaining limits

- Lima source inspection and installed-output checks: **passed**, with the baseline/limitations above.
- Hash-matched fixture aggregate and region counts: **passed**; input identity rechecked after review.
- `python3 scripts/check_docs.py`: **passed**, 144 Markdown pages and 1,601 local links, zero errors; includes page bounds, anchors and presentation checks.
- `python3 scripts/diagrams.py check`: **passed**, all nine SVGs fresh, links and notation valid.
  No puml/include/SVG was edited by this review.
- `git diff --check`: **passed**, including the pre-existing working changes.
- Private fixture SHA-256 after review: **unchanged**, matching the recorded expected digest.
- Cargo suites and AOSP builds: **not run**.
  This is documentation-only work; existing runtime discrepancies need their own behavior decisions and SWE.6 verification.

Remaining design work:

- Resolve process identity across AMS restarts, missing starts and indistinguishable snapshots.
- Reconcile capture_ref's string/object representations, per-artifact extension shapes and required leaf evidence against abbreviated examples before implementing a validator.
- Define the promised artifact-specific severity_rule tables and align every example with them.
- Separate window span/message completeness; calculate actual intersections instead of treating duration ratios as message-retention proof.
- Revisit uptime-based boot derivations across suspend: uptimeMillis excludes deep sleep while elapsedRealtime includes it (`android/os/SystemClock.java:58-75`).
  Wall time minus active uptime is not a universally valid kernel boot instant.
  This fixture's narrow cold-boot inference does not establish a general method.
- Confirm the OEM build/source match, device property overrides, command outcomes and any optional HAL/raw-file output before applying upstream assumptions to a different archive.

The existing [committed requirement discrepancies](../../docs/decisions/index.md) and SWE.6-T1 conformance work remain open.
Historical records/notebooks imply no new passing executions.
