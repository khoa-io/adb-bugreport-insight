# `README.md` — the human triage document

> **Draft.** See [`index.md`](./index.md). Requirement: `DRAFT-REQ-ART-01`.

Original §4.1 of `artifact-schemas.md`, preceded by the §4 preamble. It is the only artifact a person
reads end to end, so it is the only one whose section *order* is normative: identity, capture
context, verdict, top findings, freshness ledger, noise register, evidence index, what this archive
cannot tell you, reproduction inputs.

Field-level detail it depends on lives in [`conventions-evidence.md`](./conventions-evidence.md).
## 4. The Seven Artifacts

Each entry below gives the artifact's purpose, its top-level keys, field tables for its
domain objects, provenance and freshness notes, a worked example from the reference archive,
and the open questions carried over from the owning team file.

**The reference archive** used for every worked example is:

```
tmp/bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17/
  bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.txt   64,645,191 B, 682,951 lines
  version.txt  main_entry.txt  dumpstate_log.txt  dumpstate_board.txt
  FS/  proto/  lshal-debug/  visible_windows.zip
```

`version.txt` is `2.0` (3 bytes, no trailing newline) and `main_entry.txt` names the main
`.txt`. Note that `bugreport-format.md:37` documents the metadata entry as `main-entry.txt`
with a hyphen; the code writes `main_entry.txt` with an underscore at `dumpstate.cpp:2752`.
**Trust the code.**

---

### 4.1 `README.md`

#### Purpose

`README.md` is the **human triage document** — the thing an engineer reads before opening the
JSON. It answers, in order: *what is this device, is it healthy right now, what is the single
most important thing, how much of this can I trust, and where do I look next.* It is
generated, but every number in it is a pointer to a JSON finding; it never asserts anything
the JSON does not.

#### Structure

`README.md` is Markdown and carries no envelope. It has nine sections, in this order:

| # | Section | Answers | Grounded in |
| --- | --- | --- | --- |
| 1 | **Header** — product, build, format, archive name and hash | *What am I looking at?* | `source.device`, `source.format_version`, `source.archive_sha256` |
| 2 | **Capture context** | *When was this captured, and is the device even up?* | `capture.captured_at`, `capture.uptime_at_capture_seconds`, `source.device` |
| 3 | **Verdict** | *Is this device healthy right now?* | Derived: worst `severity` among `freshness ∈ {now, in_window}` findings |
| 4 | **Top findings** | *What do I fix first?* | Top 10 findings by `(severity desc, age_seconds_at_capture asc)`, freshness shown inline |
| 5 | **Freshness ledger** | *Which parts of this archive can I trust?* | `freshness_summary` from all six JSON artifacts, plus the section age table |
| 6 | **Noise register** | *What did you search for and deliberately not report?* | Every `counts.*` match block |
| 7 | **Evidence index** | *Where do I look in the 64 MB file?* | De-duplicated list of every `evidence` locator, grouped by archive path |
| 8 | **What this archive cannot tell you** | *Where are the blind spots?* | `scope.sections_not_found`, `scope.truncated`, `notes`, `negative_evidence` |
| 9 | **Reproduction** | *Can I regenerate this?* | `generator.*`, `source.archive_sha256`, the command line |

Sections 1–6 are mandatory and always rendered. Sections 4 is capped at 10 rows; 7–9 are
generated tables and are the only ones that grow. Target length **≤ 400 lines**.

#### Section 3 — the verdict, and the rule that keeps it honest

The verdict is a single sentence derived **only** from findings whose `freshness` is `now` or
`in_window`. Three forms:

- **No current findings** → *"No `now`/`in_window` findings. This is not a clean bill of
  health: **N** findings are `stale` and **M** are `undated`; see §5."*
- **Only `info`/`low` current** → *"N current findings, none above `low`."*
- **`high` or `critical` current** → *"**N** current `high`/`critical` findings; oldest is **A** s old."*

**The cold-boot rule.** When `capture.uptime_at_capture_seconds < 300`, section 3 must carry
this banner verbatim, before any finding list:

> #

