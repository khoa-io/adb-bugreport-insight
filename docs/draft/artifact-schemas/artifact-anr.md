# `anr.json` — keyed on the incident

> **Draft.** See [`index.md`](./index.md). Requirement: `DRAFT-REQ-ART-05`.

Original §4.4 of `artifact-schemas.md`. Reports ANR *evidence* in the archive and the ANR *incidents*
that evidence supports; those are frequently not the same set, and 65 retained trace files are not 65
ANRs.#

### 4.4 `anr.json`

#### Purpose

`anr.json` answers: **did an Application Not Responding happen, and if so when — and what
thread dumps exist, of what age, and from which process?** It deliberately does *not* answer
"is the device responsive now", because a bugreport cannot answer that. The archive is a
collection of sections captured at different instants under different retention rules, and a
finding that does not carry its own evidence clock is a finding that will eventually lie.

**The six ANR evidence channels are different things with different clocks; merging them is
the bug this artifact exists to prevent.**

| # | Channel | Emitted by | Bugreport section | Time base | Timestamped? |
| --- | --- | --- | --- | --- | --- |
| C1 | Live logcat `E ActivityManager: ANR in <pkg>` | `ProcessErrorStateRecord.java:445-461`, logged at `:563` | `SYSTEM LOG` | wall clock, per line | **yes** — `MM-DD HH:MM:SS.mmm` |
| C2 | ANR file inventory | `dumpstate.cpp:1204`, `RunCommand("ANR FILES", {"ls","-lt",ANR_DIR})` | `ANR FILES` | file mtime | **yes** — minute precision |
| C3 | Newest ANR thread dump | `dumpstate.cpp:1184-1185`; data from `GetDumpFds(ANR_DIR,"anr_")` `:395-436`, sorted **descending by mtime** `:432-434` | `VM TRACES AT LAST ANR` | per-process wall clock, ns + tz | **yes, twice** — section header mtime *and* `----- pid N at <ts> -----` per process |
| C4 | `dumpsys … lastanr` | `ActivityTaskManagerService.java:4254-4261`, `dumpLastANRLocked` | `DUMPSYS` | **none** | **no** — the state carries no clock |
| C5 | Dropbox `*_anr` | `AMS:9766`; `processClass` at `AMS:9714-9722` | `dumpsys dropbox -p <tag>` — but dumpstate requests **only** `-p system_server_crash` and `-p system_app_crash` (`dumpstate.cpp:1858-1859`) | wall clock in `Timestamp:` | **yes** |
| C6 | Event log `am_anr` | `ProcessErrorStateRecord.java:374`; layout `EventLogTags.logtags:17` | `EVENT LOG` | wall clock, per entry | **yes** — but **buffer-wrapped and lossy** |

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | One entry per channel, with presence, count block reference, completeness, and provenance. |
| `anr_incidents` | array of object | yes | **One entry per `appNotResponding` event.** Empty is a result, not an absence. |
| `trace_files` | object | yes | The C2/C3 disk evidence. **Never an incident by itself.** |
| `last_anr_trace` | object | yes | The C3 parse of `VM TRACES AT LAST ANR`. |

**`anr_incidents` is keyed by incident, not by process and not by file.** Three candidates
were considered:

- **By file** — wrong. `/data/anr` is a ring of up to 64 retained dumps
  (`StackTracesDumpHelper.java:516`, `tombstoned.cpp:137-141`). Files are evidence carriers,
  reused and overwritten; one incident yields exactly one file. Keying by file makes the
  incident count a function of a retention constant.
- **By process** — wrong for the common case. The archived trace holds **31 processes** but
  the ANR had **one** subject; the other 30 are bystanders dumped for context
  (`ProcessErrorStateRecord.java:403-435` adds parent, `MY_PID`, persistent, and IME
  processes). Keying by process would manufacture 31 ANRs from one.
- **By incident** — correct. The unit is one `appNotResponding` event, which has a wall-clock
  instant and a `Reason:`, and appears in C1, C5, and C6 with the same `pid` and process name.
  It is the only key stable across channels, unaffected by retention, and it is what a human
  means by "an ANR".

