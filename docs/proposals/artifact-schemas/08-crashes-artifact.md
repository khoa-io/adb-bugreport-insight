# `crashes.json` — channels kept apart

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Requirement: `REQ-INSIGHT-04`.

Different channels can retain a live crash loop and months-old native crashes in one archive.
Java and native never share a number.

### 4.5 `crashes.json`

#### Purpose

`crashes.json` answers: **what crashed, how often, in what pattern, and how old is the evidence?**
**Seven channels differ in completeness**: one 64 MB file holds a current 271-crash loop and four-month-old 50-file native incident.
The schema must keep them apart and must not let a lossy channel's count stand in for the truth.

#### The channels

| Channel | Emitter kind | Guarantee | What it bounds |
| --- | --- | --- | --- |
| `crash_buffer` (`FATAL EXCEPTION`) | `logcat_tag` | ring buffer, complete until wrap | **the retained Java crash records**, after de-duplication |
| `am_crash_event_log` | `event_log_tag` | ring-wrapped, lossy | a bounded fraction of the loop; the event log's buffer covers well under the whole log window |
| `dropbox_crash` | `dumpsys_section` | rate-limited per `(process, eventType)` | first occurrences only; the rate limit is not published by any archive |
| `tombstone_files` | `zip_entry_listing` | ring of `tombstoned.max_tombstone_count` | the retained tombstone set, which may predate this boot |
| `am_proc_died` | `event_log_tag` | ring-wrapped | process death over the event-log window only |
| `manager_kill_lines` | `logcat_tag` | one line per manager decision | manager actions taken, distinct-matched |
| `rescueparty_lines` | `logcat_tag` | one line per mitigation attempt | mitigation attempts, distinct-matched |

**The guarantee column is platform behaviour, not an observation.**
Archives cannot establish ring sizes, rate limits or writer intent.
Prose records platform guarantees; artifacts carry measured `coverage_ratio` in `counts.<name>_hits`, derivable from bytes.

**Rule adopted: the crash count is always taken from the crash buffer (`C1`), the selected primary retained-record channel.
Ring wrapping, dropped messages and the requested window can still omit crashes; its count is not a guaranteed device-wide total.**
The other six are attached as *corroboration with their own counts and their own completeness*, never as the headline number.

**Java and native crashes are separated by design.**
Keep the 126-day-old native incident separate from live Java crashes.
The distinguishing test is purely structural:

| Test | Java | Native |
| --- | --- | --- |
| Where it lives | main-entry `SYSTEM LOG` text | **ZIP entry** `FS/data/tombstones/tombstone_NN` |
| Header line | `--------- beginning of crash` | `*** *** *** *** …` (`tombstone_proto_to_text.cpp:610`) |
| Identity line | `AndroidRuntime: Process: <pkg>, PID: <n>` | `pid: <p>, tid: <t>, name: <n>  >>> <proc> <<<` (`:125`) |
| Frame format | `\tat pkg.Class.method(File.java:123)` | `    #00 pc <addr>  <so> (sym+off) (BuildId: …)` |
| Own clock | line timestamp only | `Timestamp:` line, ns + tz offset (`:616`) |

In the reference archive **all 50 native entries** match the tombstone shape and **all 274 `FATAL EXCEPTION` lines** match the Java shape.
No overlap, no ambiguity.

#### Top-level keys

The fourteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`, plus `pid_max` and `min_crash_interval_ms` if read. |
| `channels` | array of object | yes | One entry per channel. Carries `emitter_kind`; `authoritative_for_count` is replaced by a single top-level `count_authority`. |
| `count_authority` | object | yes | `{channel, rule}`. **One top-level object naming which channel bounds the count and the published rule for choosing it.** The previous form was a per-channel `authoritative_for_count` boolean, which restated a policy decision on every row and let two channels disagree about it. |
| `java_crash_groups` | array of object | yes | Java crash signatures with their occurrence sets. |
| `java_crash_incidents` | array of object | yes | One entry per distinct Java crash. |
| `native_crash_groups` | array of object | yes | Native crash signatures. |
| `native_crash_incidents` | array of object | yes | One entry per distinct native crash. |

#### Grouping key

```text
signature = (language, exception_class_or_signal, top_frame_normalised)
```

| Term | Justification |
| --- | --- |
| `language` — **mandatory** | Otherwise the 271 Java crashes and the 50 native `SIGABRT`s of a different month collide into one "crash" group. |
| `exception_class` / `signal` | All 50 bluetooth tombstones are `SIGABRT`; grouping by signal alone would fuse unrelated aborts. The `Abort message:` line (`tombstone_proto_to_text.cpp:491`) is the discriminator there, but it is free text and unstable across builds — it is carried on the group as a *sub-grouping hint*, with the top native frame as tiebreak. |
| `top_frame_normalised` | **What makes a loop actionable.** `java.lang.IllegalArgumentException` occurs in thousands of unrelated bugs. `IllegalArgumentException` at `Preconditions.checkArgumentNonnegative` from `PhoneWindow.onContentApplyWindowInsets` is one bug. All 271 crashes share both. |
| Line numbers are **dropped** | They shift between builds; they are retained per occurrence for forensics. |
| `package` is **not** in the key | A loop alternating between two processes of the same app (e.g. `:remote`) must still group. `package` is carried on the group and reported as a value set. |

#### Loop versus isolated

A group is a **crash loop** when all five hold.
**The five thresholds are configurable inputs, not constants baked into the tool.**
Each is published in `classification_thresholds`, so an artifact states the bar it applied and a reader can re-apply it differently:

| Criterion | Field | Default threshold | Where the default comes from |
| --- | --- | --- | --- |
| Repeat count | `repeat_count` | ≥ 5 | Chosen so the bar sits just under the platform's own rate limiter (`DropboxRateLimiter.java:39`, `RATE_LIMIT_ALLOWED_ENTRIES_DEFAULT = 6`) — a limiter that lets 6 through is evidence the platform already treated 5+ as the same problem. **The limit itself is not published by any archive**, so it is a default, not a measurement. |
| Rate | `rate_per_second` | ≥ 1.0 | The loop must be faster than human retry. |
| Window | `window_seconds` | ≤ 300 s | Sized against the platform's own "this is one event, not a trend" bounds (`AnrHelper.java:56,61`). |
| Continuity | `inter_gap_max_ms` | ≤ 120000 | The platform's `quickCrash` test is `now < crashTime + MIN_CRASH_INTERVAL` (`AppErrors.java:896-897`), and `min_crash_interval` **is** printed in the archive's dumpsys output — so this one is read, not assumed. |
| Escalation | `manager_kill_count` or `rescueparty_failure_count` | ≥ 1 | Both subsystems act only on repetition. |

The `osdouble` group satisfies all five with margin.
The bluetooth tombstone group satisfies **none** of the window/continuity/escalation criteria — its occurrences span 2 h 35 m and no manager action is recorded in the log window — so it is correctly classified `stale_recurring`, **not** `crash_loop`.

#### Field tables

##### `crash_groups[]` (`java_crash_groups`, `native_crash_groups`)

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `group_id` | string | yes | `group-` plus 16 hex characters of SHA-256 over normalized signature and freshness cohort. | derived |
| `language` | enum | yes | `java` / `native`. | §above |
| `signature.exception_class` | string \| null | yes (java) | | `RuntimeInit.java:84-86` |
| `signature.signal_number` / `signal_name` | integer / string \| null | yes (native) | | `tombstone_proto_to_text.cpp:476` |
| `signature.sigcode_name` | string \| null | yes (native) | e.g. `SI_QUEUE`. | same |
| `signature.top_frame` | string | yes | Normalised, no line numbers. | §grouping key |
| `signature.abort_message_hash` / `abort_message_varies` | string \| null / boolean | yes (native) | The hash of the tombstone's `Abort message:` line, plus whether that message varies within the group. **The message is never published as free text and never used as a grouping key**, because it is unstable across builds: a build-to-build reword would silently split one bug into two. `abort_message_varies: true` forces a sub-grouping by `top_frame`. The reference archive's tombstones split 47 / 3 on this field, so a single group would have hidden a second root cause. | — |
| `packages[]` / `package_count` | array / integer | yes | May exceed 1 for `:remote` loops. | `RuntimeInit.java:94` |
| `repeat_count` | integer | yes | **Distinct** occurrences, de-duplicated. | §274-vs-271 |
| `raw_line_hits` | integer | yes | Every matching line **before** de-duplication. | — |
| `duplicate_hits` | integer | yes | `raw_line_hits − repeat_count`. | §274-vs-271 |
| `duplicate_reason` | **Removed**, replaced by `duplicate_detection`. | — | The previous value named an AOSP `path:line` and asserted a collector the tool cannot inspect — *no archive byte records which collector re-rendered a line*. Worse, its stated discriminator was a mis-parse: it read a blank uid column as a uid value. | — |
| `duplicate_detection` | object | yes | `{method, duplicate_pids}`. `method` ∈ `same_process_pid_timestamp_exception_and_normalized_frame` — the actual rule, decided from bytes — and `duplicate_pids` lists the pids seen more than once. **A statement about what matched, not about who produced it.** | derived |
| `first_seen` / `last_seen` | string | yes | Min/max occurrence time. | `logprint.cpp:1552` / `tombstone_proto_to_text.cpp:616` |
| `window_seconds` | number | yes | `last − first`. | derived |
| `rate_per_second` | number \| null | yes | `repeat_count ÷ window_seconds`. **No `rate_derivation` string**: the formula is stated here, so per [§2.9](02-evidence-conventions.md#29-null-absent-and-raw) rule 4 a sibling string would only restate it. | derived |
| `inter_gap_p50_ms` / `inter_gap_max_ms` | integer \| null | yes | Derived over the gaps between occurrence times. **Milliseconds in both the field name and the loop criterion** — the reference artifact's criterion table said `inter_gap_max_seconds` while the field table said `_ms`, and an implementer following the first would emit the wrong unit. | derived |
| `pid_span` | object | yes | `{first, last, distinct_count, pid_max}`. `null` members when the pids are cross-boot and meaningless against this boot. | `logprint.cpp:1553` |
| `classification` | enum | yes | `crash_loop` / `recurring` / `isolated` / `stale_recurring`. | §loop versus isolated |
| `classification_thresholds` | object | yes | The five values actually applied. Published so `classification` is re-derivable. | — |
| `classification_met` | array of string | yes | Which criteria fired, by name from the table above. **Replaces `classification_reason`**, which was a sentence naming thresholds met and failed — a paraphrase a reader had to trust rather than check. | derived |
| `manager_kill_count` | integer | yes | **Distinct** matches. The reference artifact reported a raw line count that included re-rendered copies, so the number exceeded the crash count beside it with no note. | `AppErrors.java:902` |
| `manager_kill_reason` | string \| null | yes | The printed `Reason:` token, verbatim. One of `crashed quickly`, `over process crash limit`. A value outside that set is a defect: the reference example published `crashed` where the archive prints `crashed quickly`. | `AppErrors.java:903` |
| `rescueparty_failure_count` | integer | yes | **Distinct** matches, same de-duplication as `manager_kill_count`. | `RescueParty.java:822` |
| `rescueparty_disabled` | boolean \| null | yes | | observed in archive |
| `corroboration[]` | array of object | yes | One entry per channel: `{channel, count, count_kind}` where `count_kind` ∈ `event_log_rows`, `logcat_lines_distinct`, `logcat_lines_raw`. **The previous form mixed raw line counts, de-duplicated counts, and event-log rows in one array of plain integers**, so a consumer could not tell which convention applied to which row — and that is the one thing `corroboration` exists to prevent. | §channels |
| `provenance` | object | yes | | — |

##### `crash_incidents[]`

One entry per distinct crash — the thing a loop groups over.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `incident_id` | string | yes | | derived |
| `group_id` | string | yes | FK into the group. | derived |
| `language` | enum | yes | | §above |
| `package` / `process` | string | yes | | `RuntimeInit.java:94` / `tombstone_proto_to_text.cpp:121` |
| `pid` | integer \| null | yes | From the **message**, cross-checked against the logcat prefix. | `RuntimeInit.java:97` / `:125` |
| `prefix_pid` / `prefix_uid` | integer \| null | yes | From the logcat prefix. **A mismatch is the signature of a re-rendered, not re-crashed, record.** | `logprint.cpp:1553` |
| `thread_name` | string \| null | yes (java) | | `RuntimeInit.java:92` |
| `exception_class` | string \| null | yes (java) | | `RuntimeInit.java:84-86` |
| `exception_message` | string \| null | yes | `null` when the archive prints `NULL` in `am_crash` and logcat carries no message line. | same |
| `top_frames[]` | array of string | yes | First five, or more. | ART stack rendering |
| `signal_number` / `signal_name` / `sigcode_name` / `fault_addr` | — | yes (native) | | `tombstone_proto_to_text.cpp:476` |
| `abort_message` | string \| null | yes (native) | | `tombstone_proto_to_text.cpp:491` |
| `occurred_at` | string | yes | java: logcat line time; native: in-file `Timestamp:`. | `logprint.cpp:1552` / `tombstone_proto_to_text.cpp:616` |
| `process_uptime_seconds` | integer \| null | yes (native) | **Process-relative monotonic — never a wall clock.** | `tombstone_proto_to_text.cpp:617` |
| `flags_hex` | string \| null | yes | `ApplicationInfo.flags`. | `AMS:9643` / `logtags:56` |
| `crash_handler` | string \| null | yes | | `AMS:9818` `Crash-Handler:` |
| `evidence_channels[]` | array of object | yes | As in `anr.json`: `{channel, join_basis}`, where `join_basis` ∈ `same_logcat_line`, `same_pid_and_window`, `heuristic`. A bare array of names claimed agreement the archive never records. | §channels |
| `provenance` | object | yes | | — |

#### The 274-versus-271 finding

Raw `FATAL EXCEPTION` count is **274**.
Decomposing by region:

| Region | Line range | `FATAL EXCEPTION` |
| --- | --- | --- |
| A — first `SYSTEM LOG` (`logcat -v threadtime … -d *:v`) | 9116–53708 | 38 |
| B — broader `DUMPSYS` block (inlined logcat) | 365371–617121 | **3** |
| C — trailing `SYSTEM LOG` (`… -T <timestamp>`) | 653373–682949 | 233 |
| | **total** | **274** |

The explicit `DROPBOX SYSTEM APP CRASHES` section at 652155–652449 contains **zero** `FATAL EXCEPTION` lines.
The three additional copies occur in the broader `DUMPSYS` block.
The attribution is measured against the hash-identified reference fixture.
De-duplicate using the observed process identity, time and matching crash content.
PID and second alone are a fixture shortcut, not a universal key: PID reuse and multiple records within a second require further discrimination.

**Therefore: 271 distinct crashes, 274 raw lines.**
Raw counts/naive `pid` keys include three duplicate records, not additional crashes.

**Process identity in the reference loop.**
271 crashes produced **271 distinct, monotonic PIDs** (1116 → 9985).
The three apparent repeats were the dropbox duplicates.
Monotonic PID keys would produce 271 singleton findings, hiding the loop.

#### Provenance and freshness

Two incidents live in one archive and must never be conflated:

**Incident 1 — the live Java loop.**
Every occurrence is inside `capture.log_window` and carries `captured_at_time_base: "logcat_line"`.
The 271st crash at `00:27:51.970` is the last line of the crash buffer, which shows crashes continuing late in collection, after the initial banner time.
It does not prove that the loop persisted after collection ended.

| Fact | Value | Read from |
| --- | --- | --- |
| Package | `<crashing_package>` | `AndroidRuntime: Process:` — the only crashing package |
| Exception | the only exception type, present in every block | 3rd line of each block |
| Top frame | `<normalised frame 1 symbol>` | frame 1 |
| Next frames | frames 2–5, symbols only, line numbers dropped | frames 2–5 |
| First / last crash | two stamps inside the log window | |
| Window | derived | derived |
| Rate | derived | derived |
| Inter-crash gaps | min / p50 / max over the gaps | derived |
| Distinct pids | monotonic across the window | `pid_max` read from the archive |
| Manager kills / RescueParty | two distinct-match counts | `AppErrors.java:902` / `RescueParty.java:822` |
| Kill reason | the printed `Reason:` token, verbatim | `min_crash_interval` printed in the dumpsys output |
| uid | from the logcat uid column, resolved | logcat uid column |

**Incident 2 — the old native group.**
Every tombstone predates the capture by months.

| Fact | Value |
| --- | --- |
| File mtimes | two stamps from the ZIP directory, with the span derived |
| In-file `Timestamp:` on one entry | the entry's own header stamp |
| `Cmdline:` | one package name, present in every tombstone |
| `signal` | one signal, present in every tombstone |
| `Abort message:` | **two distinct values across the 50 tombstones** |
| Grouping | **two** groups, 47 and 3 occurrences |

> The `Abort message:` field splits the 50 tombstones 47 / 3 — the three minority entries abort from `stack_manager.cc` at startup rather than from `hci_layer.cc` on a command response.
> Preserve the abort-message discriminator so the two failure signatures remain separate.

Separate from the Java loop by language, signal class, date and package.

