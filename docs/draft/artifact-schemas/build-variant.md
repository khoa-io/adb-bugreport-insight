# Build-variant behaviour

> **Draft.** See [`index.md`](./index.md).

Original §3 of `artifact-schemas.md`. Read this if the target device may not be a user build —
several fields in [`conventions-envelope.md`](./conventions-envelope.md) and
[`artifact-system.md`](./artifact-system.md) are gated by what this file defines.

## 3. Build-Variant Behaviour

This section is normative. A schema must state what a consumer sees on a `user` build, and
the answer is usually **not** "the same with some zeros".

### 3.1 `IsUserBuild()` is the single primitive

Every build-type gate in `dumpstate` bottoms out in one function,
`frameworks/native/cmds/dumpstate/DumpstateUtil.cpp:212-217`:

```cpp
bool PropertiesHelper::IsUserBuild() {
    if (build_type_.empty()) {
        build_type_ = android::base::GetProperty("ro.build.type", "user");   // :214
    }
    return "user" == build_type_;
}
```

Three consequences that every schema must respect:

1. **It is a literal string compare against `"user"`.** `userdebug` and `eng` are both "not a
   user build" and both take the *same inclusive* branch. **There is no `userdebug`-vs-`eng`
   distinction anywhere in `dumpstate.cpp`.** A schema that claims `userdebug` sees something
   `eng` does not is false for dumpstate sections.
2. **The default is `"user"`.** If the property read fails, `dumpstate` behaves as a *user*
   build and drops the sensitive sections. A permissions or SELinux problem at property-read
   time **silently degrades a debug bugreport to user-level**, with no error marker in the
   archive.
3. **`ro.debuggable` and `ro.secure` are never consulted** by `dumpstate.cpp`,
   `dumpstate.h`, or `DumpstateUtil.cpp`. A schema that gates on `ro.debuggable` is gating on
   something dumpstate does not use. The source-backed build-type signal is `ro.build.type`,
   and it *is* printed — in the `Build:` header line, archive line 5.

The reference archive is a `userdebug` build:

```
Build: sdk_gcar_arm64-userdebug 15 AAG6.260127.001 14784989 dev-keys
Build fingerprint: 'google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys'
```

so `IsUserBuild()` is false and every gated section is present.

### 3.2 Gated sections

Three outcomes must be modelled separately, because conflating them is the error:

- **absent** — no section header is emitted at all.
- **present-but-empty** — header emitted, zero payload.
- **present-and-different** — header emitted with different content.

| # | Section / content | Gate (cited) | On a `user` build | Schema consequence |
| --- | --- | --- | --- | --- |
| 1 | **`SYSTEM PROPERTIES`** (`getprop`) | `dumpstate.cpp:2065` `include_sensitive_info = !IsUserBuild()`, used at `:2090-2091` | **absent** | Every `getprop`-only field becomes `null`, never `false`. The source comment at `:2085-2089` says it holds PII. |
| 2 | **`DUMPSYS netd`** | `dumpstate.cpp:2124`, inside `if (include_sensitive_info)` | **absent** | Raw IP addresses; present in the reference archive (1251 hits), proof `userdebug` took the inclusive branch. |
| 3 | **`DUMPSYS connmetrics`** | `dumpstate.cpp:2127` | **absent** | Raw destination IP/MAC addresses. Present in the reference archive (6 hits). |
| 4 | **`BATTERYSTATS`** | `dumpstate.cpp:2130` | **absent** | Package/component names. Present in the reference archive. |
| 5 | **`BATTERYSTATS deviceidle`** | `dumpstate.cpp:2133` | **absent** | Present in the reference archive (14 hits). |
| 6 | **`APP SERVICES NON-PLATFORM`** | `dumpstate.cpp:2146` | **absent** | Present in the reference archive. |
| 7 | **`CHECKIN BATTERYSTATS`** | `dumpstate.cpp:2157` | **absent** | Present in the reference archive. |
| 8 | **`carrier_config`** dumpsys | `dumpstate.cpp:2100-2111` | **present-and-different** | On `user` builds it receives `--requesting-package <pkg>` instead of a full dump. The section is **narrower, not absent**. Easy to mis-model. |
| 9 | **`DUMP VENDOR RIL LOGS`** (`vril-dump`) | `dumpstate.cpp:1831`, `options.AsRoot()` only `if (!IsUserBuild())` | **present-and-different** | Source comment at `:1826`: *"su does not exist on user builds, so try running without it."* Runs unprivileged. |
| 10 | **`PROFILE DATA CUR` / `PROFILE DATA REF`** | `dumpstate.cpp:1944`, `if (!IsUserBuild()) ds.AddDir(...)` | **absent** | Verified 0 hits in the reference `userdebug` archive — correctly absent, since the gate is not taken on `user`. |
| 11 | **`package-dex-use-list`** ZIP entry | `dumpstate.cpp:1947` | **absent** | ZIP-only, never in the main `.txt` on `user` builds. Verified 0 hits. |
| 12 | **Dropbox entry files in ZIP** | `dumpstate.cpp:1957`, `dump_files("", DROPBOX_DIR, …)` | **absent** | **Only the ZIP entries are gated.** The `DROPBOX SYSTEM SERVER CRASHES` / `DROPBOX SYSTEM APP CRASHES` *dumpsys* sections (`dumpstate.cpp:1858-1859`, and a second identical pair at `:1573-1574`) are **ungated** and always present. Both are "dropbox"; they must be distinguished. |
| 13 | **UI traces** | `dumpstate.cpp:3559`, `if (IsUserBuild() \|\| use_predumped_ui_data) return;` | **absent** | — |
| 14 | **`WMTRACE_DATA_DIR`** in ZIP | `dumpstate.cpp:3591`, `if (IsUserBuild()) return;` | **absent** | Companion to #13. |
| 15 | **System trace snapshot in ZIP** | `dumpstate.cpp:1098` — not a gate, only changes the **log message** | **present-and-different** | On `user` builds it returns early; otherwise it also returns early if no trace exists, but with an `MYLOGI` pointing at `go/trace-uuid` in logcat. |

