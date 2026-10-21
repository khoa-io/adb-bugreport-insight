# Build variants, report modes and section availability

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Source baseline: `android-15.0.0_r17`.
> [Fact-check record](../../../verification/results/aosp-source-verification.md).

## 3. Build-Variant Behaviour

Section availability depends on report mode, build type, permissions, runtime configuration, file availability and command success.
Build type alone cannot predict archive contents.
Separate evidence sources: automotive `userdebug` reference archive and Lima phone-emulator `eng` output.

<a id="31-isuserbuild-is-the-single-primitive"></a>

### 3.1 `IsUserBuild()` is the build-type primitive

`frameworks/native/cmds/dumpstate/DumpstateUtil.cpp:212-217` reads `ro.build.type`, defaults to `user`, and compares literally with `user`.
`userdebug` and `eng` take the same branch at these call sites.
Inspected `dumpstate.cpp`, `dumpstate.h` and `DumpstateUtil.cpp` read neither `ro.debuggable` nor `ro.secure`; downstream/ART/vendor behavior can still differ by variant.

Read build type from the fingerprint's type segment or an observed property.
`PrintHeader()` (`dumpstate.cpp:795-818`) prints the display ID and fingerprint independently of `getprop`.
An arbitrary OEM display ID need not encode the type in a parseable form.

The helper's default does not prove that a missing section was build-gated.
Assign `skipped_user_build` only when a skip marker or a verified mode/build-specific rule supports it.

### 3.2 Gated sections

Report-mode gates are evaluated separately from the normal full-report path.
**Full reports invoke `getprop` without an `IsUserBuild()` gate** (`dumpstate.cpp:1787`).
The `include_sensitive_info` predicate at `:2065` belongs to telephony-only collection.
Properties may be absent/unreadable/filtered; `user` does not automatically imply `null`.

| Section / content | Scope and source | Behaviour |
| --- | --- | --- |
| `SYSTEM PROPERTIES` | Full report `dumpstate.cpp:1787`; telephony-only `:2086-2092` | Requested on full reports of every build type; omitted on telephony-only `user` reports |
| `DUMPSYS netd`, `DUMPSYS connmetrics`, `BATTERYSTATS`, deviceidle, non-platform app services and checkin batterystats | Telephony-only `:2123-2158` | Sensitive blocks omitted in telephony-only `user` mode; these are not universal full-report gates |
| `carrier_config` | Telephony-only `:2100-2111` | `user` receives `--requesting-package <pkg>`; other builds receive the full request |
| Vendor RIL logs | `:1826-1835` | Root requested only on non-user builds; unprivileged invocation still attempted on user builds |
| Profile directories and package dex-use file | `:1944-1948` | Copied into ZIP only on non-user builds, subject to file access; a false user-build predicate does not guarantee files exist |
| Dropbox entry files | `:1957-1961` | ZIP files copied only on non-user builds, excluding WTF/StrictMode; text crash dumps at `:1858-1859` have a separate collection path |
| UI traces | `:3559`, `:3591` | User builds return early; predumped UI data is another gate for the first path |
| System trace snapshot | `:1091-1108` | Existing snapshot added on either build type; `IsUserBuild()` changes only the message when no snapshot exists |
| Explicit root-required command | `DumpstateUtil.cpp:299-302` | User builds print a skip message before the usual command banner |

`getprop`-derived fields become `null` when the required value is unavailable in the actual archive.
A full `user` report can supply those values.
`userdebug` permits non-user branches without proving every optional section was collected.

Package dumps also check permissions (`PackageManagerService.java:6601` and `core/java/com/android/internal/util/DumpUtils.java:176-179`).
Missing/empty package dumps prove neither parser failure nor build type.

### 3.3 Failure modes that leave partial evidence

