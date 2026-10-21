# AOSP source provenance

## Source baseline

Source locators below refer to Android 15 `android-15.0.0_r17` research and the [source verification](../../../verification/results/aosp-source-verification.md).
They identify where evidence originates, not proof that an OEM device ships identical code or binaries.
The inspected `sdk_phone64_arm64` / `emu64a` eng output differs from the reference automotive SDK build.
The [source/runtime evidence](../../../verification/results/agent-led-aosp-crash-investigation.md) identifies the inspected environment and applicability limits.
Establish the source revision and target runtime before extending those claims.
The [schemas](index.md), [generation rules](12-generation-rules.md) and verified source semantics define the supported interpretation.
This lookup adds no new required artifact fields or runtime behavior.

## Modules and installed names

Paths and line numbers are revision-specific; installed presence depends on product configuration.
`bugreportz` relays dumpstate progress and completion; dumpstate writes the ZIP, and host adb pulls it.

| Module | Source directory | Defining file | Artifact or installation note |
| --- | --- | --- | --- |
| `logcat` | `system/logging/logcat` | `system/logging/logcat/Android.bp:34` | `system/bin/logcat` |
| `logd` | `system/logging/logd` | `system/logging/logd/Android.bp:90` | `system/bin/logd` |
| `liblogd` | `system/logging/logd` | `system/logging/logd/Android.bp:58` | static lib (no standalone binary) |
| `liblog` | `system/logging/liblog` | `system/logging/liblog/Android.bp:153` | `liblog.so` |
| `bugreportz` | `frameworks/native/cmds/bugreportz` | `frameworks/native/cmds/bugreportz/Android.bp:13` | `system/bin/bugreportz` |
| `bugreport` | `frameworks/native/cmds/bugreport` | `frameworks/native/cmds/bugreport/Android.bp:11` | `system/bin/bugreport` |
| `dumpstate` | `frameworks/native/cmds/dumpstate` | `frameworks/native/cmds/dumpstate/Android.bp:122` | `system/bin/dumpstate` |
| `libdumpstateutil` | `frameworks/native/cmds/dumpstate` | `frameworks/native/cmds/dumpstate/Android.bp:37` | static lib |
| `dumpsys` | `frameworks/native/cmds/dumpsys` | `frameworks/native/cmds/dumpsys/Android.bp:60` | `system/bin/dumpsys` |
| `debuggerd` | `system/core/debuggerd` | `system/core/debuggerd/Android.bp:495` | `system/bin/debuggerd` |
| `tombstoned` | `system/core/debuggerd/tombstoned` | `system/core/debuggerd/Android.bp:534` | `system/bin/tombstoned` |
| `crash_dump` | `system/core/debuggerd` | `system/core/debuggerd/Android.bp:436` | **`apex/com.android.runtime/bin/crash_dump64`** |
| `anrd` | `system/extras/ANRdaemon` | `system/extras/ANRdaemon/Android.bp:17` | Optional; product presence varies |
| `services` | `frameworks/base/services` | `frameworks/base/services/Android.bp:202` | `system/framework/services.jar` → `system_server` |
| `lmkd` | `system/memory/lmkd` | `system/memory/lmkd/Android.bp:39` | `system/bin/lmkd` |
| `memtrack` | `system/extras/memtrack` | `system/extras/memtrack/Android.bp:51` | `system/bin/memtrack` |
| `libmeminfo` | `system/memory/libmeminfo` | `system/memory/libmeminfo/Android.bp:45` | `libmeminfo.so` |
| `libunwindstack` | `system/unwinding/libunwindstack` | `system/unwinding/libunwindstack/Android.bp:143` | `libunwindstack.so` |
| `dumpeventlog` | `sdk/dumpeventlog` | `sdk/dumpeventlog/Android.bp:22` | `system/bin/dumpeventlog` |
| `carwatchdogd` | `packages/services/Car/cpp/watchdog/server` | `.../server/Android.bp:309` | Optional; product presence varies |
| `libadbd` | `packages/modules/adb` | `packages/modules/adb/Android.bp:703` | `apex/com.android.adbd/bin/adbd` |
| `adbd` | `packages/modules/adb` | `packages/modules/adb/Android.bp` | `apex/com.android.adbd/bin/adbd` |
| `simpleperf` | `system/extras/simpleperf` | `system/extras/simpleperf/Android.bp:321` | `system/bin/simpleperf` |
| `atrace` | `frameworks/native/cmds/atrace` | `frameworks/native/cmds/atrace/Android.bp:21` | `system/bin/atrace` |
| `perfetto` | `external/perfetto` | `external/perfetto/Android.bp:1156` | `system/bin/perfetto` (+ traced/traced_probes) |
| `libziparchive` | `system/libziparchive` | `system/libziparchive/Android.bp:151` | `libziparchive.so` |
| `libzip` | `external/rust/android-crates-io/crates/zip` | `external/rust/android-crates-io/crates/zip/Android.bp:17` | Rust crate / rlib-std |