Separately, one **permission**-gated source that is not build-typed: the whole `package`
dumpsys section is suppressed unless the caller holds both `DUMP` and `PACKAGE_USAGE_STATS`
(`PackageManagerService.java:6601` → `DumpUtils.checkDumpAndUsageStatsPermission`,
`frameworks/base/core/java/com/android/internal/util/DumpUtils.java:176-179`). If `apps.json`
has zero packages, that is the first thing to check — not a parsing failure.

**Two consequences a parser must respect:**

1. **Never infer `build.type` from the `getprop` section.** If `getprop` is missing, read the
   type out of the `Build:` header — it is present on every build type, because
   `PrintHeader()` reads properties directly via `GetProperty` (`dumpstate.cpp:795-818`),
   unaffected by the gate.
2. **A missing section is not an empty section.** `DumpstateUtil.cpp:299-302` prints
   `Skipping '<cmd>' on user build.` **in place of** the banner. Anything keyed on
   `------ ` sees neither a banner nor a body and will conclude "absent". The correct status
   is `skipped_user_build`. This is why `sections[].status` is an enum and not a boolean.

### 3.3 Failure modes that leave a well-formed archive

A truncated or silently degraded bugreport still parses cleanly. These must be detectable.

| # | Gate | Cited | Failure mode |
| --- | --- | --- | --- |
| 16 | `DropRootUser()` | `DumpstateInternal.cpp:49-80` | Resolves `shell` uid/gid, then `prctl(PR_SET_KEEPCAPS, 1)`, then sets supplementary groups for `log, sdcard_r, sdcard_rw, mount, inet, net_bw_stats, readproc, bluetooth, wakelock, nfc`. Any `getgrnam` returning 0 → `return false`. |
| 17 | `if (!DropRootUser()) return;` — early return from the whole dump | `dumpstate.cpp:1999`, `:2068`, `:2179` | **This is the one that silently truncates a bugreport.** `DumpstateRadioCommon` runs, then the function returns and the rest is never produced. The archive still looks well-formed. A consumer must detect the truncation, not assume the section list is complete. |
| 18 | Per-command `DROP_ROOT` failure | `DumpstateUtil.cpp:341-348` | Writes `*** could not drop root before running %s: %s` then `_exit(EXIT_FAILURE)`. The section is **present-but-empty**. |
| 19 | `su` availability for root-required commands | e.g. `dumpstate.cpp:1832`; `dumpstate_log.txt` shows `*** command '/system/xbin/su root sh -c cat /proc/modules …` | Emits an error line into the section. |
| 20 | Command timeout kill | `DumpstateUtil.cpp:403-408` | `*** command '%s' timed out after %.3fs (killing pid %d)`. Section is **present-but-truncated**: header and partial payload both present. |

Time budgets also change output shape:

| Gate | Value | Cited | Effect |
| --- | --- | --- | --- |
| `IsStrictRun()` | `dumpstate.strict_run` property, **default `true`** | `DumpstateUtil.cpp:239-244` | When true, tight timeouts are used instead of generous ones. |
| Incident dump timeout | `IsStrictRun() ? 20 : 120` **seconds** | `dumpstate.cpp:1058` | A 6× difference in how much of the incident dump you get. |
| Per-dumpsys timeouts | e.g. 90 s hard, 10 s soft | `dumpstate.cpp:2095-2135` | On expiry the native dumpsys prints `*** SERVICE 'X' DUMP TIMEOUT (Nms) EXPIRED ***` (`dumpsys.cpp:290`). |
| `dumpsys meminfo` timeout | **60 s**, escalated from `service_timeout` | `dumpstate.cpp:1285-1287` | AOSP gives this one service a 60 s budget, *"since 30s is not always enough"* — the platform itself admitting the scan is expensive. |

File-existence checks produce **present-but-empty**, not absent
(`DumpstateUtil.cpp:246-260` writes the header then `*** Error dumping %s: %s`):