### ⚠ This archive is a cold boot
> The device had been up **30 s** when this bugreport was taken, and the log window covers
> **64.662 s** (`00:26:47.418` → `00:27:52.080`). There is **no steady-state window** in this
> archive. Absence of a finding here is weak evidence of absence on the device: caches are
> cold, lazy services have not started, and a boot-time failure that self-heals would leave no
> trace. Conversely, boot-time churn is expected and is **not** by itself a defect.

Measured: `------ UPTIME (uptime) ------` (archive line 57012) prints
`00:27:25 up 0 min,  0 users,  load average: 9.66, 2.36, 0.79`; the header line 43 prints
`Uptime: up 0 weeks, 0 days, 0 hours, 0 minutes`. The `00:27:25` on that line is the **wall
clock** at which `uptime` was sampled, **not** an elapsed time — the elapsed field on the same
line is only `"0 min"`. The published uptime of 30 s comes from `system.json#/capture/uptime`.
A reader who sees "no ANR in the last 64.7 s" must not conclude "no ANR".

#### Section 5 — the freshness ledger

The section that makes the whole design pay off. One row per non-current evidence class,
**always rendered even when empty**:

| Evidence | Captured at | Age | Freshness | Basis |
| --- | --- | --- | --- | --- |
| `SYSTEM LOG` (ordinal 0) | 00:26:47.418 → 00:27:20.131 | 0–33 s | `now` | `logcat_line` |
| `SYSTEM LOG` (ordinal 1) | 00:27:20.000 → 00:27:52.080 | 0–35 s | `now` | `logcat_line` |
| `VM TRACES JUST NOW` | 2026-09-24T00:27:25+07:00 | ~8 s | `now` | `file_section_header_mtime` |
| `BINDER *` (5 sections) | 2026-09-24T00:26:52+07:00 | ~145 s | `in_window` | `file_section_header_mtime` |
| `RT_TABLES` | 2026-09-24T00:27:10+07:00 | ~67 s | `in_window` | `file_section_header_mtime` |
| **`FS/data/anr/` — 65 files** | 2026-05-20T00:54:11+07:00 → 05:13:43+07:00 | **126 d 23 h 33 m** | **`stale`** | `file_name` / `zip_entry_mtime` |
| **`VM TRACES AT LAST ANR`** | 2026-05-20T05:13:43+07:00 | **126 d 19 h 13 m** | **`stale`** | `file_section_header_mtime` |
| **`FS/data/tombstones/` — 50 files** | 2026-05-20T04:00:14+07:00 | **126 d 20 h 27 m** | **`stale`** | `zip_entry_mtime` / `tombstone_header` |
| `* BUILD-TIME RELEASE FLAGS` (4) | 2026-01-27T20:47:21+07:00 | **239 d 3 h 40 m** | **`stale`** | `file_section_header_mtime` |
| `DUMPSYS CRITICAL/HIGH/DUMPSYS` (276 services), `CPU INFO`, `MEMORY INFO`, `PROCESS TIMES` | — | — | `undated` | `dumpsys_section_snapshot` / `command_output` |

Rules for this section:

- **Every row names its `captured_at_time_base`.** A reader must never see a time without its
  basis.
- **The `undated` row is never omitted**, even if it is the only row. *"I looked and could not
  date this"* is the single most important thing this tool knows how to say.
- **A cross-boot row is called out explicitly**, in plain language: *"This trace was taken on
  a **previous boot** — the device had been up 8 h 39 m when the ANR was recorded, but only
  30 s when the bugreport was taken."*
- **The two `SYSTEM LOG` sections are two rows, with their overlap stated.** The README must
  say that the sections overlap by ~131 ms and that event counts are de-duplicated on
  `(timestamp, pid, tid, tag, message)`.

#### Section 6 — the noise register

Never omitted. One row per `counts.*` match block. From the reference archive:

| Searched | Pattern | Kind | Raw hits | Matched | Suppressed | Top suppression reason |
| --- | --- | --- | --- | --- | --- | --- |
| `tombstone` | `tombstone` | substring, case-sensitive | **749** | **0** | 749 | `mapped_library_path` (137 × `libtombstoned_client.so`) |
| `signal ` | `signal ` | substring, case-sensitive | **656** | **0** | 656 | `routine_teardown_signal` (417 × `init: Sending signal 9 …`, `system/core/init/service.cpp:206`) |
| `watchdog` | `watchdog` | substring, case-sensitive | **315** | **6** | 309 | `configuration_path` + `identifier_symbol` |
| `ANR in ` | `ANR in ` | substring | **0** | — | — | *see the mandatory footnote* |