| Condition | Source | Observable consequence and limit |
| --- | --- | --- |
| Cannot drop root | `DumpstateInternal.cpp:49-107` | Returns false after failed identity/capability setup; logs a diagnostic |
| Full-report privilege-drop failure | `dumpstate.cpp:1999-2000` | Returns `RunStatus::ERROR`; callers must not treat a readable partial ZIP as successful collection |
| Telephony/Wi-Fi privilege-drop failure | `:2068-2069`, `:2179-2180` | Returns early from the mode-specific void function; inspect logs and report mode |
| Per-command privilege-drop failure | `DumpstateUtil.cpp:341-348` | Child prints an error and exits; a banner does not imply successful payload |
| Command timeout | `DumpstateUtil.cpp:403-408` | Timeout diagnostic and possibly partial payload |
| Strict timeout policy | `DumpstateUtil.cpp:239-245`; `dumpstate.cpp:1058` | `dumpstate.strict_run` defaults true; incident budget is 20 rather than 120 seconds |
| meminfo service budget | `dumpstate.cpp:1285-1287` | 60 seconds for this service; timeout must remain distinct from an empty successful dump |
| Missing proc/sys file | `DumpstateUtil.cpp:246-260` | Header plus error diagnostic; failed reading is not a successful empty measurement |

`*** NO ANRs to dump in /data/anr/` and `*** NO TOMBSTONES to dump in /data/tombstones/` report an empty collector input set (`dumpstate.cpp:1182-1189`, `:1755-1760`).
`GetDumpFds()` (`:395-436`) can also produce an empty set after directory/open failures.
These markers alone do **not** distinguish an empty directory from inaccessible evidence; inspect associated diagnostics.
Missing dropbox/shutdown-checkpoint files can produce no payload.

Default runtime properties are `dry_run=false`, `unroot=false`, `parallel_run=true`, and `strict_run=true` (`DumpstateUtil.cpp:219-245`).
Their values on the reference device are not established by inspecting the Lima checkout.
Mode, cancellation, timeouts and failures must remain separate from intentional build suppression in the section-status model.

### 3.4 dumpsys sections are an open set

Native dumpsys enumerates services at runtime (`frameworks/native/cmds/dumpsys/dumpsys.cpp:250-274`).
`ActivityManagerService.setSystemProcess()` (`services/core/java/com/android/server/am/ActivityManagerService.java:1926-1937`) registers several services, including `activity`, `procstats`, `meminfo` and `gfxinfo`; **it is one registration site, not the whole service registry**.
Other services register elsewhere.

There is no central `android.os.DumpState` registry in this tree.
Classes named `DumpState` exist in package-manager and overlay-manager code for unrelated dump bookkeeping.
`ProcessStatsService` is under `services/core/java/com/android/server/am`; the data model `ProcessStats` still exists under `core/java/com/android/internal/app/procstats`.

Model section names as an open set.
The reference archive contains roughly 250 distinct `dumpsys <name>` lines, including AIDL HAL names.
`DumpHals()` (`dumpstate.cpp:1421-1470`) uses the HIDL hwservicemanager; its `lshal-debug/` entries do not enumerate AIDL HALs.

### 3.5 Build output is product-specific evidence

The inspected output is `out/target/product/emu64a` for `sdk_phone64_arm64`: `device/generic/goldfish/64bitonly/product/sdk_phone64_arm64.mk:37-38` defines `PRODUCT_NAME=sdk_phone64_arm64` and `PRODUCT_DEVICE=emu64a`.
The installed system build properties identify Android 15, SDK 35, build `AP4A.250205.002.C1`, variant `eng`.
This is a phone emulator product, not the automotive product that produced the private fixture.

`top`, `procrank` and `bugreport_procdump` exist in that output.
`anrd` and `carwatchdogd` were not found in the inspected installed output; that says nothing about installation or running processes on the reference device.
Source presence does not prove installation; installation does not prove execution at capture.

`anrd` is a CPU-starvation ftrace collector (`system/extras/ANRdaemon/README:1-14`, `ANRdaemon.cpp:236-260`), rather than the framework's ANR detector.
AMS and its collaborators produce framework ANR records and traces.
