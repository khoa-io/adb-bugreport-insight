# `system.json` — the device fact sheet

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Requirement: `REQ-INSIGHT-04`.

Owns `capture`; other artifacts use `capture_ref` without recomputing capture, uptime, log window or boot.
Read [`01-envelope.md`](01-envelope.md) §2.1.3 first.

### 4.2 `system.json`

#### Purpose

`system.json` is the **device fact sheet**: capture time, device and build context for interpreting other artifacts.
It answers *"is this even the device I think it is, and is this window of time trustworthy?"*, not *"what went wrong"*.
It contains **no defect findings/rankings**; those belong to `anr.json`, `crashes.json`, `memory.json`, and `cpu.json`.

#### Top-level keys

The fourteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture` | object | yes | **The canonical capture object.** Every other artifact points here. |
| `build` | object | yes | Platform build identity, including the full build-type gate state. |
| `product` | object | yes | Product/brand identity. **Property-derived**; `null` when the needed values are unavailable in the actual report. |
| `device` | object | yes | Physical/virtual nature, bootconfig excerpt. |
| `kernel` | object | yes | Kernel identity and boot args. |
| `users` | object | yes | Multi-user / profile inventory from `dumpsys user`. |
| `displays` | array | no | Display outputs, from the SurfaceFlinger CRITICAL dump. |
| `hal` | object | yes | HIDL HAL inventory from the `lshal` rows. |
| `storage` | object | yes | Mount topology and what was copied under `FS/`. |
| `scope` | object | yes | Extended: carries the full `sections[]` index with a `status` per section. |
| `archive_entries` | object | yes | Inventory of the side artifacts in the ZIP. |
| `evidence_gaps` | array | yes | Facts that could not be read. **Required in every artifact**, and non-empty is not a defect: an artifact that read everything has nothing to declare. See the shape below. |

#### Field tables

##### `capture`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `captured_at` | string \| null | yes | `== dumpstate:` banner, RFC 3339 with explicit offset. `null` if the banner is absent — legitimate, not an error. | `dumpstate.cpp:802-805` |
| `captured_at_time_base` | enum \| null | yes | `dumpstate_header` or `archive_filename`. `null` when `captured_at` is `null`. | `dumpstate.cpp:802`; filename form `:2835` |
| `uptime_at_capture_seconds` | integer \| null | yes | Uptime in seconds at capture. **Never inferred from logcat timestamps.** | `dumpstate.cpp:1687`, `:828-830` |
| `uptime.derivation` | string | yes when `uptime_at_capture_seconds` is non-null | The formula and its anchor lines. Required because the value is computed. | — |
| `log_window` | object | yes | See [§2.1.3](01-envelope.md#213-capture). | `logprint.cpp:1552-1554` |
| `boot` | object | yes | Derived boot instant plus its derivation and any epoch corroboration. | — |
| `is_cold_boot` | boolean | yes | `uptime_at_capture_seconds < 300`. | derived |

##### `build`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `display_id` | string | yes | `Build:` value. | `dumpstate.cpp:795,808` |
| `fingerprint` | string | yes | `ro.build.fingerprint`. The source comments that *"the fingerprint entry format is important for other tools."* | `dumpstate.cpp:796,810`; archive line 7 |
| `type` | enum \| null | yes | Read from `ro.build.type` when available, otherwise the fingerprint type segment; display IDs are not guaranteed to encode it. | `dumpstate.cpp:808` |
| `is_user_build` | boolean \| null | yes | `type == "user"`. One build-type predicate; [§3.2](03-build-variants.md#32-gated-sections) distinguishes report mode and other gates. | `DumpstateUtil.cpp:212-217` |
| `tags` | string \| null | yes | `dev-keys` / `release-keys`. | `display_id` tail |
| `incremental` | string \| null | yes | The incremental field of `display_id`. | `display_id` |
| `version.release` / `version.sdk` | string / integer | yes | The release name and API level. | `dumpstate.cpp:797,818` |
| `sdk_extensions` | array of string | yes | From the `SDK extensions: […]` line, produced by `SDK_EXT_INFO --header` with a 1 s timeout — may be absent. | `dumpstate.cpp:819-821` |
| `module_metadata_version` | integer \| null | yes | Only printed when non-zero. | `dumpstate.cpp:814-817` |
| `security_patch` | string \| null | yes | **Property-derived**; not automatically suppressed on full `user` reports. | `getprop` section |
| `build_date_utc` | string \| null | yes | `ro.build.date`. **`getprop`-only.** | `getprop` section |
| `bootloader` / `radio` / `network` | string \| null | yes | Header lines. `(unknown)` is an AOSP sentinel and must be normalised to `null`. | `dumpstate.cpp:798-813` |
| `boot_image_fingerprint` | string \| null | yes | `ro.bootimage.build.fingerprint`. **`getprop`-only.** | `getprop` section |

##### `product`

Read property-derived fields from available evidence.
Full reports request `getprop` on every build type (`dumpstate.cpp:1787`); telephony-only `user` reports omit that request.
Missing properties are `null`, independently of build type.
See [report modes](03-build-variants.md#32-gated-sections).

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `name` | string \| null | yes | `ro.product.name`. | `getprop` |
| `device` | string \| null | yes | `ro.product.device`. **Differs from `name`** — the `Build:` header carries one, the fingerprint the other. Store both, never one from the other. | `getprop`; `dumpstate.cpp:808,810` |
| `brand` / `manufacturer` / `model` | string \| null | yes | `ro.product.*`. | `getprop` |
| `first_api_level` | integer \| null | yes | `ro.product.first_api_level`. | `getprop` |
| `characteristics` | array of string | yes | `ro.build.characteristics`. On the reference archive the only *in-band* statement that the device is automotive. | `getprop` |
| `cpu_abi` / `cpu_abilist` | string \| null | yes | `ro.product.cpu.abi` / `.abilist`. | `getprop` |

##### `device`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `is_emulator` | boolean \| null | yes | `true` when any `is_emulator_basis` entry matched a published rule; `false` when none matched and every basis property was readable. **`null` when the required observations are unavailable** ([§2.9](02-evidence-conventions.md#29-null-absent-and-raw) rule 2). Rules `R1`–`R3` below. | `getprop` |
| `is_emulator_basis` | array of object | yes | One entry per property actually read: `{property, value, matched_rule}`. `matched_rule` is `R1`/`R2`/`R3` or `null`. Empty when `is_emulator` is `null`. | `getprop` |
| `avd_name` | string \| null | yes | `ro.boot.qemu.avd_name`. | `getprop` |
| `serial_no` | string \| null | yes | `ro.boot.serialno`. | `getprop` |
| `boot_hardware` | string \| null | yes | `androidboot.hardware` from `/proc/bootconfig`. **Available on all builds.** | `dumpstate.cpp:826-827` |
| `bootconfig_excerpt` | object | yes | The small set of keys that change device class: `boot_devices`, `hardware.vulkan`, `qemu`, `qemu.avd_name`, `qemu.skin`, `qemu.vsync`, `veritymode`, `vbmeta.digest`, `vbmeta.hash_alg`, `vbmeta.size`, `logcat`. | `dumpstate.cpp:826-827` |
| `cmdline_has_kvm_arm_protected` | boolean \| null | yes | Derived from `/proc/cmdline`. | `dumpstate.cpp:825` |

> **The `is_emulator` rule is fixed by this schema, not learned per archive.**
> Implement only the published table; per-run emulator heuristics are forbidden.
>
> | Condition | Property | Value | Effect |
> | --- | --- | --- | --- |
> | `R1` | `ro.hardware` | `ranchu` | `is_emulator: true` |
> | `R2` | `ro.kernel.qemu` | `1` | `is_emulator: true` |
> | `R3` | `ro.boot.qemu` | `1` | `is_emulator: true` |
>
> `true` if any of `R1`–`R3` matched; `false` only if all three properties were readable and none matched; `null` if the required property evidence is unavailable.
> No other rule may set this field.

##### `kernel`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `version_line` | string | yes | Verbatim `/proc/version`. | `dumpstate.cpp:823-824` |
| `version_release` | string \| null | yes | First token. | `dumpstate.cpp:823` |
| `cmdline` | string | yes | `/proc/cmdline`, first line only (`strtok(cmdline_buf, "\n")`). | `dumpstate.cpp:825` |
| `config_archived` | boolean | yes | `FS/proc/config.gz` present. Lets a consumer answer *"is KASAN on?"* without guessing. | `dumpstate.cpp:1950-1952` |

##### `users`

Sourced from `dumpsys user` — **not** from `Current user ids: [10]`, which is a SystemUI app-op monitor string at archive line 1957.
The authoritative multi-user facts are `DUMP OF SERVICE user:` → `Current user: 10` (line 613374) and `Users:` → `UserInfo{0:null:813}` / `UserInfo{10:Driver:412}` (lines 613377 / 613416).

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `current_user_id` | integer | yes | The `Current user: N` value. | `UserManagerService.java:7311-7317` |
| `list[]` | array of object | yes | One per `UserInfo{…}` line. | `UserManagerService.java:7322-7331` |
| `list[].id` | integer | yes | The user id. | `UserManagerService.java:7464` |
| `list[].name` | string \| null | yes | The profile name, `null` where the `UserInfo` carries none. | `UserInfo.java:528-533` |
| `list[].flags` | array of string | yes | Parsed from the symbol list, not the number. | `UserManagerService.java:7483-7485` |
| `list[].flags_hex` | integer | yes | The hexadecimal `UserInfo{…}` third field, parsed as a number (e.g. `813` hex becomes JSON integer `2067`). Preserve the source token in evidence alongside `flags`; never convert one into the other — the two use different conventions and conflating them produces two wrong numbers. | `UserInfo.java:532` |
| `list[].is_primary` | boolean | yes | From the `PRIMARY` flag symbol. | `UserManagerService.java:7466` |
| `list[].parent_id` | integer \| null | yes | `null` when no parent line is printed for this `UserInfo`. **No reason is published for the `null`** — whether the platform omits it for a full secondary user or for some other reason is AOSP behaviour the analyzer cannot confirm from the archive. Recorded in `evidence_gaps[]` as `UNVERIFIABLE`. | `UserManagerService.java:7467-7470` |
| `list[].type` | string | yes | The printed `Type:` line verbatim. | `UserManagerService.java:7482` |
| `list[].state` | string | yes | The printed `State:` line verbatim. | `UserManagerService.java:7486-7492` |
| `list[].serial_no` | integer | yes | The serial number from the `UserInfo` header. | `UserManagerService.java:7465` |
| `list[].start_time_ago` | string \| null | yes | The `+NmSmmm`-form "ago" string — an **uptime-relative** value, not a wall clock. Same clock as `ProcessRecord.startUpTime`. | `UserManagerService.java:7495-7497` |
| `list[].last_logged_in_ago` / `unlock_time_ago` / `last_entered_foreground_ago` | string \| null | yes | Same encoding and same clock. | `UserManagerService.java:7499-7510` |
| `list[].created_ago` | string \| null | yes | The creation "ago" string. `<unknown>` is a **literal in the archive**, normalised to `null` here per [§2.9](02-evidence-conventions.md#29-null-absent-and-raw) rule 1. | `UserManagerService.java:7493-7494` |
| `cross_check` | object | yes | **Reduced to raw counts only.** `package_dump_user_ids` (array of integer), `package_install_state_blocks`, `shared_user_blocks`, `shared_user_state_blocks`, `other_user_header_blocks`. All five are literal counts over the `DUMP OF SERVICE package:` section. **The `arithmetic` prose and the `agrees_with_dumpsys_user` verdict are removed** — a reconciliation sentence and a conclusion are judgment, and neither is needed for a consumer to see that the counts do or do not line up. | — |
| `cross_check` gap | — | — | Where the five counts do not close, the shortfall is published as an `evidence_gaps[]` entry with `reason: AMBIGUOUS_SOURCE` naming which blocks were unclassified. The tool does not choose a story that makes them close. | — |

##### `displays[]`, `hal`, `storage`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `displays[]` | `display_id` | integer | yes | The SurfaceFlinger display handle. | SurfaceFlinger CRITICAL dump, `dumpstate.cpp:1382` |
| | `hwc_display` / `port` | integer | yes | The HWC display index and the output port. | same |
| | `pnp_id` | string | yes | The plug-and-play id, `GGL` on a SwiftShader/emulated target. | same |
| | `display_name` | string | yes | The display name, `EMU_display_N` on an emulator. The only stable handle for an output. | same |
| `hal` | `hidl_services[]` | array of object | yes | Rows of `lshal --all --types=all`: `interface::instance`, `transport`, `server_pid`, `arch`, `threads_used`/`total`, `released`, `interface_hash`. `R` means released, not registered (`cmds/lshal/TableEntry.cpp:63,108-119`). | `dumpstate.cpp:1421-1423` |
| | `hidl_debug_entries[]` | array of string | yes | ZIP entries under `lshal-debug/`, one per interface whose `lshal debug -E` produced non-empty output within 2 s. | `dumpstate.cpp:1434-1466` |
| `storage` | `mountinfo_pids` | array of integer | yes | PIDs whose `/proc/<pid>/mountinfo` was read. Successful collection establishes that the PID was readable at that section's collection instant, not necessarily at the initial banner time. | `dumpstate.cpp:498-504` |
| | `archived_trees` | array of string | yes | Directory prefixes copied under `FS/`. | `dumpstate.cpp:1950-1952,1983,948,1106` |
| `storage` | `anr_files_archived` | array of object | yes | `path`, `mtime`, `bytes` per `FS/data/anr/anr_*`. **Freshness lives here.** `system.json` records the files as *device state*; `anr.json` decides staleness. | `dumpstate.cpp:1931-1933` |
| | `tombstone_count_archived` | integer \| null | yes | Count of tombstone ZIP entries. `crashes.json` owns their content. | `dumpstate.cpp:1931` |
| | `dynamic_partition_section_status` | enum | yes | The [§2.2](01-envelope.md#22-section-status-enum) status of the `LPDUMP` section. **Not** a boolean: the section can be present *and* have failed (`*** command 'lpdump --all' failed`), and a boolean cannot express that. | `dumpstate.cpp:1953` |

##### `archive_entries`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `proto[]` | array of object | yes | `path`, `bytes`. **`bytes: 0` is normal, not corruption** — `RunDumpsysProto` adds an entry for every service claiming proto support even if the dump wrote nothing. | `dumpstate.cpp:1340-1380` |
| `proto[].status` | enum | yes | `ok` or `empty` for the observed payload. Zero bytes alone cannot prove that a service intentionally declined; inspect failure/timeout diagnostics separately. | same |
| `proto.suffix_meaning` | object | yes | **Removed.** The `_CRITICAL` / `_HIGH` / `""` suffix scheme is a property of dumpstate's priority selection, not of any archive. A tool that needs it reads it from here, in prose; it never emits it as data. | — |
| `fs_paths` | array of string | yes | Every `FS/**` entry. | `dumpstate.cpp:375` |
| `fs_user_build_gated` | array of string | yes | **Removed.** The gated-path *list* is AOSP control flow. Presence or absence of a given path on a given archive is already visible in `fs_paths`; the enumeration of which paths *could* be gated is not a per-run observation. | — |
| `lshal_debug` | array of string | yes | ZIP entry names. | `dumpstate.cpp:1464` |
| `visible_windows` | object | yes | `present`, `entries[] = {id, title, bytes, mtime}`. **No `command`, `timeout_seconds`, or `producer`** — no archive prints which command produced it or how long it was given, so those three were unverifiable constants. `present: false` says all that is knowable. | `dumpstate.cpp:1111-1129` |
| `dumpstate_board` | object | yes | `present`, `bytes`, `has_board_binary`. Content is device-defined and unparseable in general. | `dumpstate.cpp:379-383` |

##### `scope.sections[]`

One entry per recognised section banner.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `title` | string | yes | Text between `------ ` and ` (`. | `DumpstateUtil.cpp:306` |
| `ordinal` | integer | yes | 0-based index among sections sharing the title. | — |
| `command` | string \| null | yes | The literal argv dumpstate ran, e.g. `/system/xbin/su root lshal --all --types=all`. `null` when the banner carries no command. **Keeping the `su root` prefix is what lets a build-type consumer work.** | `DumpstateUtil.cpp:290-297,306` |
| `status` | enum | yes | See [§2.2](01-envelope.md#22-section-status-enum). | — |
| `start_line` / `end_line` | integer | yes | Line span in the main entry. Enables re-reading without rescanning. | — |
| `duration_seconds` | number \| null | yes | From the paired duration trailer. A long duration with no content means a timeout. | `dumpstate.cpp:3831-3835` |
| `note` | string \| null | yes | **Removed.** A free-text "why the status is what it is" is prose about a collector the analyzer cannot inspect. The `status` enum from [§2.2](01-envelope.md#22-section-status-enum) already carries the machine-readable reason; where that is not enough, the fact belongs in `evidence_gaps[]`. | — |

#### Provenance and freshness

Without *findings*, provenance is **per-fact**: every leaf needs `evidence` with exact archive lines/section and one of four `observed_at` classifications:

| `observed_at` | Meaning | Examples |
| --- | --- | --- |
| `capture` | A point-in-time snapshot taken by this bugreport. | `Product LRU list`, `Users`, `displays`, `PID mappings` |
| `boot` | Fixed at image build time; **not** a runtime observation. | `build.*`, `product.*`, `kernel.version_line`, package `codePath` |
| `interval` | Summarises the log window. | `capture.log_window` |
| `archive-entry` | A copied file, whose truth is its **mtime**, not the capture time. | `FS/data/anr/*`, `FS/proc/*/mountinfo`, `proto/*`, `visible_windows.zip` |

`stale_relative_to_capture_seconds` is computed **only** for `archive-entry` facts, and is `null` everywhere else.
On this archive the staleness is extreme and structurally visible: ANR files predate capture by **126 days**.
Compute it once in `system.json`; other artifacts reference it.

#### Worked example

Abbreviated from the reference archive.
**Values are placeholders.**
The fixture is illustrative; concrete device/package/time identifiers must not be mistaken for schema rules.
Preserved: names, types, nesting, enums and rule-bearing forms, including `[<lower>, <upper>]` bounds and `"…"` elision.

```jsonc
{
  "schema_version": 1,
  "artifact": "system",

  "source": {
    "archive_name": "<archive_stem>.zip",
    "archive_size_bytes": 14710126,
    "archive_sha256": "<64 hex chars>",
    "format_version": "2.0",
    "main_entry": "<archive_stem>.txt",
    "main_entry_size_bytes": 64645191,
    "main_entry_line_count": 682951,
    "zip_entry_count": 398,            // read from the ZIP central directory
    "device": {
      "product_name": "<product>",
      "build_id": "<build_id>",
      "build_display_id": "<product>-<type> <release> <build_id> <incremental> <tags>",
      "build_fingerprint": "<vendor>/<product>/<device>:<release>/<build_id>/<incremental>:<type>/<tags>",
      "android_sdk_version": 35,
      "build_type": "<type>",
      "device": "<device>",
      "hardware": "<hardware>",
      "bootloader": null,             // header prints "(unknown)" — normalised to null per §2.9
      "radio": null                   // header prints "(unknown)" — normalised to null per §2.9
    }
  },

  "capture": {
    "captured_at": "<captured_at>",
    "captured_at_time_base": "dumpstate_header",
    "uptime_at_capture_seconds": <uptime_seconds>,
    "uptime": {
      "header_raw": "up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: <load_1m>, <load_5m>, <load_15m>",
      "section_raw": "HH:MM:SS up 0 min,  <users> users,  load average: <load_1m>, <load_5m>, <load_15m>",
      "raw_precision": "minutes",
      "seconds_at_start": <uptime_seconds_exact>,
      "bounds_seconds": [<uptime_lower_bound_seconds>, <uptime_upper_bound_seconds>],
      "derivation": "Boot instant B solved from independent (wall clock, uptime) pairs, each printed inside a section that records its own end time and duration. For each pair B = footer_end_wall − section_duration − uptime_ms/1000. The pairs bracket B to [<B_lower>, <B_upper>]; a kernel 'Booting Linux' message fixes the upper bound. Uptime at capture = banner_wall − B, giving the published bounds.",
      "derivation_inputs": [
        { "line": <line>, "kind": "meminfo_uptime", "wall": "<wall_clock>", "uptime_ms": <ms> },
        { "line": <line>, "kind": "content_uptime", "wall": "<wall_clock>", "uptime_ms": <ms> },
        { "line": <line>, "kind": "gfxinfo_uptime",  "wall": "<wall_clock>", "uptime_ms": <ms> },
        { "line": <line>, "kind": "kernel_boot",     "wall": "<wall_clock>", "uptime_ms": null },
        { "line": <line>, "kind": "banner",          "wall": "<wall_clock>", "uptime_ms": null }
      ]
    },
    "log_window": {
      "sections": ["SYSTEM LOG", "EVENT LOG", "STATS LOG", "RADIO LOG", "LAST LOGCAT", "SYSTEM LOG"],
      "first_line_at": "MM-DD HH:MM:SS.mmm",
      "last_line_at": "MM-DD HH:MM:SS.mmm",
      "duration_seconds": <log_window_seconds>,
      "is_contiguous": false,
      "first_line_minus_boot_seconds": <delta_seconds>,
      "truncated": null
    },
    "boot": {
      "at": "<boot_epoch>",
      "bounds_seconds": [<B_lower>, <B_upper>],
      "derivation": "see capture.uptime.derivation",
      "epoch_corroboration": {
        "ro_runtime_firstboot": <epoch_ms>,
        "implied_at_local": "<captured_at>",
        "agrees_with_derived": false,
        "delta_seconds": <delta_seconds>
      }
    },
    "is_cold_boot": true
  },

  "build": {
    "display_id": "<product>-<type> <release> <build_id> <incremental> <tags>",
    "fingerprint": "<vendor>/<product>/<device>:<release>/<build_id>/<incremental>:<type>/<tags>",
    "type": "<type>",
    "is_user_build": false,
    "tags": "<tags>",
    "incremental": "<incremental>",
    "version": { "release": "<release>", "sdk": 35 },
    "sdk_extensions": ["<ext>=<n>", "…"],
    "module_metadata_version": 35,
    "security_patch": "<YYYY-MM-DD>",
    "build_date_utc": "<build_date>",
    "bootloader": null,
    "radio": null,
    "network": null
  },

  "product": {
    "name": "<product_name>", "device": "<device_name>",
    "brand": "<brand>", "manufacturer": "<manufacturer>", "model": "<model>",
    "first_api_level": 35,
    "characteristics": ["automotive"],
    "cpu_abi": "<abi>", "cpu_abilist": "<abilist>"
  },

  "device": {
    "is_emulator": true,
    "is_emulator_basis": [
      { "property": "ro.hardware",   "value": "ranchu", "matched_rule": "R1" },
      { "property": "ro.kernel.qemu", "value": "1",     "matched_rule": "R2" },
      { "property": "ro.boot.qemu",   "value": null,   "matched_rule": null }
    ],
    "avd_name": "<avd_name>",
    "serial_no": "<serial>",
    "boot_hardware": "<hardware>",
    "bootconfig_excerpt": {
      "boot_devices": "<block_device>", "hardware.vulkan": "<hardware>",
      "qemu": "1", "qemu.skin": "<skin>", "qemu.vsync": "<hz>",
      "veritymode": "<mode>", "vbmeta.hash_alg": "<alg>", "logcat": "<logcat_level>"
    },
    "cmdline_has_kvm_arm_protected": true
  },

  "kernel": {
    "version_release": "<kernel_release>",
    "cmdline": "console=ttynull … kvm-arm.mode=protected bootconfig …",
    "config_archived": true
  },

  "users": {
    "current_user_id": 10,
    "list": [
      { "id": 0, "name": null, "flags": ["ADMIN","INITIALIZED","PRIMARY","SYSTEM"], "flags_hex": 2067,
        "is_primary": true, "parent_id": null,
        "type": "android.os.usertype.system.HEADLESS", "state": "RUNNING_UNLOCKED",
        "serial_no": 0, "created_ago": null,
        "start_time_ago": "+49s735ms", "last_logged_in_ago": "+45s957ms",
        "unlock_time_ago": "+46s40ms", "last_entered_foreground_ago": null },
      { "id": 10, "name": "Driver", "flags": ["ADMIN","FULL","INITIALIZED"], "flags_hex": 1042,
        "is_primary": false, "parent_id": null,
        "type": "android.os.usertype.full.SECONDARY", "state": "RUNNING_UNLOCKED",
        "serial_no": 10, "created_ago": "+128d0h36m51s528ms",
        "start_time_ago": "+49s266ms", "last_logged_in_ago": "+44s755ms",
        "unlock_time_ago": "+45s865ms", "last_entered_foreground_ago": "+48s813ms" }
    ],
    "cross_check": {
      "package_dump_users": [0, 10],
      "package_install_state_blocks": 564,
      "shared_user_blocks": 14,
      "shared_user_state_blocks": 28,
      "other_user_header_blocks": 2
    }
  },

  "hal": {
    "hidl_services": [
      { "interface_instance": "<vendor.hardware.<name>@<ver>/<name>::default",
        "transport": "binder", "server_pid": <pid>, "arch": "<arch>",
        "threads_used": <n>, "threads_total": <n>, "released": true, "interface_hash": "<hash>" },
      "…"
    ],
    "hidl_debug_entries": [
      "<vendor.hardware.<name>@<ver>__<Iface>_default.txt",
      "…"
    ]
  },

  "storage": {
    "mountinfo_pids_count": <count>,
    "archived_trees": ["data/", "linkerconfig/", "proc/", "product/", "system/", "system_ext/", "vendor/"],
    "anr_files_archived_count": <count>,
    "anr_files_date_range": ["<YYYY-MM-DD>THH:MM:SS", "<YYYY-MM-DD>THH:MM:SS"],
    "anr_files_staleness_seconds": <seconds>,
    "tombstone_count_archived": <count>,
    "tombstone_zip_entries": <count>,
    "dynamic_partition_section_status": "failed"
  },

  "archive_entries": {
    "proto_empty_count": <count>,
    "visible_windows": {
      "present": true,
      "entries": [
        { "id": "<hash>", "title": "<package>/<activity>", "bytes": <n>, "mtime": "<YYYY-MM-DD>THH:MM" },
        { "id": "<hash>", "title": "<SystemUi window title>", "bytes": <n>, "mtime": "<YYYY-MM-DD>THH:MM" }
      ]
    },
    "dumpstate_board": { "present": true, "bytes": <n>, "has_board_binary": false,
      "content_excerpt": "<verbatim first N bytes>" }
  },

  "scope": {
    "sections_scanned": <count>,
    "sections_matched": <count>,
    "sections_listed": ["UPTIME", "DUMPSYS CRITICAL", "…", "SYSTEM LOG", "…", "SYSTEM LOG"],
    "sections_skipped": [],
    "sections_not_found": [],
    "max_lines_read": <line_count>,
    "truncated": false,
    "sections": [
      { "title": "UPTIME", "ordinal": 0, "command": "uptime", "start_line": <line>, "end_line": <line>,
        "duration_seconds": <s>, "status": "ok" },
      { "title": "HARDWARE HALS", "ordinal": 0, "command": "/system/xbin/su root lshal --all --types=all",
        "start_line": <line>, "end_line": <line>, "duration_seconds": <s>, "status": "ok" },
      { "title": "SYSTEM PROPERTIES", "ordinal": 0, "command": "getprop",
        "start_line": <line>, "end_line": <line>, "duration_seconds": <s>, "status": "ok" },
      { "title": "ANR FILES", "ordinal": 0, "command": "ls -lt /data/anr/", "start_line": <line>,
        "end_line": <line>, "duration_seconds": <s>, "status": "ok" },
      { "title": "VM TRACES AT LAST ANR", "ordinal": 0,
        "command": "/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>", "start_line": <line>, "end_line": <line>,
        "duration_seconds": <s>, "status": "ok" }
    ]
  },

  "evidence_gaps": [
    { "field": "capture.uptime_at_capture_seconds", "reason": "PARSE_INDETERMINATE",
      "detail": "derived, not read. No archive-internal source states seconds-level uptime directly; the published value rests on a solved boot bracket whose bounds are dominated by the 1-second truncation of the dumpstate banner clock. header_raw and section_raw are minute-granular and cannot corroborate a seconds-level figure. See uptime.derivation and uptime.bounds_seconds." },
    { "field": "capture.log_window.truncated", "reason": "NOT_PRINTED",
      "detail": "dumpstate records no logcat truncation flag." },
    { "field": "users.list[].parent_id", "reason": "UNVERIFIABLE",
      "detail": "no parent line is printed for these users. Whether the platform omits it for a full secondary user or for another reason is AOSP behaviour this archive cannot confirm." },
    { "field": "hal.aidl_services", "reason": "UNVERIFIABLE",
      "detail": "the lshal rows enumerate the HIDL hwservicemanager only, so AIDL services are outside this section entirely. Absence here says nothing about AIDL." },
    { "field": "storage.dynamic_partition_section_status", "reason": "SECTION_FAILED",
      "detail": "the LPDUMP section is present but its command exited non-zero, so no partition metadata was captured." },
    { "field": "users.cross_check.other_user_header_blocks", "reason": "AMBIGUOUS_SOURCE",
      "detail": "these User N: blocks are not attributable to either the install-state or SharedUser sets by any archive-visible rule. The tool publishes the residual count rather than assigning the blocks to a category." }
  ],

  "findings": [],

  "notes": [
    "EMPTY_FINDINGS_BY_DESIGN: system.json is the fact sheet. It publishes what was captured, when, and on what, and carries no findings of its own; every finding in the output set belongs to one of the five signal artifacts. This array is empty rather than absent so the key count matches the shared envelope, and this note exists because §2.1 requires an empty findings array to say why.",
    "GAPS_ARE_DECLARED_NOT_HIDDEN: the evidence_gaps above are the limit of what this archive determines. An empty gaps array would assert that every published field was derivable from these bytes, which is almost never true."
  ]
}
```

#### Open questions

- **Uptime at capture is derived from device clocks.**
  No archive-internal source states seconds-level uptime directly, so the published value rests on a solved boot bracket: several `(wall clock, uptime)` pairs are read from inside sections that also print their own end time and duration, and the kernel `Booting Linux` message fixes an upper bound.
  The artifact publishes `uptime.bounds_seconds` as an interval rather than a point estimate with an error bar, and records the limit as an `evidence_gaps[]` entry with `reason: PARSE_INDETERMINATE`.
  **What remains genuinely open:** the bounds are dominated by the 1-second truncation of the `== dumpstate:` banner clock and of the `uptime` wall clocks in `header_raw` / `section_raw`.
  If `/proc/uptime` or equivalent seconds-precision evidence becomes available, use it directly and remove the gap.
- **Timezone.**
  The `== dumpstate:` header carries no offset and no epoch (`dumpstate.cpp:804`).
  Every absolute time is therefore "device-local, offset unknown".
  Recoverable only if some section prints both an absolute time and an uptime delta.
  `ro.runtime.firstboot` disagrees with the solved boot instant by 19.1 s, so it was not used.
  `timezone_offset_minutes` is `null` rather than a guessed `+07:00`.
- **`MOUNT INFO` section.**
  `add_mountinfo()` (`dumpstate.cpp:498`) creates a *duration-reporter* titled `MOUNT INFO`, **not** a `------ ` banner, so a `sections[]` index keyed on banners does not capture it.
  On this archive it does not appear.
  A parser must not conclude "no mount information" from "no banner".
- **Shared-user header source.**
  `Settings.java:5766-5773` prints `SharedUser [name] (identityHash):` and `appId=`.
  It is not a missing source.
- **`display_id` / `hwc_display` / `port` / `pnpId` / `displayName`.**
  Read from the `DUMPSYS CRITICAL SurfaceFlinger` block; the `dumpsys SurfaceFlinger` producer is citable (`dumpstate.cpp:1382`) but the per-field printer line in `SurfaceFlinger.cpp` is not.
- **lshal columns.**
  `TableEntry.cpp:63,108-119` maps `R` to interface release status: `Y`, `N` or `?`, based on the interface hash.
  Preserve unknown as `null`; `R` is not a registration flag.
  Registration must use separate service-status evidence.
- **`dumpstate_board.txt` content.**
  Produced by the device-side `IDumpstateDevice` HAL and unparseable in general.
  Presence and size only; per-OEM parsing is out of scope.
- **`package_dump_user_blocks` arithmetic.**
  Measured 594 against an expected 592.
  Two of the `User N:` blocks in the `DUMP OF SERVICE package:` section are not accounted for by `282 × 2 + 14 × 2`.
  Arithmetic/classification differs by two; retain the unresolved `evidence_gaps` entry.
- **ZIP comment.**
  `ZipWriter` has no comment API and `EocdRecord.comment_length` (`zip_archive_common.h:60-61`) is never assigned.
  Read it opportunistically if present; never require it.

---