## Domain evidence sources

Relative Java names below resolve under `frameworks/base/services/core/java/com/android/server/` unless another root is given.
Record the source revision and exact archive locator with each investigation; these channels are not a fixed dumpsys allowlist.

| Domain/channel | Producer or source | Interpretation boundary |
| --- | --- | --- |
| ANR declaration | `am/ProcessErrorStateRecord.java:374,445-461,563` | A dated declaration supports an incident; retained trace files alone do not. |
| ANR file inventory/newest trace | `frameworks/native/cmds/dumpstate/dumpstate.cpp:395-436,1184-1204`; `system/core/debuggerd/libdebuggerd/backtrace.cpp:49` | Inventory mtime, section timestamps and per-process clocks remain distinct. |
| Last ANR state | `wm/ActivityTaskManagerService.java:4254-4261,5671` | Snapshot or explicit since-boot negative assertion; not a timestamped stream. |
| ANR Dropbox/event log | `am/ActivityManagerService.java:9714-9722,9766`; `am/EventLogTags.logtags:17` | Rate limits, gates and ring retention limit coverage. |
| Java crash log | `frameworks/base/core/java/com/android/internal/os/RuntimeInit.java:84-86` | Ring-buffer evidence; account for inlined Dropbox duplicates. |
| Crash event log | `am/ActivityManagerService.java:9271-9279`; `am/EventLogTags.logtags:56` | Generated emitter arguments, not tag-name assumptions, define PID/user order. |
| Native tombstones | `system/core/debuggerd/tombstoned`; `system/core/debuggerd/tombstone_proto.cpp` | Retained files can predate capture; use embedded clocks and grouping signatures. |
| Dropbox crash copies | `am/ActivityManagerService.java:9766,9890-9900`; `am/DropboxRateLimiter.java:39` | Rate-limited corroboration and denormalized copies are not independent incidents. |
| Process death / manager / RescueParty | `am/EventLogTags.logtags:22,25`; `am/AppErrors.java:902`; `RescueParty.java:822` | Lifecycle/mitigation records are corroboration, not crash declarations. |
| Package inventory | `pm/Settings.java:5136,5471-5509,5766-5773` | Package identity, per-user install state and shared-user records are different keys. |
| Process inventory and identity | `am/ProcessRecord.java`; `am/ProcessList.java:463,2156` | Start sequence is scoped to an AMS lifetime, not globally unique across system_server restarts. |
| Usage stats | `usage/UsageStatsService.java:2038-2076` | Preserve enclosing user ID; locked users can have pending events. |
| Device facts and HAL inventory | `frameworks/native/cmds/dumpstate/dumpstate.cpp:795-830,1421-1470` | Header facts, optional properties and HIDL debug listings have separate availability; HIDL absence does not establish AIDL absence. |
| Memory summary | `am/ActivityManagerService.java:12699-12819`; `frameworks/base/core/jni/android_os_Debug.cpp:383-393` | Printed summary, components and derived PSS require distinct units/derivations. |
| Raw memory, vmstat, zones/slabs | `frameworks/native/cmds/dumpstate/dumpstate.cpp:1689,1698,1700-1704,1951` | Point readings and since-boot counters remain separate. |
| Procstats / checkin | `frameworks/base/core/java/com/android/internal/app/procstats/ProcessStats.java:1632-1637,2143-2166`; `SysMemUsageTable.java:142-171` | Each persisted aggregation subsection retains its own time window. |
| Procrank / bugreport_procdump | `system/memory/libmeminfo/tools/procrank.cpp`; `system/extras/libsmapinfo/smapinfo.cpp:194-212,247-254,287-300,1233-1242` | PSS/RSS and swap columns differ; ZSwap is an estimate. |
| ZRAM physical memory | `system/memory/libmeminfo/sysmeminfo.cpp:110-206` | Standard summaries expose physical usage, not every raw mm_stat field. |
| ZRAM IO / PSI | `frameworks/native/cmds/dumpstate/dumpstate.cpp:1230-1244,1688,1979-1981` | Block counters are cumulative; PSI averages and total microseconds have separate units. |
| lmkd logs / killinfo / tracepoint | `system/memory/lmkd/lmkd.cpp:2136-2180,2448-2488`; `event.logtags:39`; `statslog.h:57-70` | Kill text, structured event payload and live trace are separate evidence channels. |
| AMS memory events | `am/EventLogTags.logtags:31,34,67,69` | Event collection can precede the dump that emits a counter; zero retained events need a coverage/ordering explanation. |
| IOTOP / storaged | `system/extras/iotop/iotop.cpp:36-39,104,226-273`; `frameworks/native/cmds/dumpstate/dumpstate.cpp:1801,1973` | Sampling intervals, resolution floors and cumulative process-start totals must not be merged. |
| Background CPU samples | `am/AppProfiler.java:142,2156-2160,2285-2358`; `frameworks/base/core/java/com/android/internal/os/ProcessCpuTracker.java:854-914` | Dumping prints existing samples; use the printed window rather than capture time. |
| CPU aggregate/per-process counters | `ProcessCpuTracker.java:57-144,353-356,424,650` | The aggregate read excludes steal and folds nice into user; process deltas share its sample window. |
| CPU per-core samples | `cpu/CpuInfoReader.java:63-75,324-336,681-724,859-880`; `cpu/CpuMonitorService.java:205` | The cumulative block and latest-read deltas are distinct; guest counters must not be double-counted. |
| Toybox top | `external/toybox/toys/posix/ps.c:1645,1691` | Configured CPU capacity is not online count; top labels steal as host. |
| Per-UID CPU | `frameworks/base/core/java/com/android/internal/os/KernelCpuUidTimeReader.java:196-208`; `android/os/BatteryStats.java:6688-6698` | Accounting exists, but battery/UID windows differ from cpuinfo; not an absence claim. |
| ART monitor waits | `art/runtime/monitor.cc:53-54,402-420,458,592-598`; `art/libartbase/base/time_utils.cc:53-75` | Loaded ART compile-time debug flags determine thresholds; retain original duration units. |
| Sampled ART waits | `art/runtime/monitor.cc:529-536`; `art/runtime/monitor_android.cc:35-108`; `art/runtime/runtime.cc:1589` | Probability-sampled records and sample percentages do not establish the total contention count. |
| ART runtime mutex waits | `art/runtime/base/mutex.cc:42,533,567-591` | Rate-limited reporting; zero retained records does not establish enablement or health. |
| Graphics and power history | `frameworks/base/libs/hwui/ProfileData.cpp:114`; `frameworks/native/cmds/dumpstate/dumpstate.cpp:1602,2130,2134` | Per-process frame windows and persisted battery/power intervals are independent of CPU sample windows. |

## Service ownership

AMS registers several activity/memory/graphics/process services; AppProfiler owns cpuinfo, and CpuMonitorService owns cpu_monitor.
Procstats uses `am/ProcessStatsService.java`; the data model remains in `core/java/com/android/internal/app/procstats/ProcessStats.java`.
Package, user, usage, power, window/display and HAL services have their own registration sites.
There is no central `android.os.DumpState.java` section registry; similarly named package/overlay bookkeeping classes are unrelated.
Dumpsys discovers services dynamically, so unresolved vendor/AIDL emitters stay an explicit open set.
Build type alone does not establish section presence: apply [build/report/permission gates](03-build-variants.md).
