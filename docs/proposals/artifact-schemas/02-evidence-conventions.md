# Shared conventions — provenance, freshness, evidence, identity

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).


Identifying values in examples follow the [example notation](index.md#example-notation).

### 2.4 Provenance

Required on **every finding** and on **every collection object**.
Flat per-finding provenance; no `capture` wrapper.

```text
provenance = {
  evidence_origin,                  // the channel name this came from, e.g. "logcat_anr_tag"
  archive_path, section_title, section_ordinal,
  line_start, line_end,             // archive locator; line_start is the ordering key
  captured_at,                      // RFC 3339 + explicit offset, or null
  captured_at_time_base,            // 11-value enum, below
  evidence_time_source,             // closed enum, below
  age_seconds_at_capture,           // integer, or null when unknowable
  capture_kind,                     // closed enum: what KIND of measurement this is
  freshness,                        // now | in_window | stale | undated
  freshness_basis: { input, basis, threshold_seconds },
  cross_boot_reference,             // the previous boot's instant, when known
  collector_perturbed,              // true | false | null
  collector_perturbed_basis         // string | null
}
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `evidence_origin` | string | yes | The AOSP channel the evidence came from. Traceable to a `path:line`. |
| `archive_path` | string | yes | ZIP entry path, exactly as in the archive. `"-"` only when the finding is about the archive as a whole. |
| `section_title` | string \| null | yes | The `------ … ------` title, or `null` outside a section. |
| `section_ordinal` | integer ≥ 0 \| null | yes | 0-based index among sections sharing a title. **Mandatory** — `SYSTEM LOG` appears twice in the reference archive (archive lines 9116 and 653373), so title alone is not a key. `null` outside a section. |
| `line_start` / `line_end` | integer ≥ 1 \| null | yes | 1-based, inclusive. Always emit both; `line_end == line_start` for a single line. |
| `captured_at` | string \| null | yes | **When the evidence was captured.** Never substitute the archive's capture instant for it — that is the bug this whole structure exists to prevent. |
| `captured_at_time_base` | enum | yes | Which clock. See below. |
| `evidence_time_source` | enum | yes | Where `captured_at` came from. See below. |
| `age_seconds_at_capture` | integer \| null | yes | `capture.captured_at − provenance.captured_at`. `null` if either side is `null`. Clamp post-start collection evidence to `0` and note `COLLECTED_AFTER_START`; evidence collected after the banner is expected and alone does not prove clock skew. |
| `capture_kind` | enum | yes | **Required, not optional.** See below. |
| `freshness` | enum | yes | See [§2.5](#25-freshness). |
| `freshness_basis` | object | yes | `{ input, basis, threshold_seconds }`. All three required. |
| `cross_boot_reference` | integer \| null | yes | Milliseconds of uptime on a **different boot epoch**. `null` otherwise. |
| `collector_perturbed` | boolean \| null | yes | Does the evidence establish an effect of this collection on the number? `null` when overlap alone cannot decide it. |
| `collector_perturbed_basis` | enum \| null | yes | **Now a closed code, not a sentence.** `section_measured_before_dump`, `section_measured_during_dump`, `read_from_disk`, `live_log_capture`, `undated_source`. A tool picks the code by comparing the evidence's own clock against `capture.captured_at`; where that comparison is impossible because the evidence is undated, the value is `null` and an `evidence_gaps[]` entry records it. The prose form was unusable: it either restated what the code already said, or it asserted a mechanism ("the dumpstate pass did not generate these") that the archive cannot confirm. |

#### `captured_at_time_base` — the 11-value enum

Five coexisting time bases need distinct names.
**Never assume the logcat clock.**

| Value | Base | AOSP source | Present in the reference archive? |
| --- | --- | --- | --- |
| `logcat_line` | 2 | `system/logging/liblog/logprint.cpp:1464,1472` — `MM-DD HH:MM:SS.mmm`, **year-less, no offset** | yes, ~74,000 lines |
| `file_section_header_mtime` | 5 | `frameworks/native/cmds/dumpstate/DumpstateInternal.cpp:148` | yes, 14 sections |
| `zip_entry_mtime` | 5 | `dumpstate.cpp:867-868` → `get_mtime` `:4663-4669` | yes, all 65 ANR + 100 tombstone entries |
| `file_name` | 5 | `frameworks/base/…/am/StackTracesDumpHelper.java:69-70,455-456` — ANR filename encodes `yyyy-MM-dd-HH-mm-ss-SSS` | yes, 65 ANR files |
| `tombstone_header` | 5 | `system/core/debuggerd/util.cpp:63-75` → `tombstone_proto.cpp:847` → `tombstone_proto_to_text.cpp:616` | yes, 50 tombstones |
| `anr_in_file_uptime` | 3 | `StackTracesDumpHelper.java:599-602`, `SystemClock.uptimeMillis()` | yes — see `cross_boot_reference` |
| `dumpsys_section_snapshot` | none | `dumpstate.cpp:1319`, `RunDumpsys` `:4425-4431` | yes, 276 service headers |
| `command_output` | none | `frameworks/native/cmds/dumpstate/DumpstateUtil.cpp:304-306` | yes, 1,100 section headers |
| `duration_delta` | 3 | `dumpstate.cpp:3833`, `Nanotime()` deltas | yes, 1,100 trailers |
| `process_uptime` | 4 | `tombstone_proto.cpp:857-863`; `tombstone_proto_to_text.cpp:617` | yes — never a wall clock |
| `undetermined` | none | no source | yes, for every bare `dumpsys` snapshot |

**The logcat year problem is real and must be handled explicitly.**
Base 2 carries no year and no offset.
Resolution, in order:

1. If `capture.captured_at` is known, take its year **and** its UTC offset.
2. If a logcat timestamp's month-day exceeds `capture.captured_at`'s month-day by more than one day, the log wrapped a year boundary: subtract one year.
   (The reference archive's window is `09-24`, same day as capture — no wrap.)
3. If `capture.captured_at` is `null`, the year is `null` ⇒ `captured_at: null`, `captured_at_time_base: "undetermined"`, `freshness: "undated"`.
   **Never guess a year.**

Every finding so marked must also emit `notes` entry `LOGCAT_YEAR_UNRESOLVED`.

#### `evidence_time_source` — the closed enum

Which of these produced `captured_at`:

```text
logcat_line | event_log_entry | dumpsys_Timestamp | file_mtime |
filename_timestamp | embedded_trace_stamp | none
```

#### `capture_kind` — the closed enum

What *kind* of measurement this is, independent of when it was taken:

```text
point_in_time_snapshot | background_sampler | persisted_aggregate | live_log |
disk_artifact | command_output
```

`capture_kind` is **required**.
Measurement kind informs staleness:

- `persisted_aggregate` is what catches the 128-day-old `procstats` blocks — whose `AGGREGATED OVER LAST 3 HOURS` label resolves to a `Start time` of 2026-05-19.
- `background_sampler` is what rescues `dumpsys cpuinfo` — its 1.772 s sample window ended **18.415 s before dumpstate even started**, so it is clean of collection cost.

### 2.5 Freshness

Four states, mechanically assigned:

```text
freshness ∈ now | in_window | stale | undated
```

| Value | Legal when | Meaning |
| --- | --- | --- |
| `now` | The evidence's clock demonstrably falls inside `capture.log_window`, and at the capture instant. | Taken as part of this dump. |
| `in_window` | The evidence's clock demonstrably falls inside `capture.log_window`. | Inside the captured interval, not at its head. |
| `stale` | The clock is known and outside the window, **or** the cross-boot check fires, **or** `age_seconds_at_capture >= 3600`. | Provably older than the capture. |
| `undated` | The evidence carries no clock at all. | A legal, non-error value. |

**Unknown timestamps never default to recent.**
`/proc/meminfo`, `PROCRANK`, `IOTOP`, and `top` print **no clock**; never invent one.

**Threshold: 3,600 s (1 hour).**
Fixed in this specification, not configurable per run, so artifacts are comparable.
The coarse threshold separates a 64.662 s log window from 126-day-old evidence.
One hour is the coarsest value that still separates "same boot" from "a previous boot" on a device with a plausible uptime.

**Cross-boot escalation is mandatory, not advisory.**
When `cross_boot_reference` is non-null it is a monotonic millisecond value from a boot epoch that is not the one `capture.uptime_at_capture_seconds` describes.
If `cross_boot_reference > 1000 × max(uptime_at_capture_seconds, 1)`, the evidence is from a different boot and **must** be forced to `stale` regardless of wall-clock age.
Reference archive: `cross_boot_reference = 31190738` (8 h 39 m 50 s) against `uptime_at_capture_seconds = 30` — a factor of about 1,040 after converting both values to milliseconds — so `stale` twice over.
The `freshness_basis` for such a finding must contain the phrase `cross-boot reference`.

`freshness_basis` must name three things: the `input` (which clock or check was used), the `basis` (why that input decides it), and the `threshold_seconds`.

**Three invariants, all mechanically checkable:**

- `freshness ∈ {now, in_window}` ⟹ `captured_at != null` **and** `age_seconds_at_capture != null` **and** `age_seconds_at_capture < 3600` **and** `scope.truncated == false`.
- `freshness == "stale"` ⟹ the known evidence clock is outside the log window, **or** `age_seconds_at_capture >= 3600`, **or** the cross-boot check fired.
- `freshness == "undated"` ⟹ `captured_at == null` **and** `age_seconds_at_capture == null`.

**A `stale` or `undated` finding is labelled, never dropped, and never silently demoted.**
Severity is capped at `medium` for both, and the uncapped value is always retained in `unconstrained_severity`.

### 2.6 Severity

A closed five-level enum, plus one mandatory sibling:

```text
critical | high | medium | low | info
```

| Level | Ordinal | Definition |
| --- | --- | --- |
| `critical` | 4 | The device needs intervention now: a crash loop still running at capture, a kernel panic or OOM, a `system_server` restart, total loss of a core service. |
| `high` | 3 | A confirmed defect with bounded blast radius: a completed crash, a confirmed ANR, sustained memory pressure above a kernel PSI threshold, an lmkd kill of a foreground app. |
| `medium` | 2 | A real condition worth acting on but not itself a failure: sustained monitor contention, high CPU share, an OOM-adjacency trend — **or a stale/undated artifact severe enough to mislead a reader.** |
| `low` | 1 | Informational but actionable: single-instance contention below the medium bar, expected process churn. |
| `info` | 0 | A statement of fact about the archive or its absence. **Never carries a recommendation.** |

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `severity` | enum | yes | The reported severity. |
| `unconstrained_severity` | enum | yes | The severity this finding would have had absent the freshness cap. Equals `severity` when no cap applied. **Always present**, so the cap is visible and never invisible. |

**The cap.**
A finding whose `freshness` is `stale` or `undated` **cannot exceed `medium`**.
A 126-day-old ANR establishes historical failure, not present failure.
Retain the uncapped value in `unconstrained_severity`.

Two traps are encoded as rules:

1. **A raw tag count is never a severity input.**
   `watchdog` matched 315 times and yields 6 findings; the number 315 is not a severity.
2. **A sample is not stale merely because its window ended before collection.**
   Determine freshness from its actual interval; an old background sample still receives the shared cap.

`severity` is always accompanied by the numeric measurement that justified it, in `metrics`.
A severity with no measurement is a schema violation.

**Each artifact must implement its stated severity rule**; prose definitions alone are not machine mappings.
The [generation rules](12-generation-rules.md#closed-finding-kinds-and-severity-rules) supply the shared explicit `severity_rule` table for §4.2–§4.7: a closed set of machine-checkable conditions over the artifact's own fields, each naming the metrics that satisfy it.
A finding's `severity` is the highest level whose rule fires.
Consequences:

- A finding that matches **no** rule is a schema defect, not an `info`.
  It must be raised against the rule table, not emitted with a default severity.
- `info` is reachable **only** by a rule that states what fact about the archive or its absence is being reported — never as a fallback.

`severity` and `confidence` are the only two judgment fields any artifact carries.
`title` is not one of them: it is narrative, it is generated in `README.md` §4.1, and no JSON artifact carries it.

### 2.7 Evidence

Read-only ZIP evidence is a **locator** plus bounded excerpt, never a source copy.

```text
evidence = {
  archive_path,
  kind,                    // line_range | file_entry | section | property | record
  locator: { line_start, line_end, byte_offset, byte_length },
  excerpt,                 // bounded, default 400 chars
  excerpt_truncated,       // bool
  excerpt_bytes,
  excerpt_complete         // bool
}
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `archive_path` | string | yes | ZIP entry path, exact. Use `<main_entry>` for the main `.txt`. |
| `kind` | enum | yes | `line_range` (line-bounded in a text entry), `file_entry` (a whole archive entry), `section` (a whole `------ … ------` section), `property` (a value in `SYSTEM PROPERTIES`), `record` (a structured record in another artifact). |
| `locator.line_start` / `line_end` | integer \| null | yes | 1-based inclusive. Required for `kind == "line_range"` **and** `kind == "section"` — a `------ … ------` section has a definite span in the file. `null` for `file_entry`, `property`, and `record`, which are not line-bounded. |
| `locator.byte_offset` | integer \| null | yes | Uncompressed byte offset of `line_start` within the entry, so a consumer can seek without re-scanning. |
| `locator.byte_length` | integer \| null | yes | Uncompressed byte length of the located span. |
| `excerpt` | string | yes | Verbatim text. Never re-wrapped, never reflowed, never ellipsised inside. |
| `excerpt_truncated` | boolean | yes | `true` if the excerpt was cut. |
| `excerpt_bytes` | integer | yes | Actual length of `excerpt` in bytes, not characters. |
| `excerpt_complete` | boolean | yes | `true` only when the **entire** located span fits under the bound. |

**`MAX_EXCERPT_BYTES = 512`, hard.**
Rationale:

- It fits required proof snippets.
  Measured: the longest section header is 74 bytes; a `DUMP OF SERVICE` header is 80; a typical `am_crash` event line is ~140; a tombstone `signal N (SIGABRT), code …` line is ~80.
- It bounds hostile input: one line has **192,666 characters**; the **main entry has 4 NUL bytes** (other entries also contain NULs).
  The 512-byte cap bounds worst-case excerpt cost.
- It is roughly four lines of wrapped Markdown, so `README.md` stays readable.

Truncation is byte-wise at 512, then backed up to a UTF-8 boundary — never split a multi-byte sequence.
If backing up would fall below 480 bytes, `excerpt` becomes `""` with `excerpt_truncated: true` and `excerpt_complete: false`.
A `byte_length` above 512 does not invalidate a finding; it only makes the excerpt partial.

A finding whose span contains a NUL byte must set `excerpt_complete: false` and emit `notes` entry `NUL_BYTES_IN_EVIDENCE`, because the consumer's terminal may truncate at the NUL.

#### 2.7 `corroborating_evidence`

Required array, possibly empty, with the same object shape for corroborating bytes found in a second place.

`FS/data/anr/anr_2026-05-20-05-13-42-314` exists both as a ZIP entry and inlined into the main `.txt` under `------ VM TRACES AT LAST ANR (/data/anr/anr_2026-05-20-05-13-42-314: 2026-05-20 05:13:43) ------` (archive line 309986).
The duplication is deliberate in AOSP: `dumpstate.cpp:1182-1189` dumps `anr_data_[0]` into the body *and* adds the whole range to the zip as `HISTORICAL ANR`.
The sort at `dumpstate.cpp:432-435` is newest-mtime-first, which is why `:1183` can say *"The 'last' ANR will always be present in the body."*

Populate **both** locators for the newest ANR.
**A consumer that counts findings must not double-count: `findings` is the counting unit; `evidence` is not.**

### 2.8 Identity

```text
process_key = "<boot_epoch>/<start_seq>"                        // primary
            | "<process_name>#<uid>@<start_ordinal>"           // fallback, start_seq absent
```

| Form | When | Why |
| --- | --- | --- |
| `<boot_epoch>/<start_seq>` | Proposed form, usable only within one demonstrated AMS lifetime. | `ProcessList.java:463` initializes the counter to zero; `:2156` increments it. A `system_server` restart resets it without resetting kernel uptime or the boot instant, so this pair alone is not globally unique. |
| `<process_name>#<uid>@<start_ordinal>` | Proposed fallback when the sequence is unavailable. | Window-local; an unknown ordinal cannot identify an exact process instance. |

`boot_epoch` is `system.json#/capture/boot/at`, the derived device boot instant.
It does **not** identify a `system_server` lifetime.
Cross-restart joins require an AMS-lifetime component or evidenced process-start discriminator.
Until resolved, join only within a demonstrated AMS lifetime; record ambiguity in `evidence_gaps`, never derive lifetime from boot clocks.
`start_seq` is printed as `startSeq=` (`ProcessRecord.java:540`).

**`pid` is an observation attribute, never a key.**
PIDs are recycled.
`pid` without `process_key` is an incomplete observation.

Corroborating identities may be carried but must never be joined on:

| Field | Source | Why it is weaker |
| --- | --- | --- |
| `process_record_id` | `ProcessRecord{<identityHashCode> <pid>:<processName>/<uid>}` — `ProcessRecord.java:1396-1408`. 948 occurrences in the reference archive. | `identityHashCode` is a JVM object hash: not guaranteed unique, and meaningless across a `system_server` restart. |
| `start_elapsed_time` | `startElapsedTime=` — `ProcessRecord.java:527-528`. | Rendered relative to the dump instant, e.g. `-34s559ms`. Not absolute. |
| `app_id` | `Settings.java:5129`. | Stable per (package, user) for the life of an install, but **reassigned on reinstall** and **shared** — every `android.uid.system` app has `app_id = 1000`. |
| `record_identity_hash` | `ProcessRecord.java:1402-1403`. | Diagnostic only. Interestingly it *is* stable across a whole crash loop for one process, which is a useful cross-check that AOSP reuses the record. |
| `pid` | everywhere | Recycled. Display only. |

**`package_name` and `process_name` are different keys and must not be conflated.**
A package can host `:remote` and `:sandboxed_process0` children.
When a `process_name` is not a package name, `package_name` is `null` and the component is carried in `attributes`.

### 2.9 Null, absent, and raw

Six archive-driven rules:

1. **Unknown is `null`.**
   Keys are never omitted.
   The string `"unknown"` is never a value.
2. **Tri-state for build-type-dependent fields.**
   `null` means *not observable*, never `false`.
   Missing emulator properties do not establish a physical device.
   A full `user` report can include `getprop`; assign `skipped_user_build` only when report mode and actual skip evidence support it.
   See [build/report modes](03-build-variants.md#32-gated-sections).
3. **Byte-preserving raw values.**
   Any locale- or build-dependent formatted string is stored as `*_raw` (string) plus a best-effort parsed sibling plus `parsed_confidence`.
4. **Every derived number carries its derivation — unless the formula is already in the schema.**
   Schemas must state computed-value formulas; the formula is the derivation.
   Emit `*_derivation` only when the field table cannot reproduce the formula, e.g. for inputs absent from the artifact.
   Do not repeat formulas or invent interpretations beyond them.
5. **Zero counts carry a match block, never an excuse.**
   A count of `0` is published with its `pattern`, `coverage`, and `regions[]` per [§2.3](01-envelope.md#23-counts-are-match-blocks), and nothing else.
   **No artifact carries an `*_absence_reason` key.**
   Zero counts cannot establish platform omission, build suppression or analyzer failure.
   Known analyzer failures belong in generator diagnostics, never artifacts.
   A tool that has not finished looking has not produced a finding.
6. **Cross-artifact references are pointers, not copies.**
   `capture_ref.source: "system.json#/capture"` points at the canonical object.
   The same derived value must not appear twice with two slightly different answers.

Rule 1 example:
`ro.bootloader` and `gsm.version.baseband` render as the literal string `(unknown)` in the header (`dumpstate.cpp:797-798`, whose defaults are literally `"(unknown)"` at `:795-800`).
**That AOSP placeholder is a device-side sentinel and must be normalised to JSON `null`** at the `source.device` boundary.
The reference archive's `radio` is `null` for exactly this reason.

`provenance.captured_at: null` and `source.device.radio: null` both mean absent evidence, itself a finding input.

### 2.10 The two section-parsing traps

Both traps were measured in the reference archive.

#### Trap 1 — the two `SYSTEM LOG` sections overlap by 184 lines

`dumpstate.cpp:1910` runs `DoLogcat()` early (*"Capture first logcat early on; useful to take a snapshot before dumpstate logs take over the buffer"*, `:1908-1909`), records `logcat_ts` at `:1912`, then runs `DoSystemLogcat(logcat_ts)` at `:2006` — emitted last, narrowed by `-T <logcat_ts>` at `:1004-1012`.

In the reference archive the first section ends at `00:27:20.131` and the `-T`-narrowed tail starts at `00:27:20.000`.
A line-set comparison over that window finds **184 lines present in both** (verified: 184 lines matching `^09-24 00:27:20\.0` in each block, and all 184 are in the intersection).

**De-duplication is mandatory.**
The rule: scan both sections, key on `(section_title, section_ordinal)`, and de-duplicate on `(timestamp, pid, tid, tag, message)`.
A parser that concatenates them double-counts anything in that ~131 ms window.

A title is never a key.
`SYSTEM LOG` appears at archive line **9116** (ordinal 0, the early snapshot) and **653373** (ordinal 1, the `-T` tail).

The second section's header reads `------ SYSTEM LOG (logcat … -T <capture_date> 00:27:20.000) ------`.
That timestamp is `logcat_ts` — the **start of the requested window**, not the time of any line.
It must not be used as a `captured_at`.

#### Trap 2 — five time bases coexist in one archive

| # | Base | Format | Source | Carries a date? |
| --- | --- | --- | --- | --- |
| 1 | Wall clock, second precision, `CLOCK_REALTIME` at capture | `== dumpstate: %Y-%m-%d %H:%M:%S` | `dumpstate.cpp:802` from `now_ = time(nullptr)` (`:3805`) | yes; second, **no offset**, no sub-second |
| 2 | Logcat wall clock, local time, **year-less** | `MM-DD HH:MM:SS.mmm` | `logprint.cpp:1464` — `&"%Y-%m-%d %H:%M:%S"[p_format->year_output ? 0 : 3]` slices off the year; `:1472` appends `.%03ld` | month-day only; **no year, no timezone** |
| 3 | Monotonic since boot, nanosecond | `%.3fs` durations | `dumpstate.cpp:3827`, `(Nanotime() - started_)` | **no absolute date at all** |
| 4 | Process-relative monotonic, seconds | `Process uptime: %ds` | `tombstone_proto.cpp:857-863`, rendered `tombstone_proto_to_text.cpp:616-617` | **no absolute date** |
| 5 | File mtime, second precision | `%Y-%m-%d %H:%M:%S` in file-section headers; `%Y/%m/%d %H:%M:%S` in ZIP entry metadata | `DumpstateInternal.cpp:148`; `dumpstate.cpp:867-868` + `:4663-4669` | yes; second, no offset |

Plus two that are *not* capture times but are often mistaken for them:

- **`Uptime: up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: …`** — `dumpstate.cpp:828-830` runs `uptime -p`.
  The dedicated `------ UPTIME (uptime) ------` section is `dumpstate.cpp:1687`.
  This is base 3 formatted as a duration, **so a naive `MM-DD` parser will not see it**.
- **`----- dumping pid: 17511 at 31190738 -----`** — `StackTracesDumpHelper.java:599-602`, `"SystemClock.uptimeMillis()"`.
  That is **8 h 39 m 50.738 s of uptime on a boot that ended 126 days ago**, against 30 s of uptime on the boot that produced the bugreport.
  `cross_boot_reference` carries this proof that ANR payload predates the current boot.

`captured_at_time_base` exists to make all five individually nameable.

Measured ages of every non-live source in the reference archive, against `capture.captured_at = <capture_date>T00:27:17+07:00`:

| Source | Capture instant | Age at capture |
| --- | --- | --- |
| Log window | `00:26:47.418` → `00:27:52.080` (base 2) | 0–65 s |
| `VM TRACES JUST NOW` | `<capture_date> 00:27:25` (base 5) | 8 s after collection start; post-start evidence |
| `RT_TABLES` | `<capture_date> 00:27:10` (base 5) | 7 s |
| `BINDER *` (5 sections) | `<capture_date> 00:26:52` (base 5) | 25 s |
| **ANR files (newest)** | `2026-05-20 05:13:43` (base 5) | **126 d 19 h 13 m 34 s** |
| **ANR files (oldest)** | `2026-05-20 00:54:11` (base 5) | **126 d 23 h 33 m 06 s** |
| **Tombstones (all 50)** | `2026-05-20 04:00:14` (base 5) | **126 d 20 h 27 m 03 s** |
| **`* BUILD-TIME RELEASE FLAGS`** | `2026-01-27 20:47:21` (base 5) | **239 d 3 h 39 m 56 s** |
| `dumpsys procstats` `AGGREGATED OVER …` | `2026-05-19 00:16:06` (base 5) | **~128 d** |

The ANR span is **4 h 19 m 30.631 s** (oldest `anr_2026-05-20-00-54-11-683` → newest `anr_2026-05-20-05-13-42-314`; 65 files, `total 20336` KB per the `ANR FILES` listing at archive line 318024).
Three independent ANR clocks: filename, ZIP mtime and `ls -lt` column.

**Rule:** per-finding freshness needs explicit basis; artifact disclaimers cannot replace it.

### 2.11 Units and time formats

- **Every numeric field carries a unit suffix** in its key name, from a closed list: `_bytes`, `_kb`, `_mb`, `_ms`, `_us`, `_ns`, `_seconds`, `_pages`, `_count`, `_percent`, `_cycles`, `_hz`, `_kib`, `_mib`, `_hundredths`.
- **`_kb`/`_mb` are decimal (10³/10⁶); `_kib`/`_mib` are binary (2¹⁰/2²⁰).**
  Never write `_kb` for what the kernel means by KiB.
- **Identifiers, ordinals and versions** are dimensionless metadata; physical measurements require units.
  Counts use `_count` and mean occurrences where that field names a count.
- **Numbers parsed from human-formatted text keep the source's unit and say so.**
  AOSP's `Total PSS by process:` block prints `212,862K: …` (archive line 325705); that `K` is 1024-based, so it is recorded as `pss_kib`, not `pss_kib`.
  Do not silently convert; do not silently relabel.
- **Percentages** are numeric percent values, preserving source precision; `_percent` and existing `_pct` keys use 100 for one whole.
  A multicore top row can exceed 100.
  Ratios use `_ratio` and are nonnegative; specify each ratio's denominator and range (compression ratios can exceed 1).
  Never silently treat a fraction as a percentage.
- **All timestamps are RFC 3339 with an explicit numeric offset**, e.g. `2000-01-01T05:13:43+07:00`, `2000-01-02T00:27:01.455+07:00` (synthetic examples).
  **Never a bare local string.**
- **Sub-second precision is preserved when the source has it.**
  Logcat: milliseconds.
  `----- pid N at <ts> -----`: nanoseconds (`dumpstate.cpp:2222`).
  ZIP mtime: whole seconds.
  Do not pad or round.
- **Relative times use a distinct suffix and a named anchor:** `age_seconds_at_capture` (anchor: `capture.captured_at`), `duration_seconds` (anchor: the enclosing `------ N.NNNs was the duration of '…' ------` trailer), `process_uptime_seconds` (anchor: process start).
- **Time bases are never mixed in one comparison.**
  A year-less logcat wall clock is not comparable to a monotonic millisecond counter.
  A finding that would need that comparison uses `cross_boot_reference` and is capped at `medium`.
- **`localtime()` is a bug source.**
  `dumpstate.cpp:802` and `:2744`, and `DumpstateInternal.cpp:148`, all use `localtime()`.
  The device's zone is whatever the device had.
  The reference archive shows `+0700` in the `----- pid N at … -----` lines and `00:27:17` in the banner: **the offset is only recoverable from the per-process lines** (`dumpstate.cpp:2227`, `strftime("%z", &tm)`).
  A finding whose time is known only from a banner or an mtime header has no offset in the source — resolve it from `capture.captured_at`'s offset, and if that is `null`, emit `captured_at: null`.

