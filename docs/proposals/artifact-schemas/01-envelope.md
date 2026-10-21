# Shared conventions — the envelope, section status, and counts

> **Promoted contract for `UC-03`.**
> See [`index.md`](index.md) and [generation rules](12-generation-rules.md).
> Requirements: `REQ-INSIGHT-03`, `REQ-INSIGHT-04`, `REQ-INSIGHT-05`.

Shared rules for all six JSON artifacts; artifact entries add domain-specific fields.

The next file, [`02-evidence-conventions.md`](02-evidence-conventions.md), continues with provenance, freshness, severity, and evidence.

## 2. Shared Conventions

This section is normative and self-sufficient.
Read this section and the relevant [§4 artifact entry](04-readme-artifact.md#4-the-seven-artifacts).

### 2.1 The envelope

Every JSON artifact is a single object.
The six JSON artifacts share these **fourteen envelope keys**, with additional artifact-specific keys defined on their own pages:

```text
schema_version, artifact, generator, source, capture, scope, collection,
counts, severity_summary, freshness_summary, negative_evidence, evidence_gaps,
findings, notes
```

`README.md` is Markdown and has no envelope; it references these structures but does not serialise them.

`system.json` is the one artifact that owns `capture`.
In the other five, `capture` is **replaced** by `capture_ref`, a pointer to the canonical object ([§2.9](02-evidence-conventions.md#29-null-absent-and-raw) rule 6).
Both forms are counted in the fourteen, so every artifact has the same key count.

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | yes | Version of this schema. Bumped when a key is renamed or removed. |
| `artifact` | enum | yes | One of `system`, `apps`, `anr`, `crashes`, `memory`, `cpu`. Identifies which artifact this is. |
| `generator` | object | yes | Tool provenance. See [§2.1.1](#211-generator). |
| `source` | object | yes | Identity of the archive read. See [§2.1.2](#212-source). |
| `capture` | object | yes | The capture instant and its context. `system.json` only; every other artifact carries `capture_ref`. See [§2.1.3](#213-capture). |
| `scope` | object | yes | What was and was not searched. See [§2.1.4](#214-scope). |
| `collection` | object | yes | What the collector itself did, so "is this number self-inflicted?" is answerable. See [§2.1.5](#215-collection). |
| `counts` | object | yes | Match blocks. See [§2.3](#23-counts-are-match-blocks). |
| `severity_summary` | object | yes | Tally by severity. Must sum to `findings.length`. |
| `freshness_summary` | object | yes | Tally by freshness state. Must sum to `findings.length`. |
| `negative_evidence` | array | yes | Statements of absence, each with an authority level. See [§2.1.6](#216-negative_evidence). |
| `evidence_gaps` | array | yes | Fields that could not be produced, each with a machine-readable `reason` code. See [§2.1.7](#217-evidence_gaps). |
| `findings` | array | yes | The payload. May be empty; see below. |
| `notes` | array of string | yes | Free-text caveats, each prefixed `"<CODE>: <text>"`. |

**On empty arrays.**
`findings: []` is valid but requires explanation.
An artifact with no findings must carry at least one `negative_evidence` entry or a `notes` entry explaining why there are none.
See [§2.1.6](#216-negative_evidence).

**On the `findings` element shape.**
Every element of `findings[]` carries the same core fields, plus whatever its artifact defines:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | yes | `<artifact_prefix>-<4-digit ordinal>`, assigned in archive order so ids are stable across runs on the same archive. Prefix: `anr`, `crash`, `mem`, `cpu`, `app`, `sys`. Ids start at `0001`. |
| `kind` | string enum | yes | A noun for a record type, never a symptom. Artifact-specific; the closed set is listed in each artifact's entry. |
| `severity` | enum | yes | See [§2.6](02-evidence-conventions.md#26-severity). Produced by the artifact's published `severity_rule`. |
| `unconstrained_severity` | enum | yes | The severity this finding would have absent the freshness cap. Always present. Equals `severity` when no cap applied. |
| `confidence` | enum | yes | `confirmed` (a validated record), `inferred` (a derived conclusion), `reported` (a third-party assertion we did not verify). `reported` **must** be used when the only evidence is an event-log line, and **must** be used when the only evidence is an `archive_absence` entry — an absence can never be `confirmed`. |
| `process_key` | string \| null | yes | See [§2.8](02-evidence-conventions.md#28-identity). `null` only when the finding is genuinely device-wide. |
| `event_time` | string \| null | yes | When the **condition** happened. Distinct from when the evidence was captured. |
| `attributes` | object | yes | Record-type-specific **scalars only** — string, integer, number, or boolean. A nested object or array here is a schema violation; anything structured belongs in `metrics`. |
| `metrics` | object | yes | The measurements that justified the severity. A `high` or `critical` finding with empty `metrics` is a schema violation. |
| `evidence` | object | yes | Exactly one. See [§2.7](02-evidence-conventions.md#27-evidence). |
| `corroborating_evidence` | array | yes | May be empty. See [§2.7](02-evidence-conventions.md#27-corroborating_evidence). |
| `provenance` | object | yes | All keys in [§2.4](02-evidence-conventions.md#24-provenance). |

#### 2.1.1 `generator`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | Constant `"bugreport-insight"`. |
| `version` | string | yes | Cargo package version, no `v` prefix. |
| `run_started_at` | string (RFC 3339, explicit offset) | yes | **Host** wall clock. Never the device clock. |
| `run_finished_at` | string (RFC 3339, explicit offset) | yes | Host wall clock. |
| `host_timezone_offset` | string (`±HH:MM`) | yes | The host's offset at run time. |

#### 2.1.2 `source`

Archive identity.
Read from the ZIP, not derived.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `archive_name` | string | yes | Basename of the ZIP as supplied. | host |
| `archive_size_bytes` | integer ≥ 0 | yes | File size on the host. | host |
| `archive_sha256` | string | yes | Lowercase hex, 64 characters, over the compressed bytes as stored. | host |
| `format_version` | string \| null | yes | Validated marker text under [ADR 0008](../../decisions/0008-validated-marker-metadata.md), removing one terminal LF/CRLF. `null` if absent or unavailable. Only absence establishes format 0 and its core `ARCHIVE_FORMAT_0` note; invalid/oversized/unreadable markers remain unknown with a system evidence gap and marker-status finding attribute. | `dumpstate.cpp:2872`; value from `dumpstate.h:157` |
| `main_entry` | string \| null | yes | Validated text of `main_entry.txt` under ADR 0008, preserving all spaces after one LF/CRLF removal. `null` if absent or unavailable. | `dumpstate.cpp:2752-2755` |
| `main_entry_size_bytes` | integer \| null | yes | Declared uncompressed size of the entry exactly named by a valid main marker. `null` if the marker is unavailable or no entry matches. | host |
| `main_entry_line_count` | integer \| null | yes | Number of `\n`-terminated lines. `null` if unreadable. | host |
| `zip_entry_count` | integer | yes | Entries in the central directory. Read-only access must walk the central directory, never the whole file (`REQ-GEN-01`, `REQ-GEN-02`). | `system/libziparchive/zip_archive_common.h:31-67` |
| `device` | object | yes | Device identity block, below. | — |

`device`:

| Field | Type | Required | Meaning | AOSP source (`frameworks/native/cmds/dumpstate/`) |
| --- | --- | --- | --- | --- |
| `product_name` | string \| null | yes | `ro.product.name`. | `dumpstate.cpp:2832` |
| `build_id` | string \| null | yes | `ro.build.id`. | `dumpstate.cpp:2831` |
| `build_display_id` | string \| null | yes | `ro.build.display.id`. | `dumpstate.cpp:795` |
| `build_fingerprint` | string \| null | yes | `ro.build.fingerprint`. | `dumpstate.cpp:796`; archive line 7 |
| `android_sdk_version` | integer \| null | yes | `ro.build.version.sdk`. | `dumpstate.cpp:800`, printed `:818` |
| `build_type` | string \| null | yes | `user`, `userdebug`, or `eng`. Gates almost everything — see [§3](03-build-variants.md#3-build-variant-behaviour). | `DumpstateUtil.cpp:212-217` |
| `device` | string \| null | yes | Device name, parsed from the fingerprint segment after the product. | `dumpstate.cpp:810` |
| `hardware` | string \| null | yes | From `androidboot.hardware` in the `Bootconfig:` block. | `dumpstate.cpp:826-827` |
| `bootloader` | string \| null | yes | `ro.bootloader`. | `dumpstate.cpp:798`, printed `:811` |
| `radio` | string \| null | yes | `gsm.version.baseband`. | `dumpstate.cpp:797`, printed `:812` |

**Rule.**
`source` carries no time.
The capture instant lives in `capture`, and the evidence times live in each finding's `provenance`.
Separate archive and evidence clocks.

#### 2.1.3 `capture`

The capture instant and the context needed to interpret anything else.

`system.json` owns the canonical copy.
**Every other artifact references it as `capture_ref.source: "system.json#/capture"` and does not re-derive it.**
Compute uptime, log window and boot instant once to prevent divergent derivations.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `captured_at` | string (RFC 3339, explicit offset) \| null | yes | The archive's capture instant. `null` if no clock is derivable — a legitimate outcome, not an error. | `== dumpstate:` banner, `dumpstate.cpp:802-805` |
| `captured_at_time_base` | enum \| null | yes | Which clock supplied `captured_at`. `null` when `captured_at` is `null` — a clock cannot be named when no clock was found. See [§2.4](02-evidence-conventions.md#24-provenance). | — |
| `uptime_at_capture_seconds` | integer \| null | yes | Device uptime in seconds at capture. Never inferred from logcat timestamps. | `------ UPTIME (uptime) ------`, `dumpstate.cpp:1687`; header line `dumpstate.cpp:828-830` |
| `log_window` | object | yes | The archive's logcat window. See below. | `logprint.cpp:1552-1554` |
| `boot` | object | yes | Boot-instant derivation. See below. | — |
| `is_cold_boot` | boolean | yes | `uptime_at_capture_seconds < 300`. | derived |

`capture.log_window`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `sections` | array of string | yes | The logcat sections present, in file order. **The list repeats** — the reference archive has two `SYSTEM LOG` blocks. |
| `first_line_at` / `last_line_at` | string | yes | Wall-clock stamp of the first/last parseable logcat line, `MM-DD HH:MM:SS.mmm`. Year-less; see [§2.9](02-evidence-conventions.md#29-null-absent-and-raw). |
| `duration_seconds` | number | yes | `last − first`. |
| `is_contiguous` | boolean | yes | `false` when a second, `-T`-bounded `SYSTEM LOG` follows. |
| `first_line_minus_boot_seconds` | number \| null | yes | `first_line_at` minus `boot.at`, in seconds. **Publish the measured difference, not a boolean verdict about it.** The sign answers "does the log window start before boot" — but a boolean over it flips whenever the margin falls inside `boot.uncertainty_seconds`, so the boolean cannot be computed honestly and must not be in the schema. `null` when `boot.at` is `null`. |
| `truncated` | boolean \| null | yes | `null` — dumpstate records no logcat truncation flag. |

`capture.boot`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `at` | string \| null | yes | Derived boot wall-clock time. |
| `derivation` | string | yes | The formula and its two anchor lines, so the number is auditable. Required because it is computed, not read. |
| `epoch_corroboration` | object \| null | yes | `ro.runtime.firstboot` and whether it agrees. Recorded even when it disagrees, so a consumer does not silently prefer it. |

#### 2.1.4 `scope`

What was searched, what was found, and what was not there.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `sections_scanned` | integer | yes | Number of `------ … ------` headers recognised. |
| `sections_matched` | integer | yes | How many this artifact consumed. |
| `sections_listed` | array of string | yes | Titles consumed, in archive order. **Titles only, not unique** — duplicates are kept so a reader can see the two `SYSTEM LOG`s. |
| `sections_skipped` | array of string | yes | Recognised and deliberately not consumed. The reason goes in `notes` as `SKIP_<REASON>`. |
| `sections_not_found` | array of string | yes | **Expected** section titles absent from the archive. This is the field that makes "no ANR" distinguishable from "no ANR section". Non-empty ⇒ a mandatory `notes` entry. |
| `max_lines_read` | integer | yes | The safety cap actually applied (`REQ-GEN-02`). |
| `truncated` | boolean | yes | `true` if any cap was hit. **`true` forbids `freshness: "now"` on any finding** and must produce a `notes` entry `TRUNCATED`. |

`system.json` extends `scope` with a per-section `sections[]` array carrying a `status` from [§2.2](#22-section-status-enum).
Every other artifact inherits the section list from `system.json#/scope` via `capture_ref` rather than repeating it.

#### 2.1.5 `collection`

Built from dumpstate's own per-section duration footers — the `------ N.NNNs was the duration of 'TITLE' ------` lines emitted at `dumpstate.cpp:3831-3835`.
Same schema in every artifact.

Question: **did collection change the reported number?**
Swap deltas must show overlapping page-scanning sections, preventing automatic attribution to device behavior.

No `assessment` or statement-order field: swap values/durations lack per-process page-fault accounting to establish collector causation.
Publish measurements; label causal discussion as prose in `README.md` §4.1.

> **Archive order is not execution-time order.**
> Parallel tasks write temporary files; `DumpPool.cpp:37-48` appends each result when its particular future is awaited.
> The caller's wait order can differ from task completion order.
> Keep line order for evidence locators, but use printed clocks and durations for temporal overlap; line spans alone cannot prove it.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `page_scanning_sections` | array of object | yes | Sections whose body contains a per-PID row set carrying a swap column, or a `/sys/block/zram*/stat` block. Membership is decided by **what the section prints**, never by AOSP source knowledge; each entry carries `name`, `line_start`, `line_end`, `duration_ms`. |
| `page_scanning_total_ms` | number | yes | Sum of the above, from the per-section duration footers. |
| `swap_timeline` | array of object | yes | Swap readings across the dump, **ordered by `line_start`**. Every delta is an **edge between two identified nodes**; there is no scalar a consumer can diff naively. Each entry carries `line_start` pointing at the payload line the value was read from. |
| `swap_growth_kib` | integer \| null | yes | `last − first` across the timeline. `null` if fewer than two usable points. |
| `page_scanning_ms_in_interval` | number \| null | yes | Sum of durations whose measured execution intervals overlap the first and last readings. Line-order overlap alone is insufficient under parallel collection; use `null` when timing anchors are unavailable. This is overlap, not causal attribution. |

Explaining deltas requires unavailable per-process page-fault data; causal claims are not fields.
Where a collector's influence is nonetheless directly checkable — the `dumpsys cpuinfo` window closing before the dump began — it is carried per-provenance as `collector_perturbed`, which §2.4 already requires.

#### 2.1.6 `negative_evidence`

A required array.
Each entry:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `scope` | string | yes | What the statement covers: a channel, a section, an archive-wide search. |
| `statement` | string | yes | The verbatim platform string where one exists, or the precise description of what was searched. |
| `source` | object \| null | yes | Where to see it — an archive locator (`archive_path`, `section_title`, `section_ordinal`, `line_start`, `line_end`). `null` only for an archive-wide search, where no single span exists. |
| `authority` | enum | yes | See below. |

`authority` is either:

| Value | Meaning |
| --- | --- |
| `platform_assertion` | A platform string asserting absence — e.g. `  <no ANR has occurred since boot>`. **Authoritative.** |
| `archive_absence` | We looked and found nothing. **Indicative only.** |

An empty `negative_evidence` array is allowed when no bounded absence statement is supported.
An empty `findings` array still needs a negative-evidence entry or explanatory note as stated above.
This separates "65 ANR files" from "an ANR happened".
The reference archive contains 65 ANR trace files, all 126 days old, from a boot that ended before this one — and simultaneously two dumpsys sections asserting in so many words that no ANR has occurred since boot.
Carry both facts for correct interpretation.

#### 2.1.7 `evidence_gaps`

A required array in **every** artifact.
Declare unavailable evidence instead of inventing values.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `field` | string | yes | Dotted path of the field or object the gap concerns. |
| `reason` | enum | yes | A code from the closed table below. |
| `detail` | string \| null | yes | Human-readable explanation. **Never load-bearing** — a consumer must not have to parse it, and a validator must never depend on its wording. |

`reason` is a **closed vocabulary**, not prose:

| `reason` | Meaning |
| --- | --- |
| `NOT_PRINTED` | The source field exists in the platform but the archive does not print a value for it. |
| `SECTION_ABSENT` | The section that would carry this was not in the archive. |
| `SECTION_FAILED` | The section is present but its command failed or timed out. |
| `REGION_EXCLUDED` | The section was read but the relevant span was outside the read region. |
| `PARSE_INDETERMINATE` | The bytes are present but do not determine the value. |
| `AMBIGUOUS_SOURCE` | Two or more archive sources disagree and nothing in the archive adjudicates. |
| `UNVERIFIABLE` | The value would require knowledge outside the archive — platform source, build configuration, or live device state. |

`evidence_gaps[]` describes values the *archive* cannot determine; analyzer parsing failures are tool bugs, not evidence gaps.
`UNVERIFIABLE` is legitimate/frequent; its absence can hide platform assumptions.

> **An empty `evidence_gaps` array is a claim.**
> It asserts that every field this artifact publishes was derivable from this archive.
> That is almost never true, so an empty array should be as rare as a `findings: []` with no explanation.

#### 2.1.8 `notes`

Free-text caveats, each prefixed `"<CODE>: <text>"`.
Notes carry caveats without a specific field.

`notes[]` is **not** where platform knowledge should move to.
AOSP knowledge such as "`tombstone_00` is a ring index, not the newest file" belongs in specification prose, not repeated artifact notes.
Notes carry facts about *this run* — a truncation was hit, a clock skew was seen, a duplicate rendering was found.

### 2.2 Section status enum

Each entry of `system.json#/scope/sections[]` carries a `status`:

```text
ok | empty | skipped_user_build | skipped_root_denied | skipped_missing_source |
skipped_budget | not_found | unreadable
```

| Value | Meaning |
| --- | --- |
| `ok` | Section present and non-empty. |
| `empty` | Section header emitted, zero payload. |
| `skipped_user_build` | dumpstate declined to run the command on a `user` build. It prints `Skipping '<cmd>' on user build.` **in place of** the banner — see [§3](03-build-variants.md#3-build-variant-behaviour). |
| `skipped_root_denied` | The command needed root and the root drop failed. Present-but-empty with an error marker. |
| `skipped_missing_source` | A `DumpFile` whose `open()` failed. Present-but-empty with `*** Error dumping %s: %s`. |
| `skipped_budget` | A time budget or size budget excluded the section. |
| `not_found` | Expected and absent. |
| `unreadable` | Present but not parseable — e.g. the entry spans a NUL byte. |

`skipped_user_build` and `skipped_root_denied` are load-bearing.
A consumer must be able to tell **"none exist"** from **"we were not allowed to look"**.
Two platform markers make that distinction possible and must be mapped to `ok` when present:

- `*** NO TOMBSTONES to dump in /data/tombstones/` — authoritative absence.
- `*** NO ANRs to dump in /data/anr/` — authoritative absence.

### 2.3 Counts are match blocks

**No artifact may report a bare number.**
Raw grep counts must not become findings.
Measured on the reference archive: `tombstone` matches **749** times and yields **0** real tombstones; `signal ` matches **656** times and yields **0** fatal signals; `watchdog` matches **315** times and yields **6** actionable findings.

Every counted quantity is a **match block** under `counts.<name>_hits`:

```text
counts.<name>_hits = {
  pattern, pattern_kind, case_sensitive,
  raw_count,                       // every line matching the pattern
  matched_evidence,                // promoted to a finding
  suppressed_noise,                // matched the string, is not the thing
  not_emitted_by_platform,         // the platform never produced it (e.g. no section)
  suppressed_noise_note,           // REQUIRED when suppressed_noise > 0
  coverage: { window_start, window_end, archive_window, coverage_ratio },
  regions: [ … ]
}
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `pattern` | string | yes | The literal pattern searched. Never elided. |
| `pattern_kind` | enum | yes | `substring`, `word_boundary`, `line_exact`, `line_regex`, or `field`. A `substring` pattern may not report `matched_evidence > 0` without naming the validation that promoted a hit. |
| `case_sensitive` | boolean | yes | Both ways matter: `tombstone` is 749 case-sensitive and 893 case-insensitive on this archive. |
| `raw_count` | integer | yes | Matching lines, whole-archive, no section filter. |
| `matched_evidence` | integer | yes | Hits that became findings. |
| `suppressed_noise` | integer | yes | `raw_count − matched_evidence`, minus anything the platform never emitted. |
| `not_emitted_by_platform` | integer | yes | **This evidence channel does not exist in the archive** — no section, no tag, no entry. Distinct from "searched and found nothing". This one is archive-observable (the section header is absent), so it stays. **What does not stay is any assertion about *why* the platform omitted it**, which is AOSP behaviour and belongs in `evidence_gaps[]` with `reason: UNVERIFIABLE`. |
| `suppressed_noise_note` | string | yes when `suppressed_noise > 0` | Why the residue is not a finding. **Restricted to facts a reader can re-check against the same bytes** — which match, which did not, and the shape of the difference. It must not assert a cause ("the platform never emits", "this is only boot noise") unless the cause is visible in the archive, in which case the *count* belongs in `suppressed_noise` and the explanation belongs in `evidence_gaps[]`. |
| `coverage` | object | yes | See below. |
| `regions[]` | array | yes | Every contiguous run of matching lines, with its section. |

`coverage`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `window_start` / `window_end` | string \| null | yes | The channel's own observation window. `null` for a point-in-time snapshot. |
| `archive_window` | object | yes | The archive's `capture.log_window`. |
| `coverage_ratio` | number \| null | yes | **Required** whenever a channel observes a window narrower than the archive's. |

**Two invariants, both mechanically checkable:**

1. `matched_evidence + suppressed_noise + not_emitted_by_platform == raw_count`, with `not_emitted_by_platform` counted as zero within a region.
2. `coverage.coverage_ratio` **must** be present whenever a channel observes a window narrower than the archive's log window.

The real `am_proc_*` event-log buffer covers only **20.304 s** of the archive's **64.662 s** window — ratio **0.314** — which is why **40** `am_proc_died` entries coexist with **271** real process deaths.
Counts need coverage as well as noise splits.

**Count-key suffixes are closed.**
`_hits` (raw string matches), `_records` (promoted findings), `_entries` (archive entries).
No other suffix.
A bare `count`, `total`, `matches`, or `num` key is a schema violation.

**Suppression reasons are a closed vocabulary.**
New reasons require a schema revision.

| Reason | Meaning | Reference-archive instance |
| --- | --- | --- |
| `process_name_substring` | The term appears inside a process, thread, or section *name* that contains it. | `NativeTombstoneManager:tombstoneWatcher` (126) |
| `mapped_library_path` | The term appears in a mapped-library path (`/proc/PID/maps`, `SMAPS`). | `libtombstoned_client.so` (137) |
| `section_title_reference` | The term appears in a `------ … ------` header — the section is *about* the thing, not an instance of it. | `------ SHOW MAP 240: /system/bin/tombstoned ------` (2) |
| `log_tag_chatter` | A logcat tag or routine lifecycle message at `I`/`V`/`D` with no error semantics. | `tombstoned:` tag (314) |
| `routine_teardown_signal` | A signal number in an ordinary init/lifecycle message. | `init: Sending signal 9 to service '…'` — `system/core/init/service.cpp:206-207` (417) |
| `identifier_symbol` | Part of a longer identifier, symbol, SELinux context, or property name. | `ro.carwatchdog.client_healthcheck.interval` |
| `configuration_path` | A path, filename, or config reference with no runtime meaning. | `carwatchdogd.rc:22` |
| `listing_only` | The match is in a directory listing — proof a file *exists*, not that it contains the condition. | the `ANR FILES` listing naming 65 ANR files |
| `stale_region` | The match is in a region already classified `stale`. | inside `VM TRACES AT LAST ANR` |
| `duplicate_region` | The match is in a region already counted. | the 184 overlapping logcat lines |
| `format_artifact` | A structural or format line, not device content. | `------ 0.000 was the duration of 'SHOW MAP 240: /system/bin/tombstoned' ------` |

**Suppression must be visible, not silent.**
A `matched_evidence` of 0 is a **result**.
It must be surfaced as a `findings[]` entry with `kind: "signal_absence"`, `severity: "info"`, and a `freshness_basis` naming the regions searched.