| Path | Section | Cited | If missing |
| --- | --- | --- | --- |
| `/proc/pressure/{cpu,memory,io}` | `PSI cpu` / `PSI memory` / `PSI io` | `dumpstate.cpp:1979-1981` | present-but-empty + `*** Error dumping …: No such file or directory` |
| `/data/tombstones/` | `TOMBSTONE` | `dumpstate.cpp:1932`, `:1755-1760` | **Explicit marker:** `*** NO TOMBSTONES to dump in /data/tombstones/` |
| `/data/anr/` (`anr_` prefix) | `VM TRACES AT LAST ANR` + `HISTORICAL ANR` | `dumpstate.cpp:1182-1188` | **Explicit marker:** `*** NO ANRs to dump in /data/anr/` |
| `/data/system/dropbox` | dropbox files | `dumpstate.cpp:1957-1960` | silently nothing, no marker |
| `/data/system/shutdown-checkpoints/` | shutdown checkpoints | `dumpstate.cpp:1935-1936` | nothing |

**The two `*** NO …` markers are the single most useful signal for a schema.** They let a
consumer distinguish *absent because none exist* from *absent because permission denied*.
Verified on the reference archive: both markers are 0 hits, i.e. both sections are populated,
and `PSI cpu` is present with `some avg10=65.00 avg60=25.63 avg300=6.26 total=19879787`.

### 3.4 dumpsys sections are an open set

**No schema may hardcode a section allowlist.** The set of dumpsys sections in a bugreport is
a runtime property of the device, not a compile-time constant.

`DumpState.java` **does not exist in Android 15**. Verified: no match under
`frameworks/base/core/java/android/os`, and the only two hits in `frameworks/base` are
unrelated (`server/om/DumpState.java`, `server/pm/DumpState.java`). `BinderService.java`, the
class that used to host the registry, is gone from `frameworks/base` entirely.

In Android 15 a dumpsys section name is a **binder service name**, registered at runtime.
The registry is the set of `ServiceManager.addService(...)` calls in
`ActivityManagerService.setSystemProcess()` (`…/am/ActivityManagerService.java:1926-1937`):

```java
ServiceManager.addService(Context.ACTIVITY_SERVICE, this, ..., DUMP_FLAG_PRIORITY_CRITICAL | …);
ServiceManager.addService(ProcessStats.SERVICE_NAME, mProcessStats);                      // :1928
ServiceManager.addService("meminfo", new MemBinder(this), ..., DUMP_FLAG_PRIORITY_HIGH);   // :1929
ServiceManager.addService("gfxinfo", new GraphicsBinder(this));                           // :1930
ServiceManager.addService("dbinfo", new DbBinder(this));                                  // :1931
mAppProfiler.setCpuInfoService();                                                        // :1932 → "cpuinfo"
```

The native `dumpsys` then resolves the name by asking the service manager —
`frameworks/native/cmds/dumpsys/dumpsys.cpp:261`, `services = listServices(priorityFlags, asProto)`
— and with no arguments it **enumerates everything currently registered**
(`dumpsys.cpp:250-274`).

So sections are modelled as an **open set** keyed by the observed name, with "absent from
this device" as a valid state. The reference archive contains roughly **250 distinct
`dumpsys <name>` lines**, including per-hardware-module AIDL interfaces such as
`android.hardware.gnss.IGnss/default` and `android.security.compat`, plus
`media.extractor` / `media.metrics` / `media.swcodec` which resolve to *services*, not
processes.

Two further facts that constrain naming:

- `DumpHals()` (`dumpstate.cpp:1421-1470`) enumerates
  `android::hardware::defaultServiceManager()` — the **HIDL** hwservicemanager only. AIDL
  HALs are invisible there even when present. A small `lshal-debug/` (3 entries on this
  device) therefore means "few HIDL HALs", **not** "few HALs". `system.json#/hal` carries
  `covers_aidl: false` for exactly this reason.
- `DumpState.java` is also not where `procstats` lives: that registration is
  `ProcessStats.SERVICE_NAME` at `ActivityManagerService.java:1928`, with the literal name at
  `ProcessStatsService.java:373`. `am/ProcessStats.java` no longer exists in `am/` — it is
  `am/ProcessStatsService.java`. Any citation of `am/ProcessStats.java` is stale.

**Two modules are defined but not built for this product**, verified against the build
output rather than the `.bp` files:

| Module | Defined at | Built? |
| --- | --- | --- |
| `anrd` | `system/extras/ANRdaemon/Android.bp:17` | **No** — `find` over the whole product tree returns nothing. ANR traces come from AMS inside `services`. |
| `carwatchdogd` | `packages/services/Car/cpp/watchdog/server/Android.bp:309` | **No** — also absent, despite this being an automotive (`sdk_gcar_arm64`) build. |

This matters for triage: it is tempting to attribute car-watchdog output to
`carwatchdogd`, but that process is not in this image, and `anrd` is not an ANR detector
either — it is a CPU-starvation ftrace collector
(`system/extras/ANRdaemon/README:1-14`, `ANRdaemon.cpp:236-260`). It declares no ANR, writes
no `/data/anr/` entry, and contributes **no field** to `anr.json`.

---