Files and processes become **child arrays of the incident**, each with its own provenance.

#### The anti-staleness rule

> **An `anr.json` object may only be classified as current if its evidence time lies inside
> the archive's `capture.log_window`.** Every object carries `captured_at` +
> `captured_at_time_base` + `age_seconds_at_capture` + `freshness`. An object with no derivable
> time is `freshness: "undated"` and is **never promoted to an incident**.

The rule is *structural*, not advisory: `freshness` is required, its enum has no
"default = recent" member, and `anr_incidents` entries are only ever constructed from C1/C5/C6
records with a wall clock inside the window. C2 and C3 can populate `trace_files` and
`last_anr_trace` but **cannot create an `anr_incidents` entry** unless the derived time is
in-window — in which case they become corroborating evidence on that incident, never the
incident itself.

#### Field tables

##### `channels[]`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `channel` | enum | yes | A descriptive channel name, **not** a `C1`…`C6` label. Those identifiers were an artefact of the original research pass and mean nothing to a consumer who has not read it. Use `logcat_anr_tag`, `anr_file_listing`, `vm_traces_at_last_anr`, `dumpsys_activity_last_anr`, `dumpsys_windowmanager_last_anr`, `dropbox_anr`, `event_log_am_anr`. |
| `emitted_by` | **Removed.** | — | An AOSP `path:line` string is not a per-archive observation: no archive names the component that emitted a line. The spec cites the emitter in prose where the channel is introduced; the artifact does not repeat it. |
| `emitter_kind` | enum | yes | **Replaces `emitted_by`, and is machine-decidable from the bytes alone:** `logcat_tag`, `event_log_tag`, `dumpsys_section`, `zip_entry_listing`, `zip_inlined_section`. The first is a substring search, the second a bracketed event-log line, the third a `DUMP OF SERVICE` section, the last two ZIP-entry-sourced. |
| `present` | boolean | yes | The section existed and was non-empty. |
| `counts_ref` | string \| null | yes | The key in `counts` holding this channel's match block. `null` when the channel carries no match block — and where the reference names a key that does not exist, that is a defect, not a value. |
| `completeness` | **Removed.** | — | `complete` / `lossy` / `unknown` is a judgment about a collector the tool cannot inspect: nothing in a read-only archive records whether a pass dropped rows. **`window_start`, `window_end`, and `coverage_ratio` — already present in the `counts[].*_hits.coverage` block — are the machine-readable form of the same question.** |
| `rate_limit_per_tag` / `rate_limit_source` | **Removed.** | — | A hardcoded platform constant. Worse, the reference example attached `rate_limit_per_tag: 6` to a channel marked `present: false` — a constant about a channel that does not exist. The *observed* effect is already visible in the match block's `raw_count`. |
| `decode_warning` | **Removed.** | — | Replaced by an `evidence_gaps[]` entry. A decode warning is either "the bytes do not determine this", which is a gap, or it is prose about an AOSP writer the tool never read. It is never a data field. |
| `provenance` | object | yes | See [§2.4](./conventions-evidence.md#24-provenance). |

##### `anr_incidents[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `incident_id` | string | yes | Stable id over `(channel, wall_clock, pid, process)`. | derived |
| `process` | string | yes | Offending process name. | `ProcessErrorStateRecord.java:446` |
| `pid` | integer | yes | Offending pid. | `ProcessErrorStateRecord.java:449`; `am_anr` field 2, `logtags:17` |
| `user_id` | integer \| null | yes | Read from the event-log payload when the slot layout is decodable, `null` otherwise. The slot order is declared in the platform's tag file, which is **not in the archive**, so a tool must record an `evidence_gaps[]` entry rather than guess — see `am_anr` below. | `logtags:17` field 1; `ProcessErrorStateRecord.java:374` |
| `user_id_basis` | enum \| null | yes | `decoded_from_payload` or `null`. **Always published.** Distinguishes a user id that was read from one that is unavailable, so a consumer never has to guess why the field is `null`. | — |
| `occurred_at` | string | yes | **The only field that can carry a current freshness.** | C1 line time, `logprint.cpp:1552` |
| `activity_component` | string \| null | yes | The `(Activity)` suffix. | `ProcessErrorStateRecord.java:447-449` |
| `reason` | string \| null | yes | Verbatim `Reason:` payload. | `ProcessErrorStateRecord.java:451-453`; origin `InputManagerCallback.java:350-351` |
| `parent_component` | string \| null | yes | | `ProcessErrorStateRecord.java:454-457` |
| `error_id` | string \| null | yes | | `ProcessErrorStateRecord.java:458-460` |
| `frozen` | boolean \| null | yes | | `ProcessErrorStateRecord.java:461` |
| `app_flags_hex` | string \| null | yes | `ApplicationInfo.flags`. | `ProcessErrorStateRecord.java:374` arg 4 |
| `processes[]` | array of object | yes | Subject plus bystanders from the trace. | `ProcessErrorStateRecord.java:403-435` |
| `trace_files[]` | array of string | yes | `anr_` files attributable to this incident. | `dumpstate.cpp:395-436` |
| `evidence_channels[]` | array of object | yes | Which channels carry this incident. **Each entry is `{channel, join_basis}`** where `join_basis` ∈ `same_logcat_line`, `same_pid_and_window`, or `heuristic`. A bare string array claimed the channels agreed on the same incident; the reference artifact joined a logcat line, an event-log entry, and a dropbox record on `(pid, ~10 ms)` with **no recorded linkage key** anywhere in the archive. `heuristic` is honest about that and forces `confidence: "inferred"`. |
| `provenance` | object | yes | | — |

> **`am_anr` slot decoding is not implementable from the archive alone.** The event-log payload
> is positional, and its field order is declared in the platform's `logtags` file — which a
> read-only analyzer never sees. The reference archive contains `am_anr` entries but the spec
> itself records that the corresponding emitter was never located. Rather than assert a slot
> order, a tool that cannot confirm it emits `user_id: null`, `user_id_basis: null`, and an
> `evidence_gaps[]` entry with `reason: UNVERIFIABLE`.

`processes[]` entries:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer | yes | | `backtrace.cpp:49` |
| `cmdline` | string \| null | yes | | `backtrace.cpp:52`; `libdebuggerd_client.cpp:109` |
| `is_subject` | **Removed**, replaced by `subject_inference`. | — | The predicate is machine-checkable — `----- dumping pid:` precedes the block — but the *name* asserted blame. The reference trace carries **no** `Subject:` header at all, and the artifact still published `is_subject: true` beside `confidence: "confirmed"`. What is knowable is which process the dumper led with. |
| `subject_inference` | object | yes | `{basis, subject_pid, subject_inferred}` where `basis` ∈ `subject_header` (a `Subject:` line was read — the only case where a subject is a *fact`) or `dumping_pid_marker` (the dumper's first `----- dumping pid:`, which identifies a process but **not** a culprit). `subject_inferred: true` whenever `basis != "subject_header"`, and any finding resting on it must carry `confidence: "inferred"`, never `confirmed`. |
| `stamped_at` | string | yes | Per-process nanosecond wall clock. | `backtrace.cpp:49` |
| `java_thread_count` | integer \| null | yes | Parsed from `DALVIK THREADS (n):`. | ART thread dump |
| `native_frame_count` | integer \| null | yes | `    #NN pc` lines. | `system/unwinding/libunwindstack` |
| `waiter_count` | integer \| null | yes | `- waiting to lock` lines. | `backtrace.cpp` thread dump |

**There is deliberately no `timeout_ms` field.** `DEFAULT_ANR_TIMEOUT` and
`DEFAULT_INPUT_ANR_TIMEOUT` do not exist anywhere in this `android-15.0.0_r17` tree — they were
removed from `ActivityManagerConstants`. The input-dispatch timeout is a *reason string only*
(`InputManagerCallback.java:350-351` — `"Input dispatching timed out."`); the duration is not
written into the `Reason:` line. The FGS delays that *do* exist are
`DEFAULT_SERVICE_START_FOREGROUND_ANR_DELAY_MS = 10_000` and `short_fgs_anr_extra_wait_duration`
default 10 s (`ActivityManagerConstants.java:303,860-861,1135-1143`), but those are
device-configurable and not the timeout the incident actually tripped. **A field that cannot be
sourced does not go in the spec.**

##### `trace_files`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `count` | integer | yes | `anr_` file count in the listing. | `dumpstate.cpp:203` `ANR_DIR` |
| `listing_total_kb_as_printed` | integer | yes | The `total` line `ls -lt` prints. **An allocated-block figure, not a byte total.** | `dumpstate.cpp:1204` |
| `total_bytes` | integer | yes | The sum of the printed per-file size column. **Exact**, and therefore needs no `_derivation` sibling per [§2.9](./conventions-evidence.md#29-null-absent-and-raw) rule 4. The previous `total_bytes_approx` derived from the `ls` total was both less exact than the archive allowed and ~507 KB larger than the true byte total. |
| `oldest_created_at` / `newest_created_at` | string \| null | yes | Min/max `created_at` over `files[]`. | `StackTracesDumpHelper.java:69-70` |
| `span_seconds` | number \| null | yes | `newest − oldest`. `null` when fewer than two files. | derived |
| `all_outside_log_window` | boolean | yes | `true` when no file's `created_at` falls inside `capture.log_window`. | derived |
| `files[]` | array of object | yes | One per file. Below. **All files are emitted**; where an artifact is abbreviated for presentation it carries `files_truncated_in_example`, as `cpu.json`'s contention array does. | — |

`files[]` entries:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `path` | string | yes | ZIP entry name. | `dumpstate.cpp:203`; zip path `dumpstate.cpp:462` |
| `file_name` | string | yes | Basename. | `StackTracesDumpHelper.java:454-457` |
| `size_bytes` | integer | yes | From the `ls -l` size column. | `dumpstate.cpp:1204` |
| `mtime_local` | string | yes | `ls` mtime, **minute granularity**. | `dumpstate.cpp:1204` |
| `created_at` | string | yes | **Millisecond** creation time decoded from the `anr_` name. | `StackTracesDumpHelper.java:69-70`, `:455` |
| `is_newest` | boolean | yes | `anr_data_[0]` — the one dumped into the main text. | `dumpstate.cpp:432-434`, `:1184` |
| `mtime_ls_minute_differs_from_filename` | boolean | yes | **`true` when the minute-granular `ls` mtime and the second-precision filename disagree.** This replaces `time_conflict`, which was named as a clock-change detector but could not be one: both values are written by the same device clock, so a clock change shifts both together and the flag would essentially never fire. Under the new name the field makes no causal claim — it reports the disagreement and leaves the cause open. | derived |
| `provenance` | object | yes | | — |

##### `last_anr_trace`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `section_header` | string | yes | The literal header line, verbatim. | `DumpstateInternal.cpp:138-152` |
| `source_path` | string | yes | Path parsed out of the header. | same |
| `file_mtime_local` | string | yes | From the header. | `DumpstateInternal.cpp:146-150` |
| `trace_first_stamp` / `trace_last_stamp` | string | yes | Min/max `----- pid N at`. | `backtrace.cpp:49` |
| `process_count` | integer | yes | `----- pid N at` block count. |
| `is_culprit_trace` | **Removed**, replaced by `has_dumping_pid_marker`. | — | Same defect as `is_subject`: the predicate passes, the *name* asserts blame. "Culprit" is a conclusion about which component was responsible for a hang, and a trace that opens with `----- dumping pid:` says only that the dumper started there. |
| `has_dumping_pid_marker` | boolean | yes | `true` when the first process block is preceded by `----- dumping pid:`. **A statement about the file's layout, not about blame.** Any finding that uses it to attribute an ANR must carry `confidence: "inferred"`. | `StackTracesDumpHelper.java:599-602` |
| `uptime_ms_at_dump` | integer \| null | yes | **Uptime, not wall clock** — and therefore a `cross_boot_reference`. | `StackTracesDumpHelper.java:599-602` |
| `processes[]` | array of object | yes | As above, with `stamped_at` required. | `backtrace.cpp:47-95` |
| `lock_contention_markers` | object | yes | `{waiting_to_lock, held_by_thread}`. | — |
| `provenance` | object | yes | `cross_boot_reference` populated here. | — |

#### Provenance and freshness

This artifact is the reason the whole convention exists. On the reference archive:

| Clock | Where it is read |
| --- | --- |
| File creation (the ANR event) | the millisecond timestamp encoded in the `anr_` filename |
| File mtime | the file-section header stamp |
| First process stamp | the first `----- pid N at` header inside the trace |
| Last process stamp | the last `----- pid N at` header inside the trace |
| Archive captured | the `== dumpstate:` banner (`system.json#/capture/captured_at`) |
| **Age at capture** | derived from the above |
| Cross-boot reference | the `uptime_ms_at_dump` value in the trace's first marker |

**Three clocks that disagree with each other are the finding, not a defect to be reconciled.**
The filename, the section-header mtime and the in-trace process stamps are all written by the
same device clock but at different instants and precisions. The artifact publishes all three
rather than picking a favourite, and `mtime_ls_minute_differs_from_filename` reports any
disagreement without asserting which one is right.

Meanwhile **this boot had zero ANRs**, provable three independent ways:

1. The `ANR in ` logcat tag has **0** occurrences archive-wide.
2. The `am_anr` event-log tag has **0** entries.
3. Two dumpsys sections assert it in so many words — the window-manager last-ANR and the
   activity last-ANR sections both print `  <no ANR has occurred since boot>`.

…while the `ANR FILES` listing at archive line **318023** shows 65 files:

```
------ ANR FILES (ls -lt /data/anr/) ------
total <listing_total_kb>
-rw------- 1 system system <size_bytes> <YYYY-MM-DD> <HH:MM> anr_<YYYY-MM-DD-HH-mm-ss-SSS>
...
------ <duration>s was the duration of 'ANR FILES' ------
```

**The listing is the whole story, and it is a stale-retained-files story.** The files span a
window of hours, they all predate this boot entirely, and none of them is an incident in this
capture. **The correct artifact states both facts — the files and the absence of incidents — and
lets `negative_evidence` carry the authoritative one.**

Why the files were not pruned: `maybePruneOldTraces`
(`StackTracesDumpHelper.java:512-530`) keeps `min(64, 1 day)` files but **only runs immediately
after a new ANR file is written** (`:183`). The count therefore persists indefinitely whenever
survivors exceed `max + 1` and no new ANR arrives to trigger a prune. **The retention rule is
event-driven, not clock-driven** — which is why `trace_files.count` is published as an
observation and its explanation lives here rather than in the artifact.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "anr",

  "capture_ref": { "source": "system.json#/capture" },

  "counts": {
    "anr_tag_hits": {
      "pattern": "ANR in ", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0,
      "not_emitted_by_platform": 0, "suppressed_noise_note": null,
      "coverage": { "window_start": "2026-09-24T00:26:47.418+07:00",
                    "window_end": "2026-09-24T00:27:52.080+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": []
    },
    "am_anr_hits": {
      "pattern": "am_anr", "pattern_kind": "word_boundary", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0,
      "not_emitted_by_platform": 0, "suppressed_noise_note": null,
      "coverage": { "window_start": "MM-DD HH:MM:SS.mmm",
                    "window_end": "MM-DD HH:MM:SS.mmm",
                    "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
      "regions": []
    },
    "anr_file_listing_hits": {
      "pattern": "^-rw[^ ]+ +[0-9]+ +[^ ]+ +[^ ]+ +[0-9]+ +(\\S{3} +[0-9]+ +[0-9:]+)\\s+(anr_[0-9]{4}-.*)$",
      "pattern_kind": "line_regex", "case_sensitive": true,
      "raw_count": <listing_file_count>, "matched_evidence": <listing_file_count>, "suppressed_noise": 0,
      "not_emitted_by_platform": 0, "suppressed_noise_note": null,
      "coverage": { "window_start": null, "window_end": null,
                    "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "ANR FILES",
                     "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
                     "line_count": <listing_file_count> } ]
    }
  },

  "channels": [
    { "channel": "logcat_anr_tag", "emitter_kind": "logcat_tag", "present": false,
      "counts_ref": "anr_tag_hits" },
    { "channel": "anr_file_listing", "emitter_kind": "zip_entry_listing", "present": true,
      "counts_ref": "anr_file_listing_hits" },
    { "channel": "vm_traces_at_last_anr", "emitter_kind": "zip_inlined_section", "present": true,
      "counts_ref": null },
    { "channel": "dumpsys_activity_last_anr", "emitter_kind": "dumpsys_section", "present": true,
      "counts_ref": null },
    { "channel": "dropbox_anr", "emitter_kind": "zip_entry_listing", "present": false,
      "counts_ref": "dropbox_anr_hits" },
    { "channel": "event_log_am_anr", "emitter_kind": "event_log_tag", "present": true,
      "counts_ref": "am_anr_hits" }
  ],

  "anr_incidents": [],

  "trace_files": {
    "count": <listing_file_count>,
    "listing_total_kb_as_printed": <listing_total_kb>,
    "total_bytes": <sum_of_printed_sizes>,
    "total_bytes_note": "the sum of the printed per-file size column. This is exact, so per §2.9 rule 4 it needs no derivation string. The ls 'total' line is an allocated-block figure and is published separately as listing_total_kb_as_printed rather than summed.",
    "oldest_created_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>",
    "newest_created_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>",
    "span_seconds": <span_seconds>,
    "all_outside_log_window": true,
    "files": [
      { "path": "FS/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
        "file_name": "anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
        "size_bytes": <n>, "mtime_local": "<YYYY-MM-DD>T<HH:MM:00>",
        "created_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>",
        "is_newest": true, "mtime_ls_minute_differs_from_filename": false,
        "provenance": { "evidence_origin": "anr_file_listing", "archive_path": "<main_entry>",
          "section_title": "ANR FILES", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>", "captured_at_time_base": "file_name",
          "evidence_time_source": "filename_timestamp",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "filename timestamp",
            "basis": "the ANR file name encodes creation time; the age at capture follows from it",
            "threshold_seconds": 3600 },
          "collector_perturbed": false,
          "collector_perturbed_basis": "read_from_disk" } },
      { "path": "FS/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
        "file_name": "anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
        "size_bytes": <n>, "mtime_local": "<YYYY-MM-DD>T<HH:MM:00>",
        "created_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>",
        "is_newest": false, "mtime_ls_minute_differs_from_filename": false,
        "provenance": { "section_title": "ANR FILES", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>", "captured_at_time_base": "file_name",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "filename timestamp",
            "basis": "oldest of the retained files, measured against the newest by the filename timestamps",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ]
  },

  "last_anr_trace": {
    "section_header": "------ VM TRACES AT LAST ANR (/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>: <YYYY-MM-DD> HH:MM:SS) ------",
    "source_path": "/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
    "file_mtime_local": "<YYYY-MM-DD>T<HH:MM:SS>",
    "trace_first_stamp": "<YYYY-MM-DD>T<HH:MM:SS.nnnnnnnnn><TZ>",
    "trace_last_stamp": "<YYYY-MM-DD>T<HH:MM:SS.nnnnnnnnn><TZ>",
    "process_count": <n>,
    "has_dumping_pid_marker": true,
    "uptime_ms_at_dump": <uptime_ms>,
    "processes": [
      { "pid": <pid>, "cmdline": "system_server",
        "subject_inference": { "basis": "dumping_pid_marker", "subject_pid": <pid>, "subject_inferred": true },
        "stamped_at": "<YYYY-MM-DD>T<HH:MM:SS.nnnnnnnnn><TZ>",
        "java_thread_count": <n>, "native_frame_count": null, "waiter_count": <n> },
      { "pid": <pid>, "cmdline": "/apex/<vendor.hardware.drm-service>",
        "subject_inference": { "basis": "dumping_pid_marker", "subject_pid": <pid>, "subject_inferred": false },
        "stamped_at": "<YYYY-MM-DD>T<HH:MM:SS.nnnnnnnnn><TZ>",
        "java_thread_count": null, "native_frame_count": <n>, "waiter_count": 0 },
      { "pid": <pid>, "cmdline": "/apex/com.android.hardware.<name>/bin/hw/android.hardware.<name>-service",
        "subject_inference": { "basis": "dumping_pid_marker", "subject_pid": <pid>, "subject_inferred": false },
        "stamped_at": "<YYYY-MM-DD>T<HH:MM:SS.nnnnnnnnn><TZ>",
        "java_thread_count": null, "native_frame_count": null, "waiter_count": null }
    ],
    "lock_contention_markers": { "waiting_to_lock": <n>, "held_by_thread": <n> },
    "provenance": { "evidence_origin": "vm_traces_at_last_anr", "archive_path": "<main_entry>",
      "section_title": "VM TRACES AT LAST ANR", "section_ordinal": 0,
      "line_start": <line>, "line_end": <line>,
      "captured_at": "<YYYY-MM-DD>T<HH:MM:SS><TZ>", "captured_at_time_base": "file_section_header_mtime",
      "evidence_time_source": "embedded_trace_stamp",
      "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
      "cross_boot_reference": <uptime_ms>,
      "freshness": "stale",
      "freshness_basis": { "input": "section header mtime and the per-process '----- pid N at' stamps",
        "basis": "all stamps agree with the header mtime; cross_boot_reference is orders of magnitude larger than the capture uptime, so the trace is from a previous boot",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "read_from_disk" }
  },

  "negative_evidence": [
    { "scope": "dumpsys_activity_last_anr",
      "statement": "  <no ANR has occurred since boot>",
      "source": { "archive_path": "<main_entry>", "section_title": "<dumpsys activity section>",
                  "line_start": <line>, "line_end": <line> },
      "authority": "platform_assertion" },
    { "scope": "dumpsys_windowmanager_last_anr",
      "statement": "  <no ANR has occurred since boot>",
      "source": { "archive_path": "<main_entry>", "section_title": "<critical dumpsys section>",
                  "line_start": <line>, "line_end": <line> },
      "authority": "platform_assertion" },
    { "scope": "logcat_anr_tag",
      "statement": "0 occurrences of the literal 'ANR in ' in the main logcat sections",
      "source": { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG",
                  "section_ordinal": 0, "line_start": <line>, "line_end": <line> },
      "authority": "archive_absence" },
    { "scope": "event_log_am_anr",
      "statement": "0 'am_anr' entries in the event-log section",
      "source": { "archive_path": "<main_entry>", "section_title": "EVENT LOG",
                  "section_ordinal": 0, "line_start": <line>, "line_end": <line> },
      "authority": "archive_absence" }
  ],

  "findings": [
    { "id": "anr-0001", "kind": "anr_absence_with_stale_files",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "anr_incidents_count": <n>, "anr_files_on_disk": <n>,
                      "anr_files_outside_log_window": <n> },
      "metrics": { "newest_anr_file_age_seconds": <age>, "oldest_anr_file_age_seconds": <age>,
                   "anr_file_span_seconds": <span_seconds>, "cross_boot_uptime_ms": <uptime_ms> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
                    "locator": { "line_start": <line>, "line_end": <line> },
                    "excerpt": "------ ANR FILES (ls -lt /data/anr/) ------\ntotal <listing_total_kb>\n-rw------- 1 system system <size_bytes> <YYYY-MM-DD> <HH:MM> anr_<YYYY-MM-DD-HH-mm-ss-SSS>",
                    "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [
        { "archive_path": "FS/data/anr/anr_<YYYY-MM-DD-HH-mm-ss-SSS>", "kind": "file_entry",
          "locator": { "line_start": <line>, "line_end": <line> },
          "excerpt": "----- dumping pid: <pid> at <uptime_ms>\nproc <pid>\n",
          "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": false }
      ],
      "provenance": { "evidence_origin": "anr_file_listing", "archive_path": "<main_entry>",
        "section_title": "ANR FILES", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": "<YYYY-MM-DD>T<HH:MM:SS.mmm><TZ>", "captured_at_time_base": "file_name",
        "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
        "freshness": "stale",
        "freshness_basis": { "input": "filename timestamp + cross-boot reference",
          "basis": "the files predate capture and come from a boot that ended before this one; see last_anr_trace.provenance",
          "threshold_seconds": 3600 },
        "cross_boot_reference": <uptime_ms>, "collector_perturbed": false,
        "collector_perturbed_basis": "read_from_disk" } }
  ],

  "evidence_gaps": [
    { "field": "anr_incidents[]", "reason": "SECTION_ABSENT",
      "detail": "no appNotResponding event in any channel. The array is empty, which per this specification is a result and not an absence — negative_evidence carries the two platform assertions and the two archive searches that support it." },
    { "field": "anr_incidents[].subject", "reason": "UNVERIFIABLE",
      "detail": "the am_anr event-log slot layout would have to be decoded from the platform's tag declaration to read a subject; no such declaration is present in the archive, so no subject is published for any incident." },
    { "field": "dropbox_anr", "reason": "SECTION_ABSENT",
      "detail": "the dropbox ZIP entry is present but holds no ANR-tagged entries. Whether the platform requested ANR tags for this build is not observable from the archive." },
    { "field": "last_anr_trace.processes[].subject_inference", "reason": "UNVERIFIABLE",
      "detail": "the trace carries no Subject: header, so subject_inference is derived from the dumper's first dumping-pid marker. That identifies where the dumper started, not which component was responsible for the hang." },
    { "field": "trace_files.count", "reason": "UNVERIFIABLE",
      "detail": "the retention rule that decides how many ANR files persist is platform behaviour; the tool reports the observed count and does not explain it." }
  ],

  "notes": [
    "ANR_TAG_ABSENT_BUT_FILES_PRESENT: a zero match for the logcat 'ANR in ' tag is not evidence that no ANR occurred; trace files exist on disk from a previous boot. An ANR is read from the ANR FILES listing and from the inlined VM TRACES section, never from a logcat tag count."
  ]
}
```

#### Open questions

- **No ANR subject for the archived trace.** The newest trace's first block is `system_server`,
  preceded by a `----- dumping pid: … at <uptime_ms>` marker, so the trace *appears* to have
  opened on `system_server`. But the trace carries **no `Subject:` header** — the platform
  writes one only when a subject is available, and no subject is recorded for this dump — and
  no `ANR in` line survives from that boot. **`subject_inference` therefore derives from the
  dumping-pid marker only, and is flagged `subject_inferred: true`.** No ANR subject is
  attributed for the stale trace, and no finding resting on it may claim `confidence: "confirmed"`.
- **C5 dropbox ANR reachability.** `dumpstate` requests only `-p system_server_crash` and
  `-p system_app_crash` (`dumpstate.cpp:1858-1859`), so ANR dropbox entries reach the archive
  only as ZIP entries under `FS/data/system/dropbox`, and only on non-`user` builds
  (`dumpstate.cpp:1957-1960`). This archive is `userdebug`, so the directory *should* be
  present — but it contains no `*_anr` files, consistent with zero ANRs. The **`user`-build
  behaviour is not verified**, so C5's completeness is `lossy`, not `complete`.
- **`trace_files` has no approximate total.** `ls -lt` prints an allocated-block `total` in 1K
  blocks, which is not a byte total and is larger than the sum of the per-file size column. The
  artifact therefore publishes `total_bytes` as the exact sum of the printed sizes and keeps
  `listing_total_kb_as_printed` only as the verbatim `ls` figure. No `_derivation` sibling is
  needed: the formula is stated in the field table, per [§2.9](./conventions-evidence.md#29-null-absent-and-raw) rule 4.
- **ANR file timestamps are well-dated.** An earlier claim that the trace carries "no
  timestamp inside" is **refuted**: every process block carries a nanosecond wall clock in its
  `----- pid N at … -----` header. **Staleness here is provable, not inferred.**
- **The `ANR FILES` `ls` column is minute-granular** while the filename is millisecond. The
  `mtime_ls_minute_differs_from_filename` field reports where the two disagree. It makes no
  causal claim — both are written by the same device clock, so it cannot detect a clock change.

---