The `ANR in ` row carries a **mandatory footnote** whenever real evidence exists but the tag
does not:

> **`ANR in ` matched 0 times, and that is not evidence of "no ANR".** This device emitted no
> `am_anr` / `appnotresponding` event in the 64.7 s log window, but the archive contains
> **65 ANR trace files** in `FS/data/anr/` spanning **4 h 19 m** on a boot 126 days before
> capture. ANRs are sourced from the `ANR FILES` listing (`dumpstate.cpp:1204`) and the
> `FS/data/anr/` entries (`dumpstate.cpp:1188`), not from a logcat tag.

#### Worked example

```markdown
# bugreport-insight — sdk_gcar_arm64 (emulator_car64_arm64)

| | |
|---|---|
| Archive | `bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip` (64,645,191 B) |
| Format | 2.0 (`version.txt`), main entry `bugreport-…-00-27-17.txt` (682,951 lines) |
| Build | `AAG6.260127.001` · `sdk_gcar_arm64-userdebug 15 … dev-keys` · SDK 35 · userdebug |
| Fingerprint | `google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys` |
| Captured | 2026-09-24T00:27:17+07:00 |
| Device uptime at capture | **30 s** ± 0.6 s (derived — see `system.json#/capture/uptime/derivation`) |
| Generated by | bugreport-insight 0.1.0 at 2026-10-01T00:31:12+07:00 |

> **⚠ The majority of findings in this report are more than 24 hours old.** They describe the
> device as it was on **2026-05-20**, not as it is. Only findings marked `now` or `in_window`
> describe the capture moment.

> **⚠ This archive is a cold boot.** The device had been up **30 s** when this bugreport was
> taken and the log window covers **64.662 s** (`00:26:47.418` → `00:27:52.080`). There is no
> steady-state window in this archive; absence of a finding is weak evidence of absence.

## Verdict

**2 `current` `critical` findings; oldest is 0 s old.**

The device is **actively crash-looping** at the moment of capture.

## 5. Freshness ledger

| Evidence | Captured at | Age | Freshness | Basis |
| --- | --- | --- | --- | --- |
| `SYSTEM LOG` (ordinal 0) | 2026-09-24T00:26:47+07:00 → 00:27:20+07:00 | 0–33 s | `now` | `logcat_line` |
| `SYSTEM LOG` (ordinal 1) | 2026-09-24T00:27:20+07:00 → 00:27:52+07:00 | 0–35 s | `now` | `logcat_line` |
| `VM TRACES JUST NOW` | 2026-09-24T00:27:25+07:00 | ~8 s | `now` | `file_section_header_mtime` |
| **The ANR set is from a different boot.** The newest trace's own header reads
> `----- dumping pid: 17511 at 31190738` (`StackTracesDumpHelper.java:599-602`) — the device
> had been up **8 h 39 m 50 s** when that ANR was recorded, against **30 s** at capture. It is
> not merely old; it is from a boot that ended 126 days ago.
```

#### Resolved

- **`capture.log_window.overlaps_boot_instant` — resolved: `false`.** With the corrected boot
  instant `B ≈ 00:26:47.33` ± 0.10 s, the first parseable logcat line (`00:26:47.418`) falls
  **inside** the bracket rather than before it — it is the kernel's own
  `Booting Linux on physical CPU 0` line at archive line 9126. So the log window begins
  essentially *at* boot, not before it, and the earlier "precedes boot by 12.464 s" reading was
  an artifact of the wrong boot instant.
- **Uptime at capture — resolved: 30 s ± 0.6 s.** Two earlier figures were published and both
  were wrong. **27 s** came from reading the wall clock `00:27:25` on the `UPTIME` section line
  as an elapsed time; the elapsed field on that line is only `"0 min"`. **17.118 s** is
  `00:27:17.001 − 00:26:59.883`, i.e. the **age of the `com.android.car` process at the banner
  instant**, not device uptime. The published figure and its derivation are in
  [`system.json`](./artifact-system.md#42-systemjson).

---

### 4.2 `system.json`
