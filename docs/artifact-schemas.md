# Artifact Schemas

## Table of Contents

- [1. Introduction](#1-introduction)
  - [1.1 What the tool is](#11-what-the-tool-is)
  - [1.2 What an artifact is](#12-what-an-artifact-is)
  - [1.3 Read-only over ZIP](#13-read-only-over-zip)
  - [1.4 Relationship to `docs/requirements.md`](#14-relationship-to-docsrequirementsmd)
- [2. Shared Conventions](#2-shared-conventions)
  - [2.1 The envelope](#21-the-envelope)
  - [2.2 Section status enum](#22-section-status-enum)
  - [2.3 Counts are match blocks](#23-counts-are-match-blocks)
  - [2.4 Provenance](#24-provenance)
  - [2.5 Freshness](#25-freshness)
  - [2.6 Severity](#26-severity)
  - [2.7 Evidence](#27-evidence)
  - [2.8 Identity](#28-identity)
  - [2.9 Null, absent, and raw](#29-null-absent-and-raw)
  - [2.10 The two section-parsing traps](#210-the-two-section-parsing-traps)
  - [2.11 Units and time formats](#211-units-and-time-formats)
- [3. Build-Variant Behaviour](#3-build-variant-behaviour)
  - [3.1 `IsUserBuild()` is the single primitive](#31-isuserbuild-is-the-single-primitive)
  - [3.2 Gated sections](#32-gated-sections)
  - [3.3 Failure modes that leave a well-formed archive](#33-failure-modes-that-leave-a-well-formed-archive)
  - [3.4 dumpsys sections are an open set](#34-dumpsys-sections-are-an-open-set)
- [4. The Seven Artifacts](#4-the-seven-artifacts)
  - [4.1 `README.md`](#41-readmemd)
  - [4.2 `system.json`](#42-systemjson)
  - [4.3 `apps.json`](#43-appsjson)
  - [4.4 `anr.json`](#44-anrjson)
  - [4.5 `crashes.json`](#45-crashesjson)
  - [4.6 `memory.json`](#46-memoryjson)
  - [4.7 `cpu.json`](#47-cpujson)
- [5. Divergences Resolved](#5-divergences-resolved)
  - [5.1 The D1–D5 decisions](#51-the-d1d5-decisions)
  - [5.2 Corrected facts](#52-corrected-facts)
- [6. Known Limitations](#6-known-limitations)

---

## 1. Introduction

### 1.1 What the tool is

`bugreport-insight` is a static analysis tool for Android bugreport ZIP archives. It is
organized as a Cargo workspace written in Rust 2024, built around a single shared foundation
crate (`core`) that owns all archive access, thin per-signal analyzer crates
(`anr`, `apps`, `stacktrace`, `memory`, `cpu`), and two independent frontends: a CLI for
engineers and an MCP server for AI agents. The crate map and dependency rules are in
[`architecture.md`](./architecture.md); this document says nothing about them.

This document is the **authoritative schema specification for the seven analysis artifacts**
that the tool emits. It is the contract that the Rust types in `core` and the analyzer crates
will implement, and it is binding on any future artifact author.

### 1.2 What an artifact is

An artifact is a single output file for one archive, in one of seven forms:

| Artifact | Form | Purpose in one line |
| --- | --- | --- |
| `README.md` | Markdown | The human triage document a person reads first. |
| `system.json` | JSON | The device fact sheet: what was captured, when, on what. |
| `apps.json` | JSON | What is installed, and which process ran on whose behalf. |
| `anr.json` | JSON | Whether an ANR happened, when, and what traces exist. |
| `crashes.json` | JSON | What crashed, how often, in what pattern. |
| `memory.json` | JSON | Memory pressure, attribution, and whether the collector caused it. |
| `cpu.json` | JSON | CPU and I/O utilization, and lock/blocking contention. |

An artifact is **not** a copy of the archive. It is a set of claims, each of which carries a
locator back into the ZIP and an explicit statement of how old the evidence behind it is.

### 1.3 Read-only over ZIP

The tool never extracts the archive. It opens the ZIP, reads entries in place, and writes
only into its own output directory. This is `REQ-GEN-01` in
[`requirements.md`](./requirements.md), and it has one direct consequence that shapes every
schema here:

> **Evidence is always a locator, never a copy.**

A finding does not embed a crash dump. It embeds an entry path, a line range, and a bounded
excerpt — enough for a reader to find the bytes without the artifact having to reproduce
them. The bound is hard (see [§2.7](#27-evidence)).

The archive is hostile to naive tooling. In the reference archive the main entry is
64,645,191 bytes across 682,951 lines, **one line is 192,666 characters long**, and there are
**4 NUL bytes** at offsets 44,988,012–15. Any parser built on line-oriented reads must bound
every excerpt and must handle a line that does not fit in a buffer.

### 1.4 Relationship to `docs/requirements.md`

The two documents answer different questions and neither replaces the other:

| Document | Answers | Does not answer |
| --- | --- | --- |
| [`requirements.md`](./requirements.md) | **What** each artifact must let a consumer determine. | How the artifact is shaped to make that possible. |
| This document | **How** the artifact is shaped: keys, types, enums, invariants. | Why a consumer should want the information in the first place. |

`requirements.md` states, for example, that `anr.json` exists so an engineer can tell a
live ANR from a historical one. This document specifies the `freshness` enum, the
`capture_kind` field, and the severity cap that make that distinction impossible to lose.

A field that appears in this document but cannot be traced to an AOSP source or an archive
section does not belong here. It belongs in that artifact's *Open questions* subsection.

---

## 2. Shared Conventions

This section is normative and self-sufficient. An artifact author needs to read this section
and their own artifact's [§4](#4-the-seven-artifacts) entry, and nothing else.

### 2.1 The envelope

Every JSON artifact is a single object. Six of the seven carry **exactly the same thirteen
top-level keys**, in this order:

```
schema_version, artifact, generator, source, capture, scope, collection,
counts, severity_summary, freshness_summary, negative_evidence, findings, notes
```

`README.md` is Markdown and has no envelope; it references these structures but does not
serialise them.

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | yes | Version of this schema. Bumped when a key is renamed or removed. |
| `artifact` | enum | yes | One of `system`, `apps`, `anr`, `crashes`, `memory`, `cpu`. Identifies which artifact this is. |
| `generator` | object | yes | Tool provenance. See [§2.1.1](#211-generator). |
| `source` | object | yes | Identity of the archive read. See [§2.1.2](#212-source). |
| `capture` | object | yes | The capture instant and its context. See [§2.1.3](#213-capture). |
| `scope` | object | yes | What was and was not searched. See [§2.1.4](#214-scope). |
| `collection` | object | yes | What the collector itself did, so "is this number self-inflicted?" is answerable. See [§2.1.5](#215-collection). |
| `counts` | object | yes | Match blocks. See [§2.3](#23-counts-are-match-blocks). |
| `severity_summary` | object | yes | Tally by severity. Must sum to `findings.length`. |
| `freshness_summary` | object | yes | Tally by freshness state. Must sum to `findings.length`. |
| `negative_evidence` | array | yes | Statements of absence, each with an authority level. See [§2.1.6](#216-negative_evidence). |
| `findings` | array | yes | The payload. May be empty; see below. |
| `notes` | array of string | yes | Free-text caveats, each prefixed `"<CODE>: <text>"`. |

**On empty arrays.** `findings: []` is a legal and meaningful result — but it is not a
silent one. An artifact with no findings must carry at least one `negative_evidence` entry
or a `notes` entry explaining why there are none. See [§2.1.6](#216-negative_evidence).

**On the `findings` element shape.** Every element of `findings[]` carries the same core
fields, plus whatever its artifact defines:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | yes | `<artifact_prefix>-<4-digit ordinal>`, assigned in archive order so ids are stable across runs on the same archive. Prefix: `anr`, `crash`, `mem`, `cpu`, `app`, `sys`. |
| `kind` | string enum | yes | A noun for a record type, never a symptom. Artifact-specific; the closed set is listed in each artifact's entry. |
| `title` | string | yes | One line, ≤ 200 characters, no newline, no unescaped control character. |
| `severity` | enum | yes | See [§2.6](#26-severity). |
| `unconstrained_severity` | enum | yes | The severity this finding would have absent the freshness cap. Always present. Equals `severity` when no cap applied. |
| `confidence` | enum | yes | `confirmed` (a validated record), `inferred` (a derived conclusion), `reported` (a third-party assertion we did not verify). `reported` **must** be used when the only evidence is an event-log line. |
| `process_key` | string \| null | yes | See [§2.8](#28-identity). `null` only when the finding is genuinely device-wide. |
| `event_time` | string \| null | yes | When the **condition** happened. Distinct from when the evidence was captured. |
| `attributes` | object | yes | Record-type-specific scalars. Every numeric key carries a unit suffix. |
| `metrics` | object | yes | The measurements that justified the severity. A `high` or `critical` finding with empty `metrics` is a schema violation. |
| `evidence` | object | yes | Exactly one. See [§2.7](#27-evidence). |
| `corroborating_evidence` | array | yes | May be empty. See [§2.7](#27-corroborating_evidence). |
| `provenance` | object | yes | All keys in [§2.4](#24-provenance). |

#### 2.1.1 `generator`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | Constant `"bugreport-insight"`. |
| `version` | string | yes | Cargo package version, no `v` prefix. |
| `run_started_at` | string (RFC 3339, explicit offset) | yes | **Host** wall clock. Never the device clock. |
| `run_finished_at` | string (RFC 3339, explicit offset) | yes | Host wall clock. |
| `host_timezone_offset` | string (`±HH:MM`) | yes | The host's offset at run time. |

#### 2.1.2 `source`

Archive identity. Read from the ZIP, not derived.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `archive_name` | string | yes | Basename of the ZIP as supplied. | host |
| `archive_size_bytes` | integer ≥ 0 | yes | File size on the host. | host |
| `archive_sha256` | string | yes | Lowercase hex, 64 characters, over the compressed bytes as stored. | host |
| `format_version` | string \| null | yes | Verbatim body of `version.txt`, stripped. `null` if absent, which means format 0 per `bugreport-format.md:114-121`, and **must** be paired with a `notes` entry `ARCHIVE_FORMAT_0`. | `dumpstate.cpp:2872`; value from `dumpstate.h:157` |
| `main_entry` | string \| null | yes | Body of `main_entry.txt`. `null` if absent. | `dumpstate.cpp:2752-2755` |
| `main_entry_size_bytes` | integer \| null | yes | Uncompressed size of the main entry. `null` if `main_entry` is `null`. | host |
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
| `build_type` | string \| null | yes | `user`, `userdebug`, or `eng`. Gates almost everything — see [§3](#3-build-variant-behaviour). | `DumpstateUtil.cpp:212-217` |
| `device` | string \| null | yes | Device name, parsed from the fingerprint segment after the product. | `dumpstate.cpp:810` |
| `hardware` | string \| null | yes | From `androidboot.hardware` in the `Bootconfig:` block. | `dumpstate.cpp:826-827` |
| `bootloader` | string \| null | yes | `ro.bootloader`. | `dumpstate.cpp:798`, printed `:811` |
| `radio` | string \| null | yes | `gsm.version.baseband`. | `dumpstate.cpp:797`, printed `:812` |

**Rule.** `source` carries no time. The capture instant lives in `capture`, and the evidence
times live in each finding's `provenance`. Keeping them apart is what stops an archive-level
clock from being mistaken for an evidence clock.

#### 2.1.3 `capture`

The capture instant and the context needed to interpret anything else.

`system.json` owns the canonical copy. **Every other artifact references it as
`capture_ref: "system.json#/capture"` and does not re-derive it.** Uptime-at-capture, the log
window, and the boot instant are computed exactly once across the whole output set, because
a derived value that appears twice will eventually appear twice with two different answers.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `captured_at` | string (RFC 3339, explicit offset) \| null | yes | The archive's capture instant. `null` if no clock is derivable — a legitimate outcome, not an error. | `== dumpstate:` banner, `dumpstate.cpp:802-805` |
| `captured_at_time_base` | enum | yes | Which clock supplied `captured_at`. See [§2.4](#24-provenance). | — |
| `uptime_at_capture_seconds` | integer \| null | yes | Device uptime in seconds at capture. Never inferred from logcat timestamps. | `------ UPTIME (uptime) ------`, `dumpstate.cpp:1687`; header line `dumpstate.cpp:828-830` |
| `log_window` | object | yes | The archive's logcat window. See below. | `logprint.cpp:1552-1554` |
| `boot` | object | yes | Boot-instant derivation. See below. | — |
| `is_cold_boot` | boolean | yes | `uptime_at_capture_seconds < 300`. | derived |

`capture.log_window`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `sections` | array of string | yes | The logcat sections present, in file order. **The list repeats** — the reference archive has two `SYSTEM LOG` blocks. |
| `first_line_at` / `last_line_at` | string | yes | Wall-clock stamp of the first/last parseable logcat line, `MM-DD HH:MM:SS.mmm`. Year-less; see [§2.9](#29-null-absent-and-raw). |
| `duration_seconds` | number | yes | `last − first`. |
| `is_contiguous` | boolean | yes | `false` when a second, `-T`-bounded `SYSTEM LOG` follows. |
| `overlaps_boot_instant` | boolean | yes | `true` when `first_line_at` precedes the derived boot instant. Logcat is stamped from the kernel wall clock before `SystemClock.uptimeMillis()` reaches 0, while AMS relative times are uptime-based. **Log timestamps and AMS relative times are not the same clock.** |
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

`system.json` extends `scope` with a per-section `sections[]` array carrying a `status` from
[§2.2](#22-section-status-enum). Every other artifact inherits the section list from
`system.json#/scope` via `capture_ref` rather than repeating it.

#### 2.1.5 `collection`

Built from dumpstate's own per-section duration footers — the
`------ N.NNNs was the duration of 'TITLE' ------` lines emitted at
`dumpstate.cpp:3831-3835`. Same schema in every artifact.

The question this answers is narrow and important: **did the collection process change the
number it is reporting?** In the reference archive, two sections walk every process's page
tables — `BUGREPORT_PROCDUMP` (5.187 s) and `DUMPSYS HIGH meminfo` (3.525 s) — and swap
usage rises from 18,176 K to 230,912 K across exactly that interval. A naive schema that
diffs two swap snapshots reports ~207 MiB of "device thrashing" that is entirely the
collector.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `page_scanning_sections` | array of object | yes | Sections known to walk process page tables, with name, line range, and duration. |
| `page_scanning_total_ms` | number | yes | Sum of the above. |
| `swap_timeline` | array of object | yes | Ordered swap snapshots across the dump. Every delta is an **edge between two identified nodes**; there is no scalar a consumer can diff naively. |
| `swap_growth_kb` | integer \| null | yes | `last − first` across the timeline. `null` if fewer than two usable points. |
| `self_inflicted_growth_kb` | integer \| null | yes | Growth attributable to `page_scanning_total_ms`. |
| `assessment` | enum | yes | `collector`, `device`, `indeterminate`, or `no_growth_observed`. Computed, not asserted. |
| `assessment_basis` | string | yes | One sentence naming the sections, durations, and negative evidence. |

The rule, stated in the artifact:

```
collector          ⇔  swap_growth_kb > 0
                     ∧ the interval containing the growth overlaps ≥ 1 page-scanning section
                     ∧ kill_evidence shows no reaction (no lmkd kill, /proc/vmstat oom_kill == 0)
device             ⇔  swap_growth_kb > 0 ∧ there IS a reaction
indeterminate      ⇔  swap_timeline has < 2 usable points
no_growth_observed ⇔  swap_growth_kb <= 0
```

`cpu.json` adds one field: `cpu_clean_channels` / `cpu_perturbed_channels`, naming which
channels were measured *before* the dump and which *during* it, plus a `mechanism` sentence.
This is the opposite case from `memory.json` and it is why the field is in the shared
convention rather than in one artifact.

#### 2.1.6 `negative_evidence`

A required array. Each entry:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `scope` | string | yes | What the statement covers: a channel, a section, an archive-wide search. |
| `statement` | string | yes | The verbatim platform string where one exists, or the precise description of what was searched. |
| `source` | string \| null | yes | Where to see it — an archive locator. |
| `authority` | enum | yes | See below. |

`authority` is either:

| Value | Meaning |
| --- | --- |
| `platform_assertion` | A platform string asserting absence — e.g. `  <no ANR has occurred since boot>`. **Authoritative.** |
| `archive_absence` | We looked and found nothing. **Indicative only.** |

**An empty `negative_evidence` is itself a defect in the artifact.** This array is the
mechanism that stops "65 ANR files on disk" from being read as "an ANR happened". The
reference archive contains 65 ANR trace files, all 126 days old, from a boot that ended
before this one — and simultaneously two dumpsys sections asserting in so many words that
no ANR has occurred since boot. Both facts are true. Only an artifact that carries both can
be read correctly.

### 2.2 Section status enum

Each entry of `system.json#/scope/sections[]` carries a `status`:

```
ok | empty | skipped_user_build | skipped_root_denied | skipped_missing_source |
skipped_budget | not_found | unreadable
```

| Value | Meaning |
| --- | --- |
| `ok` | Section present and non-empty. |
| `empty` | Section header emitted, zero payload. |
| `skipped_user_build` | dumpstate declined to run the command on a `user` build. It prints `Skipping '<cmd>' on user build.` **in place of** the banner — see [§3](#3-build-variant-behaviour). |
| `skipped_root_denied` | The command needed root and the root drop failed. Present-but-empty with an error marker. |
| `skipped_missing_source` | A `DumpFile` whose `open()` failed. Present-but-empty with `*** Error dumping %s: %s`. |
| `skipped_budget` | A time budget or size budget excluded the section. |
| `not_found` | Expected and absent. |
| `unreadable` | Present but not parseable — e.g. the entry spans a NUL byte. |

`skipped_user_build` and `skipped_root_denied` are load-bearing. A consumer must be able to
tell **"none exist"** from **"we were not allowed to look"**. Two platform markers make that
distinction possible and must be mapped to `ok` when present:

- `*** NO TOMBSTONES to dump in /data/tombstones/` — authoritative absence.
- `*** NO ANRs to dump in /data/anr/` — authoritative absence.

### 2.3 Counts are match blocks

**No artifact may report a bare number.** A raw grep count becomes a finding; that is the
failure mode this convention exists to prevent. Measured on the reference archive:
`tombstone` matches **749** times and yields **0** real tombstones; `signal ` matches **656**
times and yields **0** fatal signals; `watchdog` matches **315** times and yields **6**
actionable findings.

Every counted quantity is a **match block** under `counts.<name>_hits`:

```
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
| `not_emitted_by_platform` | integer | yes | The evidence channel does not exist in this archive. Distinct from "searched and found nothing". |
| `suppressed_noise_note` | string | yes when `suppressed_noise > 0` | Why the residue is not a finding. |
| `coverage` | object | yes | See below. |
| `regions[]` | array | yes | Every contiguous run of matching lines, with its section. |

`coverage`:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `window_start` / `window_end` | string \| null | yes | The channel's own observation window. `null` for a point-in-time snapshot. |
| `archive_window` | object | yes | The archive's `capture.log_window`. |
| `coverage_ratio` | number \| null | yes | **Required** whenever a channel observes a window narrower than the archive's. |

**Two invariants, both mechanically checkable:**

1. `matched_evidence + suppressed_noise + not_emitted_by_platform == raw_count`, with
   `not_emitted_by_platform` counted as zero within a region.
2. `coverage.coverage_ratio` **must** be present whenever a channel observes a window
   narrower than the archive's log window.

Invariant 2 has a measured instance. The real `am_proc_*` event-log buffer covers only
**20.304 s** of the archive's **64.662 s** window — ratio **0.314** — which is why **40**
`am_proc_died` entries coexist with **271** real process deaths. A count published without
its coverage is the same class of error as a count published without its noise split, one
level up.

**Count-key suffixes are closed.** `_hits` (raw string matches), `_records` (promoted
findings), `_entries` (archive entries). No other suffix. A bare `count`, `total`, `matches`,
or `num` key is a schema violation.

**Suppression reasons are a closed vocabulary.** New reasons require a schema revision.

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

**Suppression must be visible, not silent.** A `matched_evidence` of 0 is a **result**. It
must be surfaced as a `findings[]` entry with `kind: "signal_absence"`, `severity: "info"`,
and a `freshness_basis` naming the regions searched.

### 2.4 Provenance

Required on **every finding** and on **every collection object**. Flat — not nested under a
`capture` wrapper, because per-finding provenance is the entire point.

```
provenance = {
  evidence_origin,                  // the AOSP channel id, e.g. "C1_anr_logcat"
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
| `age_seconds_at_capture` | integer \| null | yes | `capture.captured_at − provenance.captured_at`. `null` if either side is `null`. Never negative; if the arithmetic goes negative, set `0` and emit `notes` `CLOCK_SKEW`. |
| `capture_kind` | enum | yes | **Required, not optional.** See below. |
| `freshness` | enum | yes | See [§2.5](#25-freshness). |
| `freshness_basis` | object | yes | `{ input, basis, threshold_seconds }`. All three required. |
| `cross_boot_reference` | integer \| null | yes | Milliseconds of uptime on a **different boot epoch**. `null` otherwise. |
| `collector_perturbed` | boolean \| null | yes | Did the collection plausibly change this number? |
| `collector_perturbed_basis` | string \| null | yes | Why, naming the sections and durations. |

#### `captured_at_time_base` — the 11-value enum

Five distinct time bases coexist in one archive. This enum exists to make each of them
individually nameable. **Never assume the logcat clock.**

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

**The logcat year problem is real and must be handled explicitly.** Base 2 carries no year and
no offset. Resolution, in order:

1. If `capture.captured_at` is known, take its year **and** its UTC offset.
2. If a logcat timestamp's month-day exceeds `capture.captured_at`'s month-day by more than
   one day, the log wrapped a year boundary: subtract one year. (The reference archive's
   window is `09-24`, same day as capture — no wrap.)
3. If `capture.captured_at` is `null`, the year is `null` ⇒ `captured_at: null`,
   `captured_at_time_base: "undetermined"`, `freshness: "undated"`. **Never guess a year.**

Every finding so marked must also emit `notes` entry `LOGCAT_YEAR_UNRESOLVED`.

#### `evidence_time_source` — the closed enum

Which of these produced `captured_at`:

```
logcat_line | event_log_entry | dumpsys_Timestamp | file_mtime |
filename_timestamp | embedded_trace_stamp | none
```

#### `capture_kind` — the closed enum

What *kind* of measurement this is, independent of when it was taken:

```
point_in_time_snapshot | background_sampler | persisted_aggregate | live_log |
disk_artifact | command_output
```

`capture_kind` is **required**. Every staleness decision in practice turned on *what kind of
measurement it is*, not on when it was taken:

- `persisted_aggregate` is what catches the 128-day-old `procstats` blocks — whose
  `AGGREGATED OVER LAST 3 HOURS` label resolves to a `Start time` of 2026-05-19.
- `background_sampler` is what rescues `dumpsys cpuinfo` — its 1.772 s sample window ended
  **18.415 s before dumpstate even started**, so it is clean of collection cost.

### 2.5 Freshness

Four states, mechanically assigned:

```
freshness ∈ now | in_window | stale | undated
```

| Value | Legal when | Meaning |
| --- | --- | --- |
| `now` | The evidence's clock demonstrably falls inside `capture.log_window`, and at the capture instant. | Taken as part of this dump. |
| `in_window` | The evidence's clock demonstrably falls inside `capture.log_window`. | Inside the captured interval, not at its head. |
| `stale` | The clock is known and outside the window, **or** the cross-boot check fires, **or** `age_seconds_at_capture >= 3600`. | Provably older than the capture. |
| `undated` | The evidence carries no clock at all. | A legal, non-error value. |

**There is no "unknown defaults to recent" member.** That omission is the bug this
convention exists to prevent. Raw `/proc/meminfo`, `PROCRANK`, `IOTOP`, and `top` print **no
clock at all**; inventing one is worse than admitting it.

**Threshold: 3,600 s (1 hour).** Fixed in this specification, not configurable per run, so
artifacts are comparable. It is deliberately coarse: the reference archive's log window is
64.662 s and the gap it must separate is 126 days. One hour is the coarsest value that still
separates "same boot" from "a previous boot" on a device with a plausible uptime.

**Cross-boot escalation is mandatory, not advisory.** When `cross_boot_reference` is non-null
it is a monotonic millisecond value from a boot epoch that is not the one
`capture.uptime_at_capture_seconds` describes. If
`cross_boot_reference > 1000 × max(uptime_at_capture_seconds, 1)`, the evidence is from a
different boot and **must** be forced to `stale` regardless of wall-clock age. Reference
archive: `cross_boot_reference = 31190738` (8 h 39 m 50 s) against
`uptime_at_capture_seconds = 30` — a factor of 1.0 million — so `stale` twice over. The
`freshness_basis` for such a finding must contain the phrase `cross-boot reference`.

`freshness_basis` must name three things: the `input` (which clock or check was used), the
`basis` (why that input decides it), and the `threshold_seconds`.

**Three invariants, all mechanically checkable:**

- `freshness ∈ {now, in_window}` ⟹ `captured_at != null` **and** `age_seconds_at_capture != null`
  **and** `age_seconds_at_capture <= 3600` **and** `scope.truncated == false`.
- `freshness == "stale"` ⟹ the clock is known and `age_seconds_at_capture > 3600`, **or** the
  cross-boot check fired.
- `freshness == "undated"` ⟹ `captured_at == null` **and** `age_seconds_at_capture == null`.

**A `stale` or `undated` finding is labelled, never dropped, and never silently demoted.**
Severity is capped at `medium` for both, and the uncapped value is always retained in
`unconstrained_severity`.

### 2.6 Severity

A closed five-level enum, plus one mandatory sibling:

```
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

**The cap.** A finding whose `freshness` is `stale` or `undated` **cannot exceed `medium`**.
A 126-day-old ANR is evidence that something was broken 126 days ago; it is not evidence
that anything is broken now. The uncapped value is not discarded — it lives in
`unconstrained_severity`.

Two traps are encoded as rules:

1. **A raw tag count is never a severity input.** `watchdog` matched 315 times and yields 6
   findings; the number 315 is not a severity.
2. **A measurement taken by a background sampler is never severity-capped for being old.**
   `dumpsys cpuinfo`'s window ended before the dump began, so its age is not a defect.

`severity` is always accompanied by the numeric measurement that justified it, in `metrics`.
A severity with no measurement is a schema violation.

### 2.7 Evidence

Because the tool is read-only over a ZIP, evidence is a **locator** plus a bounded excerpt —
never a copy of the source file.

```
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
| `locator.line_start` / `line_end` | integer \| null | yes | 1-based inclusive. `null` unless `kind == "line_range"`. |
| `locator.byte_offset` | integer \| null | yes | Uncompressed byte offset of `line_start` within the entry, so a consumer can seek without re-scanning. |
| `locator.byte_length` | integer \| null | yes | Uncompressed byte length of the located span. |
| `excerpt` | string | yes | Verbatim text. Never re-wrapped, never reflowed, never ellipsised inside. |
| `excerpt_truncated` | boolean | yes | `true` if the excerpt was cut. |
| `excerpt_bytes` | integer | yes | Actual length of `excerpt` in bytes, not characters. |
| `excerpt_complete` | boolean | yes | `true` only when the **entire** located span fits under the bound. |

**`MAX_EXCERPT_BYTES = 512`, hard.** Justification, not taste:

- It is larger than the longest thing a finding needs to prove. Measured: the longest section
  header is 74 bytes; a `DUMP OF SERVICE` header is 80; a typical `am_crash` event line is
  ~140; a tombstone `signal N (SIGABRT), code …` line is ~80. All fit.
- It is small enough to be safe against the archive's documented pathology: one line is
  **192,666 characters** and there are **4 NUL bytes**. A 512-byte cap makes a worst-case
  excerpt cost bounded no matter how hostile the input is.
- It is roughly four lines of wrapped Markdown, so `README.md` stays readable.

Truncation is byte-wise at 512, then backed up to a UTF-8 boundary — never split a
multi-byte sequence. If backing up would fall below 480 bytes, `excerpt` becomes `""` with
`excerpt_truncated: true` and `excerpt_complete: false`. A `byte_length` above 512 does not
invalidate a finding; it only makes the excerpt partial.

A finding whose span contains one of the archive's 4 NUL bytes must set
`excerpt_complete: false` and emit `notes` entry `NUL_BYTES_IN_EVIDENCE`, because the
consumer's terminal may truncate at the NUL.

#### 2.7 `corroborating_evidence`

Optional array, same object shape, for the same bytes found in a second place.

The reference archive makes this necessary. `FS/data/anr/anr_2026-05-20-05-13-42-314` exists
both as a ZIP entry and inlined into the main `.txt` under
`------ VM TRACES AT LAST ANR (/data/anr/anr_2026-05-20-05-13-42-314: 2026-05-20 05:13:43) ------`
(archive line 309986). The duplication is deliberate in AOSP: `dumpstate.cpp:1182-1189` dumps
`anr_data_[0]` into the body *and* adds the whole range to the zip as `HISTORICAL ANR`. The
sort at `dumpstate.cpp:432-435` is newest-mtime-first, which is why `:1183` can say *"The
'last' ANR will always be present in the body."*

So a finding for the newest ANR legitimately has **two** evidence locators and must populate
both. **A consumer that counts findings must not double-count: `findings` is the counting
unit; `evidence` is not.**

### 2.8 Identity

```
process_key = "<boot_epoch>/<start_seq>"                        // primary
            | "<process_name>#<uid>@<start_ordinal>"           // fallback, start_seq absent
```

| Form | When | Why |
| --- | --- | --- |
| `<boot_epoch>/<start_seq>` | Always preferred. | `start_seq` is AMS's own monotonic counter (`ProcessList.java:2155-2157`), so it is globally unique for the life of the AMS process. Paired with `boot_epoch` because it resets to 0 on a `system_server` restart (`ProcessList.java:1858`). |
| `<process_name>#<uid>@<start_ordinal>` | Only when `start_seq` is absent — a `dumpsys`-only sighting. | Weaker: the ordinal is window-local and `null` for a process that started before the window. |

`boot_epoch` is `system.json#/capture/boot/at` — **the boot instant, the wall clock at which
uptime was zero.** It is *not* the timestamp of any process start, event, or log line; on the
reference archive it is `2026-09-24T00:26:47.33+07:00` ± 0.10 s, which is distinct from the
`Start proc 944` line at `00:26:59.882` and from the `== dumpstate:` banner at `00:27:17`.
Because `boot_epoch` is half of the key, a wrong value silently re-identifies every process in
the artifact; it must be the solved boot instant and nothing else. `start_seq` is read from the
`startSeq=` field of a `dumpsys activity processes` record (`ProcessRecord.java:540`).

**`pid` is an observation attribute, never a key.** PIDs are recycled. `pid` without
`process_key` is an incomplete observation.

Corroborating identities may be carried but must never be joined on:

| Field | Source | Why it is weaker |
| --- | --- | --- |
| `process_record_id` | `ProcessRecord{<identityHashCode> <pid>:<processName>/<uid>}` — `ProcessRecord.java:1396-1408`. 948 occurrences in the reference archive. | `identityHashCode` is a JVM object hash: unique per instance, meaningless across a `system_server` restart. |
| `start_elapsed_time` | `startElapsedTime=` — `ProcessRecord.java:527-528`. | Rendered relative to the dump instant, e.g. `-34s559ms`. Not absolute. |
| `app_id` | `Settings.java:5129`. | Stable per (package, user) for the life of an install, but **reassigned on reinstall** and **shared** — every `android.uid.system` app has `app_id = 1000`. |
| `record_identity_hash` | `ProcessRecord.java:1402-1403`. | Diagnostic only. Interestingly it *is* stable across a whole crash loop for one process, which is a useful cross-check that AOSP reuses the record. |
| `pid` | everywhere | Recycled. Display only. |

**`package_name` and `process_name` are different keys and must not be conflated.** A package
can host `:remote` and `:sandboxed_process0` children. When a `process_name` is not a package
name, `package_name` is `null` and the component is carried in `attributes`.

### 2.9 Null, absent, and raw

Six rules, each of which exists because the archive forced it.

1. **Unknown is `null`.** Keys are never omitted. The string `"unknown"` is never a value.
2. **Tri-state for build-type-dependent fields.** `null` means *not observable*, never
   `false`. `"is_emulator": false` on a `user` build is a lie a consumer will act on. Pair
   such a field with a `sections[].status` of `skipped_user_build`.
3. **Byte-preserving raw values.** Any locale- or build-dependent formatted string is stored
   as `*_raw` (string) plus a best-effort parsed sibling plus `parsed_confidence`.
4. **Every derived number carries its derivation.** If a schema computes a value rather than
   reading it, it emits a sibling `*_derivation` (or `*_derived: true`). The literal
   `2,007,345` for total PSS **does not appear in the archive** — a reader must be able to see
   it is `used_pss_kb + cached_pss` from two different `ActivityManagerService` lines rather
   than trust it.
5. **Zero counts carry an absence reason.** Any count of `0` explainable by dump ordering —
   e.g. `am_meminfo_count: 0` because the event log was captured before the dump that writes
   it — carries a `*_absence_reason`.
6. **Cross-artifact references are pointers, not copies.** `capture_ref: "system.json#/capture"`
   points at the canonical object. The same derived value must not appear twice with two
   slightly different answers.

The AOSP archive forces rule 1 concretely. `ro.bootloader` and `gsm.version.baseband` render
as the literal string `(unknown)` in the header (`dumpstate.cpp:797-798`, whose defaults are
literally `"(unknown)"` at `:795-800`). **That AOSP placeholder is a device-side sentinel and
must be normalised to JSON `null`** at the `source.device` boundary. The reference archive's
`radio` is `null` for exactly this reason.

The symmetry matters: `provenance.captured_at: null` and `source.device.radio: null` mean the
same thing — *the evidence is absent, and that absence is itself a finding's input*.

### 2.10 The two section-parsing traps

Both are real in the reference archive and both are counted, measured, and specified here.

#### Trap 1 — the two `SYSTEM LOG` sections overlap by 184 lines

`dumpstate.cpp:1910` runs `DoLogcat()` early (*"Capture first logcat early on; useful to take
a snapshot before dumpstate logs take over the buffer"*, `:1908-1909`), records
`logcat_ts` at `:1912`, then runs `DoSystemLogcat(logcat_ts)` at `:2006` — emitted last,
narrowed by `-T <logcat_ts>` at `:1004-1012`.

In the reference archive the first section ends at `00:27:20.131` and the `-T`-narrowed tail
starts at `00:27:20.000`. A line-set comparison over that window finds **184 lines present in
both** (verified: 184 lines matching `^09-24 00:27:20\.0` in each block, and all 184 are in
the intersection).

**De-duplication is mandatory.** The rule: scan both sections, key on
`(section_title, section_ordinal)`, and de-duplicate on
`(timestamp, pid, tid, tag, message)`. A parser that concatenates them double-counts
anything in that ~131 ms window.

A title is never a key. `SYSTEM LOG` appears at archive line **9116** (ordinal 0, the early
snapshot) and **653373** (ordinal 1, the `-T` tail).

One further trap in the same area: the second section's header reads
`------ SYSTEM LOG (logcat … -T 2026-09-24 00:27:20.000) ------`. That timestamp is
`logcat_ts` — the **start of the requested window**, not the time of any line. It must not be
used as a `captured_at`.

#### Trap 2 — five time bases coexist in one archive

| # | Base | Format | Source | Carries a date? |
| --- | --- | --- | --- | --- |
| 1 | Wall clock, second precision, `CLOCK_REALTIME` at capture | `== dumpstate: %Y-%m-%d %H:%M:%S` | `dumpstate.cpp:802` from `now_ = time(nullptr)` (`:3805`) | yes; second, **no offset**, no sub-second |
| 2 | Logcat wall clock, local time, **year-less** | `MM-DD HH:MM:SS.mmm` | `logprint.cpp:1464` — `&"%Y-%m-%d %H:%M:%S"[p_format->year_output ? 0 : 3]` slices off the year; `:1472` appends `.%03ld` | month-day only; **no year, no timezone** |
| 3 | Monotonic since boot, nanosecond | `%.3fs` durations | `dumpstate.cpp:3827`, `(Nanotime() - started_)` | **no absolute date at all** |
| 4 | Process-relative monotonic, seconds | `Process uptime: %ds` | `tombstone_proto.cpp:857-863`, rendered `tombstone_proto_to_text.cpp:616-617` | **no absolute date** |
| 5 | File mtime, second precision | `%Y-%m-%d %H:%M:%S` in file-section headers; `%Y/%m/%d %H:%M:%S` in ZIP entry metadata | `DumpstateInternal.cpp:148`; `dumpstate.cpp:867-868` + `:4663-4669` | yes; second, no offset |

Plus two that are *not* capture times but are often mistaken for them:

- **`Uptime: up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: …`** — `dumpstate.cpp:828-830`
  runs `uptime -p`. The dedicated `------ UPTIME (uptime) ------` section is
  `dumpstate.cpp:1687`. This is base 3 formatted as a duration, **so a naive `MM-DD` parser
  will not see it**.
- **`----- dumping pid: 17511 at 31190738 -----`** —
  `StackTracesDumpHelper.java:599-602`, `"SystemClock.uptimeMillis()"`. That is **8 h 39 m
  50.738 s of uptime on a boot that ended 126 days ago**, against 30 s of uptime on the boot
  that produced the bugreport. It is the single cleanest proof in the archive that the ANR
  payload predates the current boot, and it is what `cross_boot_reference` carries.

`captured_at_time_base` exists to make all five individually nameable.

Measured ages of every non-live source in the reference archive, against
`capture.captured_at = 2026-09-24T00:27:17+07:00`:

| Source | Capture instant | Age at capture |
| --- | --- | --- |
| Log window | `00:26:47.418` → `00:27:52.080` (base 2) | 0–65 s |
| `VM TRACES JUST NOW` | `2026-09-24 00:27:25` (base 5) | ~8 s |
| `RT_TABLES` | `2026-09-24 00:27:10` (base 5) | ~67 s |
| `BINDER *` (5 sections) | `2026-09-24 00:26:52` (base 5) | ~145 s |
| **ANR files (newest)** | `2026-05-20 05:13:43` (base 5) | **126 d 19 h 13 m 34 s** |
| **ANR files (oldest)** | `2026-05-20 00:54:11` (base 5) | **126 d 23 h 33 m 06 s** |
| **Tombstones (all 50)** | `2026-05-20 04:00:14` (base 5) | **126 d 20 h 27 m 03 s** |
| **`* BUILD-TIME RELEASE FLAGS`** | `2026-01-27 20:47:21` (base 5) | **239 d 3 h 39 m 56 s** |
| `dumpsys procstats` `AGGREGATED OVER …` | `2026-05-19 00:16:06` (base 5) | **~128 d** |

The ANR span is **4 h 19 m 30.631 s** (oldest `anr_2026-05-20-00-54-11-683` → newest
`anr_2026-05-20-05-13-42-314`; 65 files, `total 20336` KB per the `ANR FILES` listing at
archive line 318024). The filename is itself a timestamp, so ANR age is available three
independent ways: filename, ZIP entry mtime, and the `ls -lt` column.

**The design rule that follows:** freshness is a per-finding property with an explicit basis,
never an artifact-level disclaimer.

### 2.11 Units and time formats

- **Every numeric field carries a unit suffix** in its key name, from a closed list:
  `_bytes`, `_kb`, `_mb`, `_ms`, `_us`, `_ns`, `_seconds`, `_pages`, `_count`, `_percent`,
  `_cycles`, `_hz`, `_kib`, `_mib`, `_hundredths`.
- **`_kb`/`_mb` are decimal (10³/10⁶); `_kib`/`_mib` are binary (2¹⁰/2²⁰).** Never write `_kb`
  for what the kernel means by KiB.
- **Unitless numerics** do not exist. The only exception is a count, which uses `_count` and
  means *occurrences*.
- **Numbers parsed from human-formatted text keep the source's unit and say so.** AOSP's
  `Total PSS by process:` block prints `212,862K: …` (archive line 325705); that `K` is
  1024-based, so it is recorded as `pss_kib`, not `pss_kb`. Do not silently convert; do not
  silently relabel.
- **Percentages** are integers 0–100 with `_percent`, never fractions. Ratios use `_ratio`
  and are floats 0.0–1.0. Never both.
- **All timestamps are RFC 3339 with an explicit numeric offset**, e.g.
  `2026-05-20T05:13:43+07:00`, `2026-09-24T00:27:01.455+07:00`. **Never a bare local string.**
- **Sub-second precision is preserved when the source has it.** Logcat: milliseconds.
  `----- pid N at <ts> -----`: nanoseconds (`dumpstate.cpp:2222`). ZIP mtime: whole seconds.
  Do not pad or round.
- **Relative times use a distinct suffix and a named anchor:** `age_seconds_at_capture`
  (anchor: `capture.captured_at`), `duration_seconds` (anchor: the enclosing
  `------ N.NNNs was the duration of '…' ------` trailer), `process_uptime_seconds` (anchor:
  process start).
- **Time bases are never mixed in one comparison.** A year-less logcat wall clock is not
  comparable to a monotonic millisecond counter. A finding that would need that comparison
  uses `cross_boot_reference` and is capped at `medium`.
- **`localtime()` is a bug source.** `dumpstate.cpp:802` and `:2744`, and
  `DumpstateInternal.cpp:148`, all use `localtime()`. The device's zone is whatever the
  device had. The reference archive shows `+0700` in the `----- pid N at … -----` lines and
  `00:27:17` in the banner: **the offset is only recoverable from the per-process lines**
  (`dumpstate.cpp:2227`, `strftime("%z", &tm)`). A finding whose time is known only from a
  banner or an mtime header has no offset in the source — resolve it from
  `capture.captured_at`'s offset, and if that is `null`, emit `captured_at: null`.

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

> ### ⚠ This archive is a cold boot
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
  [`system.json`](#42-systemjson).

---

### 4.2 `system.json`

#### Purpose

`system.json` is the **device fact sheet**: what was captured, when, on what, by which build —
everything a consumer needs *before* it can interpret any other artifact. It answers *"is this
even the device I think it is, and is this window of time trustworthy?"*, not *"what went
wrong"*. It contains **no findings about defects and no ranked list** — that is the whole
point of keeping it separate from `anr.json`, `crashes.json`, `memory.json`, and `cpu.json`.

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture` | object | yes | **The canonical capture object.** Every other artifact points here. |
| `build` | object | yes | Platform build identity, including the full build-type gate state. |
| `product` | object | yes | Product/brand identity. **`getprop`-only** — all `null` on a `user` build. |
| `device` | object | yes | Physical/virtual nature, bootconfig excerpt. |
| `kernel` | object | yes | Kernel identity and boot args. |
| `users` | object | yes | Multi-user / profile inventory from `dumpsys user`. |
| `displays` | array | no | Display outputs, from the SurfaceFlinger CRITICAL dump. |
| `hal` | object | yes | HIDL HAL inventory, with `covers_aidl: false`. |
| `storage` | object | yes | Mount topology and what was copied under `FS/`. |
| `scope` | object | yes | Extended: carries the full `sections[]` index with a `status` per section. |
| `archive_entries` | object | yes | Inventory of the side artifacts in the ZIP. |
| `evidence_gaps` | array | yes | Facts that could not be read, each with a reason. |

#### Field tables

##### `capture`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `captured_at` | string \| null | yes | `== dumpstate:` banner, RFC 3339 with explicit offset. `null` if the banner is absent — legitimate, not an error. | `dumpstate.cpp:802-805` |
| `captured_at_time_base` | enum | yes | `dumpstate_header` or `archive_filename`. | `dumpstate.cpp:802`; filename form `:2835` |
| `uptime_at_capture_seconds` | integer \| null | yes | Uptime in seconds at capture. **Never inferred from logcat timestamps.** | `dumpstate.cpp:1687`, `:828-830` |
| `uptime.derivation` | string | yes when `uptime_at_capture_seconds` is non-null | The formula and its anchor lines. Required because the value is computed. | — |
| `log_window` | object | yes | See [§2.1.3](#213-capture). | `logprint.cpp:1552-1554` |
| `boot` | object | yes | Derived boot instant plus its derivation and any epoch corroboration. | — |
| `is_cold_boot` | boolean | yes | `uptime_at_capture_seconds < 300`. | derived |

##### `build`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `display_id` | string | yes | `Build:` value. | `dumpstate.cpp:795,808` |
| `fingerprint` | string | yes | `ro.build.fingerprint`. The source comments that *"the fingerprint entry format is important for other tools."* | `dumpstate.cpp:796,810`; archive line 7 |
| `type` | enum \| null | yes | Parsed from `display_id` (`…-userdebug`). **Available on every build type**, unlike `ro.build.type` from `getprop`. | `dumpstate.cpp:808` |
| `is_user_build` | boolean \| null | yes | `type == "user"`. **The single flag that gates everything in [§3.2](#32-gated-sections).** | `DumpstateUtil.cpp:212-217` |
| `tags` | string \| null | yes | `dev-keys` / `release-keys`. | `display_id` tail |
| `incremental` | string \| null | yes | `14784989`. | `display_id` |
| `version.release` / `version.sdk` | string / integer | yes | `15`, `35`. | `dumpstate.cpp:797,818` |
| `sdk_extensions` | array of string | yes | From the `SDK extensions: […]` line, produced by `SDK_EXT_INFO --header` with a 1 s timeout — may be absent. | `dumpstate.cpp:819-821` |
| `module_metadata_version` | integer \| null | yes | Only printed when non-zero. | `dumpstate.cpp:814-817` |
| `security_patch` | string \| null | yes | **`getprop`-only** ⇒ `null` on a `user` build. | `getprop` section |
| `build_date_utc` | string \| null | yes | `ro.build.date`. **`getprop`-only.** | `getprop` section |
| `bootloader` / `radio` / `network` | string \| null | yes | Header lines. `(unknown)` is an AOSP sentinel and must be normalised to `null`. | `dumpstate.cpp:798-813` |
| `boot_image_fingerprint` | string \| null | yes | `ro.bootimage.build.fingerprint`. **`getprop`-only.** | `getprop` section |

##### `product`

Every field here is **`getprop`-only** and therefore `null` on a `user` build. That is the
tri-state rule of [§2.9](#29-null-absent-and-raw) in its purest form.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `name` | string \| null | yes | `sdk_gcar_arm64`. | `getprop` |
| `device` | string \| null | yes | `emulator_car64_arm64`. **Differs from `name`** — the `Build:` header carries one, the fingerprint the other. Store both, never one from the other. | `getprop`; `dumpstate.cpp:808,810` |
| `brand` / `manufacturer` / `model` | string \| null | yes | `google` / `Google` / `Car on arm64 emulator`. | `getprop` |
| `first_api_level` | integer \| null | yes | 35. | `getprop` |
| `characteristics` | array of string | yes | `["automotive"]`. On this archive the only *in-band* statement that this is a car. | `getprop` |
| `cpu_abi` / `cpu_abilist` | string \| null | yes | `arm64-v8a`. | `getprop` |

##### `device`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `is_emulator` | boolean \| null | yes | `ro.hardware == "ranchu"` **or** `ro.kernel.qemu == 1`. Decisive for reading virtio / `kvm-arm.mode=protected` as expected rather than anomalous. **⇒ `null` on a `user` build, never `false`.** | `getprop` |
| `is_emulator_basis` | array of string | yes | The properties that decided it. Empty when `null`. | `getprop` |
| `avd_name` | string \| null | yes | `ro.boot.qemu.avd_name`. | `getprop` |
| `serial_no` | string \| null | yes | `ro.boot.serialno`. | `getprop` |
| `boot_hardware` | string \| null | yes | `androidboot.hardware` from `/proc/bootconfig`. **Available on all builds.** | `dumpstate.cpp:826-827` |
| `bootconfig_excerpt` | object | yes | The small set of keys that change device class: `boot_devices`, `hardware.vulkan`, `qemu`, `qemu.avd_name`, `qemu.skin`, `qemu.vsync`, `veritymode`, `vbmeta.digest`, `vbmeta.hash_alg`, `vbmeta.size`, `logcat`. | `dumpstate.cpp:826-827` |
| `cmdline_has_kvm_arm_protected` | boolean \| null | yes | Derived from `/proc/cmdline`. | `dumpstate.cpp:825` |

##### `kernel`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `version_line` | string | yes | Verbatim `/proc/version`. | `dumpstate.cpp:823-824` |
| `version_release` | string \| null | yes | First token. | `dumpstate.cpp:823` |
| `cmdline` | string | yes | `/proc/cmdline`, first line only (`strtok(cmdline_buf, "\n")`). | `dumpstate.cpp:825` |
| `config_archived` | boolean | yes | `FS/proc/config.gz` present. Lets a consumer answer *"is KASAN on?"* without guessing. | `dumpstate.cpp:1950-1952` |

##### `users`

Sourced from `dumpsys user` — **not** from `Current user ids: [10]`, which is a SystemUI
app-op monitor string at archive line 1957. The authoritative multi-user facts are
`DUMP OF SERVICE user:` → `Current user: 10` (line 613374) and `Users:` →
`UserInfo{0:null:813}` / `UserInfo{10:Driver:412}` (lines 613377 / 613416).

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `current_user_id` | integer | yes | 10. | `UserManagerService.java:7311-7317` |
| `list[]` | array of object | yes | One per `UserInfo{…}` line. | `UserManagerService.java:7322-7331` |
| `list[].id` | integer | yes | 0, 10. | `UserManagerService.java:7464` |
| `list[].name` | string \| null | yes | `null` for user 0, `Driver` for user 10. | `UserInfo.java:528-533` |
| `list[].flags` | array of string | yes | Parsed from the symbol list, not the number. | `UserManagerService.java:7483-7485` |
| `list[].flags_hex` | integer | yes | 813 (0x32b), 412 (0x19c). | `UserInfo.java:532` |
| `list[].is_primary` | boolean | yes | `true` / `false`. | `UserManagerService.java:7466` |
| `list[].parent_id` | integer \| null | yes | Printed only when it differs from `id`. **`null` for a full secondary user on this device** — the profile-of-0 relationship is *not printed*, so "which profile group does user 10 belong to" is **unanswerable from the archive**. | `UserManagerService.java:7467-7470` |
| `list[].type` | string | yes | `android.os.usertype.system.HEADLESS`, `…full.SECONDARY`. | `UserManagerService.java:7482` |
| `list[].state` | string | yes | `RUNNING_UNLOCKED`. | `UserManagerService.java:7486-7492` |
| `list[].serial_no` | integer | yes | 0, 10. | `UserManagerService.java:7465` |
| `list[].start_time_ago` | string | yes | `+49s735ms` — an **uptime-relative** "ago" string, not a wall clock. Same clock as `ProcessRecord.startUpTime`. | `UserManagerService.java:7495-7497` |
| `list[].last_logged_in_ago` / `unlock_time_ago` / `last_entered_foreground_ago` | string | yes | Same encoding. | `UserManagerService.java:7499-7510` |
| `list[].created_ago` | string \| null | yes | `+128d0h36m51s528ms` for user 10 vs `<unknown>` for user 0. `<unknown>` is a **literal in the archive**, normalised to `null` here. | `UserManagerService.java:7493-7494` |
| `cross_check.package_dump_user_ids` | array of integer | yes | Users for which the package dump carries install state. | `Settings.java:5471` |

##### `displays[]`, `hal`, `storage`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `displays[]` | `display_id` | integer | yes | `4619827259835644672`. | SurfaceFlinger CRITICAL dump, `dumpstate.cpp:1382` |
| | `hwc_display` / `port` | integer | yes | 0, 1. | same |
| | `pnp_id` | string | yes | `GGL`. | same |
| | `display_name` | string | yes | `EMU_display_0`, `EMU_display_1`. The only stable handles for the two outputs. | same |
| `hal` | `hidl_services[]` | array of object | yes | Rows of `lshal --all --types=all`: `interface::instance`, `transport`, `server_pid`, `arch`, `threads_used`/`total`, `registered`. | `dumpstate.cpp:1421-1423` |
| | `hidl_debug_entries[]` | array of string | yes | ZIP entries under `lshal-debug/`, one per interface whose `lshal debug -E` produced non-empty output within 2 s. | `dumpstate.cpp:1434-1466` |
| | `covers_aidl` | boolean | yes | **Always `false`.** `DumpHals()` enumerates the HIDL service manager only (`dumpstate.cpp:1426-1428`). A tiny `lshal-debug/` means "few HIDL HALs", **not** "few HALs". | `dumpstate.cpp:1422,1426-1428` |
| | `lshal_debug_requires_root` | boolean | yes | `true`. `AsRootIfAvailable()` is downgraded to a no-op on `user` builds. | `dumpstate.cpp:1458`; `DumpstateUtil.cpp:299-302` |
| `storage` | `mountinfo_pids` | array of integer | yes | PIDs whose `/proc/<pid>/mountinfo` was read. **A PID appearing here was alive at capture** — a cheap independent liveness signal. | `dumpstate.cpp:498-504` |
| | `archived_trees` | array of string | yes | Directory prefixes copied under `FS/`. | `dumpstate.cpp:1950-1952,1983,948,1106` |
| | `anr_files_archived` | array of object | yes | `path`, `mtime`, `bytes` per `FS/data/anr/anr_*`. **Freshness lives here** — all 65 dated 2026-05-20. `system.json` records them as *device state*; `anr.json` decides staleness. | `dumpstate.cpp:1931-1933` |
| | `tombstone_count_archived` | integer \| null | yes | 50 text + 50 `.pb` = 100 ZIP entries. Recorded as a count plus date range; `crashes.json` owns the content. | `dumpstate.cpp:1931` |
| | `dynamic_partition_info_present` | boolean | yes | `true` if `DumpDynamicPartitionInfo()` ran. | `dumpstate.cpp:1953` |

##### `archive_entries`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `proto[]` | array of object | yes | `path`, `bytes`. **`bytes: 0` is normal, not corruption** — `RunDumpsysProto` adds an entry for every service claiming proto support even if the dump wrote nothing. | `dumpstate.cpp:1340-1380` |
| `proto[].status` | enum | yes | `ok` or `declined`. `declined` ⇔ `bytes == 0`. | same |
| `proto.suffix_meaning` | object | yes | `{ "_CRITICAL": CRITICAL, "_HIGH": HIGH, "": NORMAL }`. | `dumpstate.cpp:1354-1359` |
| `fs_paths` | array of string | yes | Every `FS/**` entry. | `dumpstate.cpp:375` |
| `fs_user_build_gated` | array of string | yes | Paths present **only** on non-`user` builds: `FS/data/misc/profiles/…`, the dexopt use-list, and the inline Dropbox entries. Listed so their absence is interpretable. | `dumpstate.cpp:1943-1946,1956-1960` |
| `lshal_debug` | array of string | yes | ZIP entry names. | `dumpstate.cpp:1464` |
| `visible_windows` | object | yes | `present`, `entries[] = {id, title, bytes, mtime}`, `command`, `timeout_seconds`, `producer`. | `dumpstate.cpp:1111-1129` |
| `dumpstate_board` | object | yes | `present`, `bytes`, `has_board_binary`. Content is device-defined and unparseable in general. | `dumpstate.cpp:379-383` |

##### `scope.sections[]`

One entry per recognised section banner.

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `title` | string | yes | Text between `------ ` and ` (`. | `DumpstateUtil.cpp:306` |
| `ordinal` | integer | yes | 0-based index among sections sharing the title. | — |
| `command` | string | yes | The literal argv dumpstate ran, e.g. `/system/xbin/su root lshal --all --types=all`. **Keeping the `su root` prefix is what lets a build-type consumer work.** | `DumpstateUtil.cpp:290-297,306` |
| `status` | enum | yes | See [§2.2](#22-section-status-enum). | — |
| `start_line` / `end_line` | integer | yes | Line span in the main entry. Enables re-reading without rescanning. | — |
| `duration_seconds` | number \| null | yes | From the paired duration trailer. A long duration with no content means a timeout. | `dumpstate.cpp:3831-3835` |
| `note` | string \| null | yes | Why the status is what it is. Required when `status` is not `ok`. | — |

#### Provenance and freshness

`system.json` has no *findings*, so per-finding provenance reduces to **per-fact**, and the
rule is that every leaf carries an `evidence` block naming the exact archive line(s) and the
section it came from, plus one of four `observed_at` classifications:

| `observed_at` | Meaning | Examples |
| --- | --- | --- |
| `capture` | A point-in-time snapshot taken by this bugreport. | `Product LRU list`, `Users`, `displays`, `PID mappings` |
| `boot` | Fixed at image build time; **not** a runtime observation. | `build.*`, `product.*`, `kernel.version_line`, package `codePath` |
| `interval` | Summarises the log window. | `capture.log_window` |
| `archive-entry` | A copied file, whose truth is its **mtime**, not the capture time. | `FS/data/anr/*`, `FS/proc/*/mountinfo`, `proto/*`, `visible_windows.zip` |

`stale_relative_to_capture_seconds` is computed **only** for `archive-entry` facts, and is
`null` everywhere else. On this archive the staleness is extreme and structurally visible:
ANR files at 2026-05-20 vs capture 2026-09-24 → **126 days**. That number, once, in
`system.json`, is what stops every other artifact from re-deriving it.

#### Worked example

Abbreviated from the reference archive; every value was measured.

```jsonc
{
  "schema_version": 1,
  "artifact": "system",

  "source": {
    "archive_name": "bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip",
    "archive_size_bytes": 64645191,
    "archive_sha256": "<64 hex chars>",
    "format_version": "2.0",
    "main_entry": "bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.txt",
    "main_entry_size_bytes": 64645191,
    "main_entry_line_count": 682951,
    "zip_entry_count": null,          // not measured; walk the central directory to fill
    "device": {
      "product_name": "sdk_gcar_arm64",
      "build_id": "AAG6.260127.001",
      "build_display_id": "sdk_gcar_arm64-userdebug 15 AAG6.260127.001 14784989 dev-keys",
      "build_fingerprint": "google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys",
      "android_sdk_version": 35,
      "build_type": "userdebug",
      "device": "emulator_car64_arm64",
      "hardware": "ranchu",
      "bootloader": "unknown",        // AOSP prints "unknown" literally for ro.bootloader
      "radio": null                   // AOSP prints "(unknown)" — normalised to null per §2.9
    }
  },

  "capture": {
    "captured_at": "2026-09-24T00:27:17+07:00",
    "captured_at_time_base": "dumpstate_header",
    "uptime_at_capture_seconds": 30,
    "uptime": {
      "header_raw": "up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: 10.42, 2.38, 0.78",
      "section_raw": "00:27:25 up 0 min,  0 users,  load average: 9.66, 2.36, 0.79",
      "raw_precision": "minutes",
      "precision": "seconds",
      "precision_note": "header_raw and section_raw are minute-granular wall clock samples and are retained verbatim as evidence only; they cannot corroborate a seconds-level figure. The published value is derived, at +/- 0.6 s.",
      "seconds_at_start": 30.2,
      "uncertainty_seconds": 0.6,
      "derivation": "Boot instant B solved from three independent (wall clock, uptime) pairs, each printed inside a section that also records its own end time. For each pair B = footer_end_wall - section_duration - uptime_ms/1000. (1) line 319851 'Uptime: 45590 Realtime: 45590' in DUMPSYS HIGH meminfo, footer line 326111 '3.525s ... ending at: 2026-09-24 00:27:36' -> B = 00:26:46.885. (2) lines 488275-488276 'Now: 57365 (2026-09-24 00:27:44)' / 'Uptime: 57s' in dumpsys content, footer line 489329 '0.025s ... ending at: 00:27:44' -> B = 00:26:46.610. (3) line 499493 'Uptime: 57918 Realtime: 57918' in DUMPSYS gfxinfo, footer line 501966 '0.849s ... ending at: 00:27:46' -> B = 00:26:47.233. Uptime == Realtime in all three, so there was zero suspend time and uptime is identically elapsedRealtime. The three bracket B to [00:26:46.610, 00:26:47.233]. The kernel message at line 9126 '09-24 00:26:47.418 ... Booting Linux on physical CPU 0x0000000000' fixes the upper bound at 00:26:47.418, giving B ~= 00:26:47.33 +/- 0.10 s. Uptime at capture = banner (line 2, '== dumpstate: 2026-09-24 00:27:17') minus B = 00:27:17.000 - 00:26:47.33 ~= 29.67 s; allowing for the banner's 1-second truncation and the B bracket gives [29.58, 30.77) s, published as 30 s +/- 0.6 s.",
      "superseded_readings": {
        "27_s": "misread the wall clock '00:27:25' on the UPTIME section line (archive lines 57012-57014) as an elapsed time. That clock is when the 'uptime' command was sampled; the elapsed field on the same line is only 'up 0 min'. The true elapsed value at that line is 00:27:25 - B ~= 38 s, which is still consistent with 'up 0 min'.",
        "17_118_s": "decomposes exactly as 00:27:17.001 - 00:26:59.883, where 00:26:59.883 is the logcat timestamp of 'Start proc 944:com.android.car/1000' (line 21915). It is the AGE OF THAT PROCESS at the banner instant - a real and useful quantity, but not device uptime."
      },
      "process_age_cross_check": {
        "note": "The AMS anchor validates as a process-age cross-check. ProcessRecord at lines 390276-390289 gives startUpTime=-42s16ms for pid 944; its footer (line 395693) reads '0.218s ... dumpsys activity, ending at: 00:27:42'. So the AMS snapshot instant = 00:26:59.883 + 42.016 = 00:27:41.899, which is 0.101 s before the recorded end - internally exact. Do NOT read '-42s16ms' as an absolute uptime; that would imply 00:27:29.34, 12.7 s off.",
        "derived_snapshot_instant": "2026-09-24T00:27:41.899+07:00",
        "gap_to_footer_seconds": 0.101
      },
      "evidence": {
        "banner": "line 2",
        "kernel_boot": "line 9126",
        "meminfo_uptime": "line 319851",
        "meminfo_footer": "line 326111",
        "content_uptime": "lines 488275-488276",
        "content_footer": "line 489329",
        "gfxinfo_uptime": "line 499493",
        "gfxinfo_footer": "line 501966",
        "start_proc": "line 21915",
        "ams_start_uptime": "lines 390276-390289",
        "ams_footer": "line 395693",
        "header_raw": "line 43",
        "section_raw": "line 57013"
      }
    },
    "log_window": {
      "sections": ["SYSTEM LOG", "EVENT LOG", "STATS LOG", "RADIO LOG", "LAST LOGCAT", "SYSTEM LOG"],
      "first_line_at": "09-24 00:26:47.418",
      "last_line_at": "09-24 00:27:52.080",
      "duration_seconds": 64.662,
      "is_contiguous": false,
      "overlaps_boot_instant": false,
      "overlap_seconds": null,
      "overlap_note": "The first parseable logcat line is 00:26:47.418, which falls INSIDE the boot bracket [00:26:46.61, 00:26:47.42] rather than before it - it is the kernel's own 'Booting Linux on physical CPU 0' line (archive line 9126). The log window therefore begins essentially at boot.",
      "truncated": null
    },
    "boot": {
      "at": "2026-09-24T00:26:47.33+07:00",
      "uncertainty_seconds": 0.10,
      "derivation": "see capture.uptime.derivation",
      "epoch_corroboration": {
        "ro_runtime_firstboot": 1790184426434,
        "implied_at_local": "2026-09-24T00:27:06.434+07:00",
        "agrees_with_derived": false,
        "delta_seconds": 19.104,
        "note": "ro.runtime.firstboot is RTC-derived and on this emulator it matches the APK-install stamp of com.example.cargearviewer (line 547084 'timeStamp=2026-09-24 00:27:06'). It sits ~19.1 s AFTER the solved boot instant, so the wall clock only became trustworthy well after uptime 0. Do not prefer it."
      }
    },
    "is_cold_boot": true
  },

  "build": {
    "display_id": "sdk_gcar_arm64-userdebug 15 AAG6.260127.001 14784989 dev-keys",
    "fingerprint": "google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys",
    "type": "userdebug",
    "is_user_build": false,
    "tags": "dev-keys",
    "incremental": "14784989",
    "version": { "release": "15", "sdk": 35 },
    "sdk_extensions": ["ad_services=15", "r=15", "s=15", "t=15", "u=15", "v=15"],
    "module_metadata_version": 35,
    "security_patch": "2025-09-05",
    "build_date_utc": "Tue Jan 27 13:18:08 UTC 2026",
    "bootloader": "unknown",
    "radio": null,
    "network": null
  },

  "product": {
    "name": "sdk_gcar_arm64", "device": "emulator_car64_arm64",
    "brand": "google", "manufacturer": "Google", "model": "Car on arm64 emulator",
    "first_api_level": 35,
    "characteristics": ["automotive"],
    "cpu_abi": "arm64-v8a", "cpu_abilist": "arm64-v8a"
  },

  "device": {
    "is_emulator": true,
    "is_emulator_basis": ["ro.hardware=ranchu", "ro.kernel.qemu=1"],
    "avd_name": "Automotive_Ultrawide",
    "serial_no": "EMULATOR36X5X11X0",
    "boot_hardware": "ranchu",
    "bootconfig_excerpt": {
      "boot_devices": "a003600.virtio_mmio", "hardware.vulkan": "ranchu",
      "qemu": "1", "qemu.skin": "automotive_ultrawide_cutout", "qemu.vsync": "60",
      "veritymode": "enforcing", "vbmeta.hash_alg": "sha256", "logcat": "*:V"
    },
    "cmdline_has_kvm_arm_protected": true
  },

  "kernel": {
    "version_release": "6.6.30-android15-8-g02f25b80c91c-ab12341571-4k",
    "cmdline": "console=ttynull stack_depot_disable=on cgroup_disable=pressure kasan.stacktrace=off kvm-arm.mode=protected bootconfig ioremap_guard …",
    "config_archived": true
  },

  "users": {
    "current_user_id": 10,
    "list": [
      { "id": 0, "name": null, "flags": ["ADMIN","INITIALIZED","PRIMARY","SYSTEM"], "flags_hex": 813,
        "is_primary": true, "parent_id": null,
        "type": "android.os.usertype.system.HEADLESS", "state": "RUNNING_UNLOCKED",
        "serial_no": 0, "created_ago": null,
        "start_time_ago": "+49s735ms", "last_logged_in_ago": "+45s957ms",
        "unlock_time_ago": "+46s40ms", "last_entered_foreground_ago": null },
      { "id": 10, "name": "Driver", "flags": ["ADMIN","FULL","INITIALIZED"], "flags_hex": 412,
        "is_primary": false, "parent_id": null,
        "type": "android.os.usertype.full.SECONDARY", "state": "RUNNING_UNLOCKED",
        "serial_no": 10, "created_ago": "+128d0h36m51s528ms",
        "start_time_ago": "+49s266ms", "last_logged_in_ago": "+44s755ms",
        "unlock_time_ago": "+45s865ms", "last_entered_foreground_ago": "+48s813ms" }
    ],
    "cross_check": {
      "package_dump_users": [0, 10],
      "package_dump_user_blocks": 594,
      "shared_user_blocks": 14,
      "arithmetic": "282 packages x 2 users = 564, + 14 SharedUser x 2 = 28 => 592; the measured section contains 594 'User N:' blocks, so two blocks are attributable to neither set — see open questions",
      "agrees_with_dumpsys_user": true
    }
  },

  "hal": {
    "hidl_binderized_count": 55,
    "hidl_passthrough_count": 10,
    "hidl_debug_entries": [
      "android.hardware.audio.effect@6.0__IEffectsFactory_default.txt",
      "android.hardware.media.c2@1.0__IComponentStore_default.txt",
      "android.hardware.media.c2@1.2__IComponentStore_software.txt"
    ],
    "covers_aidl": false,
    "lshal_debug_requires_root": true
  },

  "storage": {
    "mountinfo_pids_count": 88,
    "archived_trees": ["data/", "linkerconfig/", "proc/", "product/", "system/", "system_ext/", "vendor/"],
    "anr_files_archived_count": 65,
    "anr_files_date_range": ["2026-05-20 00:54:11", "2026-05-20 05:13:42"],
    "anr_files_staleness_days_vs_capture": 126,
    "tombstone_count_archived": 50,
    "tombstone_zip_entries": 100,      // 50 text + 50 .pb
    "dynamic_partition_info_present": true
  },

  "archive_entries": {
    "proto_declined_count": 3,
    "fs_user_build_gated": ["FS/data/misc/profiles/**", "<PACKAGE_DEX_USE_LIST>", "<DROPBOX_DIR> entries (wtf/strictmode filtered)"],
    "visible_windows": {
      "present": true,
      "command": "cmd window dump-visible-window-views",
      "timeout_seconds": 10,
      "entries": [
        { "id": "4422b2b", "title": "com.example.cargearviewer/com.example.cargearviewer.MainActivity", "bytes": 12777, "mtime": "2026-09-24 00:27" },
        { "id": "42b3c53", "title": "TopCarSystemBar", "bytes": 51634 }
      ],
      "triage_value": "The foreground app at capture is com.example.cargearviewer, 28.5 s after the osdouble crash loop began — the loop was running behind an unrelated app."
    },
    "dumpstate_board": { "present": true, "bytes": 153, "has_board_binary": false,
      "content_unparsed": "vendor stub: 'verbose logging: disabled' / '[default] Hello, world!'" }
  },

  "scope": {
    "sections_scanned": 1100,
    "sections_matched": 1100,
    "sections_listed": ["UPTIME", "DUMPSYS CRITICAL", "…", "SYSTEM LOG", "…", "SYSTEM LOG"],
    "sections_skipped": [],
    "sections_not_found": [],
    "max_lines_read": 682951,
    "truncated": false,
    "sections": [
      { "title": "UPTIME", "ordinal": 0, "command": "uptime", "start_line": 57012, "end_line": 57014,
        "duration_seconds": 0.006, "status": "ok" },
      { "title": "HARDWARE HALS", "ordinal": 0, "command": "/system/xbin/su root lshal --all --types=all",
        "status": "ok", "note": "the 'su root' prefix is retained so a build-type consumer can tell this section was root-gated" },
      { "title": "SYSTEM PROPERTIES", "ordinal": 0, "command": "getprop", "status": "ok",
        "user_build_gate": "not_applicable" },
      { "title": "ANR FILES", "ordinal": 0, "command": "ls -lt /data/anr/", "start_line": 318023,
        "end_line": 318090, "duration_seconds": 0.006, "status": "ok" },
      { "title": "VM TRACES AT LAST ANR", "ordinal": 0,
        "command": "/data/anr/anr_2026-05-20-05-13-42-314", "start_line": 309986, "end_line": 318022,
        "status": "ok", "note": "header carries mtime 2026-05-20 05:13:43; freshness is stale and is decided in anr.json" }
    ]
  },

  "evidence_gaps": [
    { "field": "capture.uptime_at_capture_seconds", "reason": "derived, not read: 30 s +/- 0.6 s from the solved boot instant B ~= 00:26:47.33 +/- 0.10 s (three (wall clock, uptime) pairs bracketed by the kernel 'Booting Linux' line at archive line 9126). See uptime.derivation. The residual limitation is that the +/- 0.6 s is dominated by the 1-second truncation of the banner and 'uptime' wall clocks, and that header_raw and section_raw are themselves minute-granular, so they cannot independently corroborate a seconds-level figure." },
    { "field": "capture.log_window.truncated", "reason": "dumpstate records no logcat truncation flag" },
    { "field": "users.list[].parent_id", "reason": "UserManagerService.java:7467-7470 prints parentId only when it differs from id" },
    { "field": "hal.aidl_services", "reason": "dumpstate.cpp:1426-1428 enumerates the HIDL hwservicemanager only" },
    { "field": "source.zip_entry_count", "reason": "not measured on the host during this pass; must be read from the central directory per REQ-GEN-01" },
    { "field": "users.cross_check.package_dump_user_blocks", "reason": "measured 594, but 282*2 + 14*2 = 592; two blocks are unaccounted for" }
  ]
}
```

#### Open questions

- **Uptime at capture — resolved, with a residual error bar.** The published figure is
  **30 s ± 0.6 s**, derived from the boot instant `B ≈ 00:26:47.33` ± 0.10 s. The two earlier
  figures were both wrong: **27 s** misread the wall clock `00:27:25` on the `UPTIME` section
  line as an elapsed time, and **17.118 s** is the age of the `com.android.car` process
  (`Start proc 944`, line 21915) rather than device uptime. The derivation is in the worked
  example above.
  **What remains genuinely open:** no archive-internal source states seconds-level uptime
  *directly* — the published figure rests entirely on the derived boot bracket, and its ± 0.6 s
  uncertainty is dominated by the 1-second truncation of the `== dumpstate:` banner clock and of
  the `uptime` wall clocks in `header_raw` / `section_raw`. A future run that can read
  `/proc/uptime` or an equivalent seconds-precision source should replace the derivation with a
  direct read.
- **Timezone.** The `== dumpstate:` header carries no offset and no epoch
  (`dumpstate.cpp:804`). Every absolute time is therefore "device-local, offset unknown".
  Recoverable only if some section prints both an absolute time and an uptime delta.
  `ro.runtime.firstboot` disagrees with the solved boot instant by 19.1 s, so it was not used.
  `timezone_offset_minutes` is `null` rather than a guessed `+07:00`.
- **`MOUNT INFO` section.** `add_mountinfo()` (`dumpstate.cpp:498`) creates a
  *duration-reporter* titled `MOUNT INFO`, **not** a `------ ` banner, so a `sections[]` index
  keyed on banners does not capture it. On this archive it does not appear. A parser must not
  conclude "no mount information" from "no banner".
- **The `SharedUser [` printer.** The *shared-user permission* printer is at `Settings.java:5803`;
  the line printing the `  SharedUser [<name>] (<hex>):` header itself was not isolated. The
  ids in the worked example are read from the archive, not from a cited printer.
- **`display_id` / `hwc_display` / `port` / `pnpId` / `displayName`.** Read from the
  `DUMPSYS CRITICAL SurfaceFlinger` block; the `dumpsys SurfaceFlinger` producer is citable
  (`dumpstate.cpp:1382`) but the per-field printer line in `SurfaceFlinger.cpp` is not.
- **`lshal` column semantics.** The header reads
  `Interface / Transport / Server / PTR / Arch / Thread Use / R / Hash`; the meanings of `R`
  and `Hash` were not verified in `lshal`'s source. If the mapping of `R` to `registered` is
  wrong, that field should be dropped rather than kept with a wrong name.
- **`dumpstate_board.txt` content.** Produced by the device-side `IDumpstateDevice` HAL and
  unparseable in general. Presence and size only; per-OEM parsing is out of scope.
- **`package_dump_user_blocks` arithmetic.** Measured 594 against an expected 592. Two of the
  `User N:` blocks in the `DUMP OF SERVICE package:` section are not accounted for by
  `282 × 2 + 14 × 2`. Either the arithmetic assumption or the block classification is off by
  two; unresolved, and recorded in `evidence_gaps` rather than papered over.
- **ZIP comment.** `ZipWriter` has no comment API and `EocdRecord.comment_length`
  (`zip_archive_common.h:60-61`) is never assigned. Read it opportunistically if present; never
  require it.

---

### 4.3 `apps.json`

#### Purpose

`apps.json` answers two questions and refuses to blur them: **what is installed, and who is it
for?** (inventory) and **which process, on whose behalf, at which moment?** (attribution).

Its central design problem is that a bugreport captures a *moving* system in one snapshot:
the reference archive holds 83 live process records whose PIDs are churned roughly every 3
seconds inside one crash loop. Any schema that keys a process on `pid` produces hundreds of
distinct "processes" that are all the same app failing.

**The link between package and process is a first-class object, not an implicit join.**
`ProcessRecord.processName` is not a package name — a webview child process lists the host
package *plus* its own via `packageList={…}` (`ProcessRecord.java:508`), which is the
authoritative many-to-many mapping. The join is stored, not recomputed.

#### Top-level keys

The thirteen envelope keys, with `capture` replaced by `capture_ref` per rule 6 of
[§2.9](#29-null-absent-and-raw), plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | `{ source, boot_epoch, uptime_seconds_at_start, log_window_ref }`. **Must not re-derive capture context.** |
| `channels` | array of object | yes | One entry per evidence channel actually present, with its own window and completeness. |
| `inventory` | object | yes | `{ counts, packages[], processes[], shared_uids[], index }`. |
| `lifecycle_events` | array of object | yes | `am_proc_start` / `am_proc_bound` / `am_proc_died` / `am_kill`, normalised. |
| `activity_events` | array of object | yes | `usagestats` `ACTIVITY_RESUMED` / `ACTIVITY_STOPPED` with timestamps. |
| `attribution` | object | yes | PID-reuse evidence, kill-attribution split, unresolved list. |

#### Field tables

##### `channels[]`

This is the field that makes a count interpretable: a channel whose window is shorter than the
log window cannot bound anything over the full window.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | enum | yes | `dumpsys_package`, `dumpsys_activity_processes`, `am_proc_event_log`, `usagestats`, `logcat_main`, `fs_archive`, `pm_list_packages`. |
| `present` | boolean | yes | Whether the channel exists in this archive. |
| `first_at` / `last_at` | string \| null | yes | The channel's own window. `null` for a point-in-time snapshot. |
| `duration_seconds` | number \| null | yes | Window length. |
| `covers_log_window` | boolean \| null | yes | Whether the channel spans the whole `capture.log_window`. `null` for snapshots. |
| `coverage_ratio` | number \| null | yes | **Required** when `covers_log_window` is false. See invariant 2 of [§2.3](#23-counts-are-match-blocks). |
| `incomplete_reason` | string \| null | yes | `event_log_buffer_retention`, `gated_by_user_build`, `permission_dump_and_usage_stats`, `declined_by_service`, `not_emitted_by_platform`. |
| `snapshot_at` | string \| null | yes | For point-in-time channels, when the dump ran. |
| `blocks[]` | array of object | no | For channels split across duplicate sections: `{ index, command, line_start, line_end }`. `index` disambiguates the two `SYSTEM LOG` blocks. |
| `carries` | array of string | yes | What the channel contains, with counts. |

##### `inventory.counts`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `packages` | integer | yes | 282 `Package [` blocks. | `Settings.java:5118` |
| `processes` | integer | yes | 83 = 74 `*APP*` + 9 `*PERS*`. | `ProcessList.java:4454-4477,4469` |
| `processes_persistent` / `processes_app` | integer | yes | 9 / 74. | `ProcessList.java:4469` |
| `distinct_process_names` | integer | yes | 58. | derived |
| `packages_with_live_process` | integer | yes | 58. The coverage ratio is `58/282`. | derived |
| `users` | integer | yes | 2. | `UserManagerService.java:7311-7331` |
| `shared_uids` | integer | yes | 14 `SharedUser [` blocks. | `Settings.java` shared-user dump |
| `package_user_state_blocks` | integer | yes | 594, with the arithmetic stored next to it. | `Settings.java:5471-5510` |
| `lru` | object | yes | `{ total: 83, non_activity: 4, non_service: 4 }`, straight from the header. | `ProcessList.java:4323-4331` |
| `car_service_process_names` | integer | yes | **13 — the count of `com.android.car.*` *processes*.** Explicitly not packages. | `ProcessRecord.java:1396` |
| `car_service_packages` | integer | yes | **100+ — the count of `com.android.car.*` *packages*.** Storing both is the point. | `Settings.java:5118` |
| `lifecycle_counts` | object | yes | `{ am_proc_start: 121, am_proc_bound: 121, am_proc_died: 40, am_kill: 0, am_process_crashed_too_much: 38 }`. | `EventLogTags.logtags:20,22,25,34,45` |
| `lifecycle_channel_window` | object | yes | The event-log buffer's own window and coverage. See below. | `dumpstate.cpp:1028` |
| `sources_available` / `sources_absent` | array of string | yes | Which evidence sources exist and which the platform never emits. | — |

`lifecycle_channel_window` is load-bearing:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `first_event_at` / `last_event_at` | string | yes | `09-24 00:26:59.872` → `09-24 00:27:20.176`. |
| `duration_seconds` | number | yes | 20.304. |
| `covers_full_log_window` | boolean | yes | `false`. |
| `coverage_ratio` | number | yes | **0.314.** |
| `all_in_first_system_log_block` | boolean | yes | `true` — every `am_proc_*` line falls in archive lines 53824–55656, inside the **first** `SYSTEM LOG` block (9116–53708). The second block (653373–682950) contains zero. |
| `consequence` | string | yes | The plain-language statement of what the coverage gap does to the counts. |

##### `inventory.packages[]` — `PackageEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `package_name` | string | yes | **Primary key.** | `Settings.java:5118-5122` |
| `identity_hash` | string | yes | `(2c690bd)` — `System.identityHashCode`. **Not an identity**; per-boot and allocator-dependent. Stored so a human can cross-reference a section. | `Settings.java:5120-5122` |
| `app_id` | integer | yes | `10250`. **Join accelerator only** — reassigned on reinstall, and shared (`android.uid.system` apps all have `app_id = 1000`). | `Settings.java:5129` |
| `shared_uid_name` | string \| null | yes | `android.uid.system`. **This is what makes `app_id=1000` interpretable.** | `Settings.java:5131-5133` |
| `version_name` | string \| null | yes | | `Settings.java:5178` |
| `version_code` / `min_sdk` / `target_sdk` | integer | yes | | `Settings.java:5154-5160` |
| `code_path` / `resource_path` | string | yes | | `Settings.java:5136,5141` |
| `code_path_prefix` | enum | yes | Derived: `/data/app`, `/system`, `/system_ext`, `/product`, `/vendor`, `/apex`, `other`. Feeds `tier`. | derived from `Settings.java:5136` |
| `tier` | enum | yes | `shared_system` / `system` / `third_party` / `unknown`. **A label, never a filter.** | derived |
| `tier_basis` | array of string | yes | The raw evidence `tier` was computed from, so it is recomputable. | derived |
| `flags` / `private_flags` | array of string | yes | From `flags=[ … ]` and `privateFlags=[ … ]`. | `Settings.java:5185-5190` |
| `apk_signing_version` | integer | yes | 2 for sideloaded, 3 for system. A cheap "was this sideloaded?" signal. | `Settings.java:5182-5184` |
| `apex_module_name` | string \| null | yes | | `Settings.java:5415` |
| `package_source` / `package_source_name` | integer / string | yes | 0 = `PACKAGE_SOURCE_UNSPECIFIED`, 1 = `OTHER`, 2 = `STORE`, 3 = `LOCAL_FILE`, 4 = `DOWNLOADED_FILE`. | `Settings.java:5387`; `PackageInstaller.java:601-626` |
| `installer_package_name` / `initiating_package_name` | string \| null | yes | | `Settings.java:5370,5376` |
| `last_update_time` | string \| null | yes | **The package's own freshness anchor.** | `Settings.java:5364-5369` |
| `time_stamp` | string \| null | yes | Package-manager file mtime. | `Settings.java:5364-5366` |
| `users[]` | array of object | yes | Per-user install state. Below. | `Settings.java:5471-5510` |
| `declared_permissions[]` | array of string | yes | | `Settings.java` declared-permissions block |
| `install_permissions[]` | array of string | yes | The granted set. **This is the "permissions" the requirements ask for** — per-package and per-user, so it lives here. | `Settings.java:5461-5466` |
| `provenance` | object | yes | See [§2.4](#24-provenance). | — |

`users[]`:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `user_id` | integer | yes | 0, 10. | `Settings.java:5471` |
| `installed` | boolean | yes | `false` in user 0, `true` in user 10, for `com.example.cargearviewer`. | `Settings.java:5476-5478` |
| `data_dir` | string \| null | yes | `null` when not installed. | `Settings.java:5503-5505` |
| `first_install_time` | string \| null | yes | `1970-01-01 08:00:00` for the **not-installed** user is the epoch — a **sentinel, not a real install**. Must be normalised to `null` with `first_install_time_sentinel: "epoch_means_not_installed"`. | `Settings.java:5507-5509` |
| `enabled` | integer | yes | 0 = `COMPONENT_ENABLED_STATE_DEFAULT`. | `Settings.java:5488-5489` |
| `stopped` / `not_launched` / `hidden` / `suspended` / `quarantined` | boolean | yes | | `Settings.java:5479-5493` |
| `ce_data_inode` / `de_data_inode` | integer \| null | yes | | `Settings.java:5472` |
| `install_reason` / `install_reason_name` | integer / string | yes | 0 = `INSTALL_REASON_UNKNOWN`. | `Settings.java:5500-5501`; `PackageManager.java:1920` |

##### `inventory.processes[]` — `ProcessEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `process_key` | string | yes | **Primary key.** `"<boot_epoch>/<start_seq>"`. | `ProcessList.java:2155-2157` |
| `start_seq` | integer | yes | 274, 61, 2. | `ProcessRecord.java:540` |
| `record_kind` | enum | yes | `persistent` / `app`, from `*PERS*` / `*APP*`. | `ProcessList.java:4469` |
| `record_identity_hash` | string | yes | Diagnostic only. Stable across the whole crash loop for `osdouble`, which is a cheap cross-check that AOSP reuses the record. | `ProcessRecord.java:1402-1403` |
| `pid` | integer | yes | **Observation attribute, not identity.** | `ProcessRecord.java:517-518` |
| `process_name` | string | yes | The *process* name, not the package name. | `ProcessRecord.java:471-472` |
| `runtime_key` | string | yes | `"<user_id>:<process_name>"`. **Not unique** — 13 PIDs share `0:com.android.car.cluster.osdouble`. | derived |
| `attribution_key` | string | yes | `"<primary package>/<user_id>"` — the human-facing identity. | derived |
| `user_id` / `uid` | integer | yes | `1000`, `1010250`. | `ProcessRecord.java:465-466` |
| `uid_formatted` | string | yes | `u10a250` style. Decoding: `uid = user * PER_USER_RANGE(100000) + (appId + FIRST_APPLICATION_UID(10000))`; a trailing `iN` is an isolated uid offset. | `ProcessRecord.java:1378-1391` |
| `app_id` | integer | yes | | `UserHandle.java:402` |
| `gids` | array of integer | yes | | `ProcessRecord.java:469-478` |
| `isolated_uid` | integer \| null | yes | Present only when `uid != info.uid`. | `ProcessRecord.java:467-469` |
| `packages[]` | array of string | yes | From `packageList={…}`. **The authoritative process→package link.** | `ProcessRecord.java:508-509` |
| `package_dependencies[]` | array of string | yes | `packageDependencies={…}`. Rare but real. | `ProcessRecord.java:510-517` |
| `start_uptime_ago` | string | yes | `-108ms`. **Uptime-relative, not wall clock.** | `ProcessRecord.java:522-523` |
| `start_elapsed_time_ago` | string \| null | yes | Same on the elapsed-realtime clock. | `ProcessRecord.java:524-525` |
| `start_wall_clock` | string \| null | yes | **Derived**: `snapshot_instant + start_uptime_ago`, i.e. the wall clock of the dumpsys snapshot that printed `startUpTime`, *minus* the process's age. **Not** `boot.at + start_uptime_ago`: `startUpTime` is a relative age (`ProcessRecord.java:522` formats `nowUptime - mStartUptime`), so it must be anchored to the snapshot instant, not to boot. Carries `start_wall_clock_evidence`. | derived |
| `last_activity_time_ago` | string | yes | | `ProcessRecord.java:520-521` |
| `persistent` / `removed` | boolean | yes | Only printed when set. | `ProcessRecord.java:527-529` |
| `start_seq_conflict` | boolean | yes | `true` if two entries share a `start_seq`. Impossible by construction, so `true` means the two `dumpsys` snapshots disagree. Cheap integrity check. | derived |
| `code_path` / `data_dir` | string | yes | | `ProcessRecord.java:503-505` |
| `instruction_set` / `required_abi` | string \| null | yes | | `ProcessRecord.java:496-499` |
| `class_name` | string \| null | yes | | `ProcessRecord.java:500-502` |
| `compat` | string \| null | yes | `{240dpi always-compat}`. | `ProcessRecord.java:518-520` |
| `oom_adj` | object | yes | `{max, cur_raw, set_raw, cur, set}`. | oom-adj dump |
| `oom_adj_name` | string | yes | From the LRU line: `pers`, `fg`, `vis`, `psvc`, `cch`, `sys`. | `ProcessList.java:1161-1180,4341` |
| `cur_proc_state` / `set_proc_state` | integer | yes | | process-state dump |
| `capability` | string | yes | `LCMNFUA`. **7 chars, positional, `-` = absent.** Carried as the raw string; no per-character decode, because that decode was not traced to a source. | `ProcessList.java:4343` |
| `lru` | object | yes | `{index, oom_adj_name, capability, flags, fixed, service, top_activity, client, cch_empty}`. | `ProcessList.java:4333-4400` |
| `services[]` / `providers[]` / `receivers[]` / `connections[]` | array | yes | Component wiring. **`crashes.json` reads these; it does not re-parse them.** | `ProcessRecord.java:559-563` |
| `lifecycle_refs` | array of object | yes | `{event_kind, at, event_seq_index}` linking to `lifecycle_events[]`. | derived |
| `provenance` | object | yes | | — |

##### `lifecycle_events[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `kind` | enum | yes | `am_proc_start` / `am_proc_bound` / `am_proc_died` / `am_kill`. | `EventLogTags.logtags:20,22,25,34` |
| `at` | string | yes | `MM-DD HH:MM:SS.mmm` from the logcat stamp. | `logprint.cpp` threadtime |
| `uid_column_raw` | string | yes | **The raw third column, verbatim.** Never coerced at parse time — see the note below. | `logprint.cpp:1492-1513` |
| `pid` / `tid` | integer | yes | | same |
| `user_id` | integer | yes | `am_proc_start`'s first arg is `UserHandle.getUserId(app.getStartUid())` — the **start** uid, not `info.uid`. | `ProcessList.java:2858-2861` |
| `start_uid` | integer | yes | `am_proc_start` only. | `ProcessList.java:2860` |
| `process_name` | string | yes | | `ProcessList.java:2861` |
| `hosting_type` / `hosting_component` | string \| null | yes | `service`, `activity`, `""`. | `ProcessList.java:2862-2863` |
| `oom_adj` | integer \| null | yes | `am_proc_died`, `am_kill`. | `logtags:22,34` |
| `reason` | string \| null | yes | `am_kill` only. | `logtags:34` |
| `rss_kb` | integer \| null | yes | `am_kill` only (`Rss\|2\|2`). | `logtags:34` |
| `process_key` | string \| null | yes | `null` with `unresolved_reason` when no `ProcessRecord` interval contains `at`. | derived |
| `provenance` | object | yes | Carries `block_index` to disambiguate the two `SYSTEM LOG` blocks, and `channel_window` so a consumer can see the event sits inside a short-retention buffer. | — |

> **The logcat uid column is a `getpwuid` name when it is ≤ 5 characters, else a decimal uid,
> else six spaces when `entry->uid < 0`** (`logprint.cpp:1492-1513`). It is *never* guaranteed to
> be an integer. Measured on the reference archive: 41,230 numeric and 6,038 symbolic
> (`root` 4060, `media` 822, `radio` 635, `shell` 200, `logd` 144, `wifi` 61, `lmkd` 18,
> `artd` 3, `gps` 2). **This is the single most likely silent parser break in this artifact.**
> Any uid-joining code must try the name table first.

##### `activity_events[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `at` | string | yes | **Second granularity, unlike logcat's milliseconds.** | `UserUsageStatsService.java:1087` |
| `type` | string | yes | `ACTIVITY_RESUMED`, `ACTIVITY_STOPPED`, `USER_UNLOCKED`, `SCREEN_INTERACTIVE`, … | `UserUsageStatsService.java:1088-1124` |
| `package_name` / `class_name` | string | yes | **This is what ties a crash loop to a specific activity.** | same |
| `range_header_form` | enum | yes | `timeRange_locale` or `beginTime_endTime_epoch_ms` — two mutually exclusive renderings. | `UserUsageStatsService.java:1157-1163` |
| `range_raw` | string | yes | Verbatim. The locale form contains U+202F narrow no-break spaces and an en dash — **do not regex-split on a space.** | same |
| `user_id` | integer \| null | yes | See open questions. | — |
| `stale_relative_to_capture_seconds` | integer \| null | yes | Meaningful only because the range header is parsed into a real interval. | derived |

##### `attribution`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kill_sources` | object | yes | `{ am_kill_events: 0, ams_crash_loop_kills: 273, app_errors_am_process_crashed_too_much: 38 }`. **Three disjoint buckets, never summed into one `kills` number.** |
| `kill_source_completeness` | object | yes | Per-bucket completeness. `am_kill` is complete for the window; `am_proc_died` is **partial** (0.314 coverage). |
| `pid_reuse` | object | yes | `{worst_process, pid_first, pid_last, pid_delta, window_first, window_last, window_seconds, estimated_full_pid_cycles, start_proc_events, conclusion}`. |
| `unresolved[]` | array of object | yes | Events that could not be joined, each with a `reason` and a `count`. |

`am_kill: 0` is a **load-bearing fact**, not an absence: it proves there was no
memory-pressure kill in the window, so the churn is AMS-initiated only. It must be emitted as a
distinct counter from `ams_crash_loop_kills`, never merged.

##### `inventory.index`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `package_to_process_keys` | object | yes | Package → process keys. |
| `process_name_to_packages` | object | yes | Process name → packages, for names that are not package names (`system`, `.adservices`, `android.process.media`, …). |
| `uid_to_process_keys` | object | yes | UID → process keys. |
| `unmapped_process_names` | object | yes | `{count, examples}`. |

#### Identity model

Three derived keys, each for a different purpose:

| Key | Formula | Use |
| --- | --- | --- |
| `process_key` | `"<boot_epoch>/<start_seq>"` | Exact instance identity. The join key to `am_proc_*` events once a wall clock is mapped. |
| `attribution_key` | `"<package_name>/<user_id>"` | "Which app, on which profile." The human-facing identity. |
| `runtime_key` | `"<user_id>:<process_name>"` | The pre-identity grouping. **Not unique.** |

`start_seq` is adopted rather than invented because **AOSP itself uses it as the instance
identity**: `ActivityManagerService.java:930` compares `existingApp.getStartSeq() ==
app.getStartSeq()`; `:4475` rejects an attach whose `startSeq` differs; `:4814` matches a
provider process on `getStartUid() == uid && getStartSeq() == startSeq`. Its only weakness is
that it resets when AMS restarts — which is exactly what `boot_epoch` supplies, so the two
compose.

**Correlating an event to an app is a two-step join.** The event-log channel has no
`startSeq`, so:

1. **Event → process instance.** Take the event's wall clock `t` and its
   `(user_id, process_name)`. Find the process record with that `runtime_key` whose uptime
   interval `[boot_at + startUpTime, boot_at + lastActivityTime]` contains `t`. This is
   unique, because consecutive instances of the same process name are separated in uptime.
2. **Process instance → app.** `process.packages[]` from `packageList={…}`.

Anything that cannot complete step 1 goes in `attribution.unresolved[]` with a reason
(`no_process_record`, `outside_snapshot_uptime_range`, `isolated_process`) rather than being
silently dropped or guessed.

#### Provenance and freshness

Five distinct clocks, and `provenance.capture_kind` plus `captured_at_time_base` name which
one applies:

| Object | Clock | Example value | `capture_kind` |
| --- | --- | --- | --- |
| `packages[]` | wall clock, `lastUpdateTime` | `2026-05-18 23:50:51` | `disk_artifact` |
| `packages[].users[]` | wall clock, `firstInstallTime` | `2026-05-18 23:51:03` | `disk_artifact` |
| `processes[]` | **uptime**, `startUpTime` | `-108ms` | `point_in_time_snapshot` |
| `lifecycle_events[]` | logcat wall clock, **event-log buffer** | `00:27:01.587` | `live_log` |
| `activity_events[]` | usage-stats wall clock, **second granularity** | `00:27:03` | `point_in_time_snapshot` |

A package entry is as fresh as its last update — **126 days** before capture for every system
package on this archive, and **11 s** for the one package sideloaded during this boot. A
process record is snapshot-scoped: fresh only at
`capture.uptime.at_snapshot_dumpsys_activity_seconds`. A lifecycle event is inside the *event
log's* window (0.314 of the log window), **not** necessarily inside the log window.

Channel coverage on this archive:

| Channel | Window | Coverage | What it can bound |
| --- | --- | --- | --- |
| `logcat_main` | 00:26:47.418 → 00:27:52.080 | 1.00 | everything (277 `Start proc`, 273 `crashed too many times`) |
| `am_proc_event_log` | 00:26:59.872 → 00:27:20.176 | **0.314** | process churn **only in that 20.3 s** (121 starts, 40 deaths) |
| `dumpsys_activity_processes` | snapshot at ≈00:27:41.9 | n/a | liveness at that instant, nothing before |
| `dumpsys_package` | snapshot at 00:27:48 (0.831 s) | n/a | install state at that instant |
| `usagestats` | 24 h window, second granularity | >1 | activity history, pre-boot included |
| `fs_archive` | file mtimes | n/a | anything — this is where the 126-day-stale ANR files live |

Reporting `am_proc_died: 40` without `coverage_ratio: 0.314` is the class of error invariant 2
of [§2.3](#23-counts-are-match-blocks) exists to eliminate.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "apps",

  "capture_ref": {
    "source": "system.json#/capture",
    "boot_epoch": "2026-09-24T00:26:47.33+07:00",
    "boot_epoch_uncertainty_seconds": 0.10,
    "uptime_seconds_at_start": 30.2,
    "at_snapshot_dumpsys_activity_seconds": 42.016
  },

  "channels": [
    { "name": "logcat_main", "present": true,
      "first_at": "09-24 00:26:47.418", "last_at": "09-24 00:27:52.080",
      "duration_seconds": 64.662, "covers_log_window": true, "coverage_ratio": 1.0,
      "incomplete_reason": null,
      "blocks": [
        { "index": 0, "command": "logcat -v threadtime -v printable -v uid -d *:v", "line_start": 9116, "line_end": 53708 },
        { "index": 1, "command": "logcat -v threadtime -v printable -v uid -d *:v -T 2026-09-24 00:27:20.000", "line_start": 653373, "line_end": 682950 }
      ],
      "carries": ["Start proc (277)", "has crashed too many times, killing! (273)", "am_proc_* events"] },
    { "name": "am_proc_event_log", "present": true,
      "first_at": "09-24 00:26:59.872", "last_at": "09-24 00:27:20.176",
      "duration_seconds": 20.304, "covers_log_window": false, "coverage_ratio": 0.314,
      "incomplete_reason": "event_log_buffer_retention",
      "carries": ["am_proc_start (121)", "am_proc_bound (121)", "am_proc_died (40)", "am_kill (0)"] },
    { "name": "dumpsys_activity_processes", "present": true, "snapshot_at": "2026-09-24T00:27:41.898+07:00",
      "covers_log_window": null, "incomplete_reason": null,
      "carries": ["PID mappings (83)", "Process LRU list (83)", "*APP*/*PERS* records (83)"] },
    { "name": "dumpsys_package", "present": true, "snapshot_at": "2026-09-24T00:27:48+07:00",
      "duration_seconds": 0.831, "covers_log_window": null, "incomplete_reason": null,
      "carries": ["Package [ blocks (282)", "SharedUser [ blocks (14)", "per-user state blocks (594)"] },
    { "name": "usagestats", "present": true,
      "range_header_form": "timeRange_locale",
      "range_raw": "9/23/2026, 12:27 AM – 9/24/2026, 12:27 AM",
      "covers_log_window": false, "coverage_ratio": null,
      "incomplete_reason": "second_granularity_only; user coverage unverified" },
    { "name": "pm_list_packages", "present": false, "covers_log_window": null,
      "incomplete_reason": "not_emitted_by_platform — dumpstate never invokes 'pm list packages'" }
  ],

  "inventory": {
    "counts": {
      "packages": 282, "processes": 83, "processes_persistent": 9, "processes_app": 74,
      "distinct_process_names": 58, "packages_with_live_process": 58,
      "users": 2, "shared_uids": 14, "package_user_state_blocks": 594,
      "lru": { "total": 83, "non_activity": 4, "non_service": 4 },
      "car_service_process_names": 13,
      "car_service_packages": 100,
      "lifecycle_counts": { "am_proc_start": 121, "am_proc_bound": 121, "am_proc_died": 40,
                            "am_kill": 0, "am_process_crashed_too_much": 38 },
      "lifecycle_channel_window": {
        "first_event_at": "09-24 00:26:59.872", "last_event_at": "09-24 00:27:20.176",
        "duration_seconds": 20.304, "covers_full_log_window": false, "coverage_ratio": 0.314,
        "all_in_first_system_log_block": true,
        "consequence": "am_proc_died = 40 against 271 real process deaths; a 6.9x undercount caused purely by event-log buffer retention, not by the absence of deaths. lifecycle_counts MUST be reported alongside this window, never as a window total."
      },
      "sources_absent": ["pm list packages -f -U -i (dumpstate never invokes it)",
                         "dumpsys package <pkg> (no per-package section exists)"]
    },

    "packages": [
      { "package_name": "com.example.cargearviewer",
        "identity_hash": "2c690bd", "app_id": 10250, "shared_uid_name": null,
        "tier": "third_party",
        "tier_basis": ["code_path_prefix=/data/app", "package_source=1 (PACKAGE_SOURCE_OTHER)",
                       "apk_signing_version=2", "flags lacks SYSTEM"],
        "version_name": "1.0", "version_code": 1, "min_sdk": 34, "target_sdk": 34,
        "code_path": "/data/app/~~TcNBmEythAusZaikmQt8LA==/com.example.cargearviewer-6-hqvlHi41x3ARGzVmXAbQ==",
        "code_path_prefix": "/data/app",
        "flags": ["DEBUGGABLE","HAS_CODE","ALLOW_CLEAR_USER_DATA","TEST_ONLY","ALLOW_BACKUP"],
        "apk_signing_version": 2,
        "package_source": 1, "package_source_name": "PACKAGE_SOURCE_OTHER",
        "initiating_package_name": "com.android.shell",
        "last_update_time": "2026-09-24 00:27:06",
        "install_permissions": ["com.example.cargearviewer.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
                                "android.car.permission.CAR_POWERTRAIN"],
        "users": [
          { "user_id": 0, "installed": false, "data_dir": null, "enabled": 0,
            "first_install_time": null, "first_install_time_sentinel": "epoch_means_not_installed",
            "install_reason": 0, "stopped": true },
          { "user_id": 10, "installed": true,
            "data_dir": "/data/user/10/com.example.cargearviewer",
            "ce_data_inode": 278835, "de_data_inode": 107122, "enabled": 0,
            "first_install_time": "2026-05-18 23:51:03",
            "install_reason": 0, "stopped": false, "not_launched": false }
        ],
        "provenance": { "evidence_origin": "dumpsys_package", "archive_path": "<main_entry>",
          "section_title": "DUMP OF SERVICE package:", "section_ordinal": 0,
          "line_start": 547055, "line_end": 547120,
          "captured_at": "2026-09-24T00:27:06+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 11, "capture_kind": "disk_artifact",
          "freshness": "now",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "package installed 11 s before capture — this is the only package updated during this boot",
            "threshold_seconds": 3600 },
          "collector_perturbed": false,
          "collector_perturbed_basis": "read from a 0.831s dumpsys package pass" } },

      { "package_name": "com.android.car.cluster.osdouble",
        "identity_hash": "a344387", "app_id": 1000, "shared_uid_name": "android.uid.system",
        "tier": "shared_system",
        "tier_basis": ["sharedUser=SharedUserSetting{2af2641 android.uid.system/1000}",
                       "app_id=1000 < FIRST_APPLICATION_UID",
                       "flags contains SYSTEM and PERSISTENT"],
        "version_name": "15", "version_code": 35, "min_sdk": 35, "target_sdk": 35,
        "code_path": "/system/app/ClusterOsDouble", "code_path_prefix": "/system",
        "flags": ["SYSTEM","HAS_CODE","PERSISTENT","ALLOW_CLEAR_USER_DATA","ALLOW_BACKUP"],
        "apk_signing_version": 3,
        "package_source": 0, "package_source_name": "PACKAGE_SOURCE_UNSPECIFIED",
        "last_update_time": "2026-05-18 23:50:51",
        "users": [
          { "user_id": 0, "installed": true,
            "data_dir": "/data/user/0/com.android.car.cluster.osdouble",
            "ce_data_inode": 344111, "de_data_inode": 90256,
            "first_install_time": "2026-05-18 23:50:51", "enabled": 0, "stopped": false }
        ],
        "provenance": { "captured_at": "2026-05-18T23:50:51+07:00",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 11061386, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "system package last updated 128 days before capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "processes": [
      { "process_key": "2026-09-24T00:26:47.33+07:00/274", "start_seq": 274,
        "record_kind": "persistent", "record_identity_hash": "52f575b", "pid": 7760,
        "process_name": "com.android.car.cluster.osdouble",
        "runtime_key": "0:com.android.car.cluster.osdouble",
        "attribution_key": "com.android.car.cluster.osdouble/0",
        "user_id": 0, "uid": 1000, "uid_formatted": "1000", "app_id": 1000,
        "gids": [1065,3002,3003,3001,3007,1007,1000,9997,1079,1077],
        "isolated_uid": null,
        "packages": ["com.android.car.cluster.osdouble"], "package_dependencies": [],
        "start_uptime_ago": "-108ms", "start_elapsed_time_ago": "-108ms",
        "start_wall_clock": "2026-09-24T00:27:41.784+07:00",
        "start_wall_clock_evidence": "logcat: 'Start proc 7760:com.android.car.cluster.osdouble/1000 for restart …'",
        "last_activity_time_ago": "-56ms",
        "persistent": true, "removed": false, "start_seq_conflict": false,
        "code_path": "/system/app/ClusterOsDouble/ClusterOsDouble.apk",
        "data_dir": "/data/user/0/com.android.car.cluster.osdouble",
        "instruction_set": "arm64", "required_abi": "arm64-v8a",
        "class_name": "com.android.car.cluster.osdouble.ClusterOsDoubleApplication",
        "compat": "{240dpi always-compat}",
        "oom_adj": { "max": -800, "cur_raw": -800, "set_raw": -800, "cur": -800, "set": -800 },
        "cur_proc_state": 1, "set_proc_state": 1, "capability": "LCMNFUA",
        "lru": { "index": 77, "oom_adj_name": "pers", "capability": "LCMNFUA", "flags": "PERU", "fixed": true },
        "providers": ["androidx.startup.InitializationProvider"],
        "lifecycle_refs": [],
        "lifecycle_unresolved_reason": "out_of_channel: the am_proc_* event log ends at 09-24 00:27:20.176 but this instance started at 00:27:41.784; its events fall outside the event-log buffer's retention window, so they are UNOBSERVABLE there even though the main logcat buffer still has them",
        "provenance": { "evidence_origin": "dumpsys_activity_processes", "section_title": "DUMP OF SERVICE activity:",
          "section_ordinal": 0, "line_start": 389256, "line_end": 389420,
          "captured_at": "2026-09-24T00:27:41.898+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 24.898, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer 'ending at: 2026-09-24 00:27:42'",
            "basis": "process liveness is only true at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false,
          "collector_perturbed_basis": "dumpsys activity -v all; dumpstate.cpp:1627" } },

      { "process_key": "2026-09-24T00:26:47.33+07:00/2", "start_seq": 2,
        "record_kind": "persistent", "record_identity_hash": "d78a714", "pid": 944,
        "process_name": "com.android.car", "runtime_key": "0:com.android.car",
        "attribution_key": "com.android.car/0",
        "user_id": 0, "uid": 1000, "app_id": 1000,
        "packages": ["com.android.car"], "package_dependencies": ["com.android.car.updatable"],
        "start_uptime_ago": "-42s16ms",
        "start_wall_clock": "2026-09-24T00:26:59.882+07:00",
        "persistent": true,
        "code_path": "/system/priv-app/CarService/CarService.apk",
        "oom_adj": { "max": -800, "cur": -800, "set": -800 },
        "lru": { "index": 81, "oom_adj_name": "pers", "flags": "F/ /PER", "fixed": true },
        "provenance": { "captured_at": "2026-09-24T00:27:41.898+07:00",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 24.898, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer",
            "basis": "this record is the uptime anchor for system.json",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "process_key": "2026-09-24T00:26:47.33+07:00/61", "start_seq": 61,
        "record_kind": "app", "record_identity_hash": "a4eb6e5", "pid": 2818,
        "process_name": "com.example.cargearviewer",
        "runtime_key": "10:com.example.cargearviewer",
        "attribution_key": "com.example.cargearviewer/10",
        "user_id": 10, "uid": 1010250, "uid_formatted": "u10a250", "app_id": 10250,
        "gids": [50250, 20250, 1009997],
        "packages": ["com.example.cargearviewer"],
        "start_uptime_ago": "-32s331ms",
        "start_wall_clock": "2026-09-24T00:27:10.551+07:00",
        "last_activity_time_ago": "-31s669ms",
        "persistent": false,
        "oom_adj": { "max": 100, "cur": 100, "set": 100 },
        "lru": { "index": 1, "oom_adj_name": "fg", "capability": "T/A/TOP", "top_activity": true },
        "provenance": { "captured_at": "2026-09-24T00:27:41.898+07:00",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 24.898, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer", "basis": "liveness at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "index": {
      "package_to_process_keys": { "com.android.car": ["…/2"], "com.android.car.cluster.osdouble": ["…/274"] },
      "process_name_to_packages": {
        "system": ["android", "com.android.providers.settings", "com.android.server.telecom", "com.android.location.fused"],
        ".adservices": ["com.google.android.adservices.api"]
      },
      "unmapped_process_names": { "count": 12,
        "examples": ["system", "android.process.media", "android.process.acore",
                     "android.car.usb.handler", ".adservices", "WebViewLoader-arm64-v8a"] }
    }
  },

  "lifecycle_events": [
    { "kind": "am_proc_start", "at": "09-24 00:26:59.882",
      "uid_column_raw": "1000", "pid": 944, "tid": 718,
      "user_id": 0, "start_uid": 1000, "process_name": "com.android.car",
      "hosting_type": "service",
      "hosting_component": "{com.android.car/com.android.car.CarService}",
      "process_key": "2026-09-24T00:26:47.33+07:00/2",
      "provenance": { "section_title": "SYSTEM LOG", "section_ordinal": 0, "line_start": 27853,
        "captured_at": "2026-09-24T00:26:59.882+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 17.118, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "kind": "am_proc_start", "at": "09-24 00:26:59.872",
      "uid_column_raw": "1000", "pid": 937, "tid": 718, "user_id": 0, "start_uid": 1037,
      "process_name": "WebViewLoader-arm64-v8a", "hosting_type": "", "hosting_component": "",
      "process_key": null,
      "unresolved_reason": "no_process_record: process had already exited by the 00:27:42 snapshot; pid 937 is absent from PID mappings",
      "provenance": { "section_title": "SYSTEM LOG", "section_ordinal": 0, "line_start": 53824,
        "captured_at": "2026-09-24T00:26:59.872+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 17.128, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "activity_events": [
    { "at": "2026-09-24T00:27:03+07:00", "type": "ACTIVITY_RESUMED",
      "package_name": "com.android.car.cluster.osdouble",
      "class_name": "com.android.car.cluster.osdouble.ClusterOsDoubleActivity",
      "range_header_form": "timeRange_locale",
      "range_raw": null, "user_id": null,
      "note": "288 ACTIVITY_RESUMED/STOPPED pairs for this one class in the first 7 s of the window — the usagestats-side view of the same crash loop, on a clock with second granularity",
      "provenance": { "section_title": "DUMP OF SERVICE usagestats:", "section_ordinal": 0,
        "line_start": 611081, "captured_at": "2026-09-24T00:27:03+07:00",
        "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 14.0, "capture_kind": "point_in_time_snapshot",
        "freshness": "now",
        "freshness_basis": { "input": "usagestats event timestamp",
          "basis": "second granularity; inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "attribution": {
    "kill_sources": {
      "am_kill_events": 0,
      "ams_crash_loop_kills": 273,
      "app_errors_am_process_crashed_too_much": 38,
      "note": "three disjoint buckets; am_kill=0 proves there was NO memory-pressure kill in the window, so the churn is AMS-initiated only"
    },
    "kill_source_completeness": {
      "am_kill_channel": "complete_for_the_window",
      "am_proc_died_channel": "PARTIAL — covers only 00:26:59.872-00:27:20.176 (20.304 s of the 64.662 s window)",
      "ams_logcat_lines": "complete_for_the_window — 'Start proc' 277, 'killed too many times' 273"
    },
    "pid_reuse": {
      "worst_process": "com.android.car.cluster.osdouble",
      "pid_first": 1116, "pid_last": 10021, "pid_delta": 8905,
      "window_first": "09-24 00:27:01.587", "window_last": "09-24 00:27:52.061",
      "window_seconds": 50.474,
      "estimated_full_pid_cycles": 2.97,
      "start_proc_events": 277,
      "conclusion": "pid is an observation attribute; process_key (boot_epoch, start_seq) is the identity"
    },
    "unresolved": [
      { "reason": "event_predates_or_outlives_snapshot", "count": null,
        "note": "am_proc_start events for processes that died before 00:27:42" },
      { "reason": "isolated_process_uid_not_directly_attributable", "count": 1, "example": "u10a212i0" }
    ]
  },

  "evidence_gaps": [
    { "field": "inventory.shared_uids[].*", "reason": "the SharedUser header printer was not isolated in Settings.java; ids are read from the archive" },
    { "field": "lifecycle_events[].prefix uid coercion", "reason": "logcat uid column may be a symbolic name; uid_column_raw preserves it verbatim and no coercion is attempted at parse time" }
  ]
}
```

#### Open questions

- **`Cmd line:` is not an AMS field.** The App Developer corrected an earlier claim: `Cmd line:`
  is emitted by `art/runtime/signal_catcher.cc:55-69` (with the comment *"The string `Cmd line:` is
  chosen to match the format used by debuggerd"*), not by `ProcessRecord.dump()`
  (`ProcessRecord.java:462-565`). In the reference archive all 251 `Cmd line:` lines are inside
  `VM TRACES JUST NOW` (184) and `VM TRACES AT LAST ANR` (67) — i.e. the ANR team's evidence.
  **Consequence: `apps.json` has no `cmd_line` field.** It belongs to `anr.json`.
- **`am_kill` is zero here.** Every kill-shaped field in `attribution.kill_sources` is
  therefore `0` on this archive, and the `Rss|2|2` 2-byte-int decoding and the `Reason`
  string vocabulary could not be validated against a real sample. Treat both as
  **spec-derived, not observation-derived**.
- **`oom_adj` object field names.** `oom adj: max=… curRaw=… setRaw=… cur=… set=…` was read out
  of the process dump and `Process LRU list` for the symbol, but the exact printer was not
  pinned — it is emitted by the process-state / `ProcessList` oom-adj dump, not by
  `ProcessRecord.dump()`.
- **`capability` string `LCMNFUA`.** Mapped to
  `ActivityManager.printCapabilitiesSummary` via `ProcessList.java:4343`, but the
  per-character definition was not read. The schema carries the raw string only, with no
  decoded per-capability booleans.
- **`am_proc_start`'s `user_id` versus `uid`.** `ProcessList.java:2859` logs
  `UserHandle.getUserId(app.getStartUid())` while `ProcessRecord.java:465-466` dumps
  `user #<userId> uid=<info.uid>`. These can disagree for a process started under one uid and
  recorded under another. Both are stored rather than normalised; no real sample of the
  disagreement was found to confirm the failure mode.
- **Per-user process split.** Measured 83 records = **50 user-10, 24 user-0, 9 shared-uid**
  (the last group prints as a bare numeric uid by `ProcessRecord.java:1377-1378`, so it is not
  attributable to a user from that field alone). Not in `counts` above because it needs the same
  parse as `uid_formatted`. If it is wanted first-class:
  `counts.processes_by_user: {"0": 24, "10": 50, "shared_uid": 9}`.
- **`dumpsys usagestats` user coverage.** The section prints `user=0` and then per-user 24 h /
  7 d blocks. Whether it covers user 10 at all in this archive **is not confirmed**. If it is
  user-0-only, every `activity_events[]` row silently describes the wrong profile on a
  multi-user device — this must be checked before `activity_events` is trusted for anything.

---

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
- **By process** — wrong for the common case. The 2026-05-20 trace holds **31 processes** but
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
| `channel` | enum | yes | `C1`…`C6`, as in the channel table above. |
| `emitted_by` | string | yes | The AOSP emitter, `path:line`. |
| `present` | boolean | yes | The section existed and was non-empty. |
| `counts_ref` | string | yes | The key in `counts` holding this channel's match block. |
| `completeness` | enum | yes | `complete`, `lossy`, or `unknown`. `lossy` for C5 (rate-limited) and C6 (ring-wrapped). |
| `rate_limit_per_tag` / `rate_limit_source` | integer \| null / string | yes when applicable | `6`, `DropboxRateLimiter.java:39` `RATE_LIMIT_ALLOWED_ENTRIES_DEFAULT = 6`. |
| `decode_warning` | string \| null | yes | Set when a channel's field order contradicts its declaration. |
| `provenance` | object | yes | See [§2.4](#24-provenance). |

##### `anr_incidents[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `incident_id` | string | yes | Stable id over `(channel, wall_clock, pid, process)`. | derived |
| `process` | string | yes | Offending process name. | `ProcessErrorStateRecord.java:446` |
| `pid` | integer | yes | Offending pid. | `ProcessErrorStateRecord.java:449`; `am_anr` field 2, `logtags:17` |
| `user_id` | integer \| null | yes | | `logtags:17` field 1; `ProcessErrorStateRecord.java:374` |
| `occurred_at` | string | yes | **The only field that can carry a current freshness.** | C1 line time, `logprint.cpp:1552` |
| `activity_component` | string \| null | yes | The `(Activity)` suffix. | `ProcessErrorStateRecord.java:447-449` |
| `reason` | string \| null | yes | Verbatim `Reason:` payload. | `ProcessErrorStateRecord.java:451-453`; origin `InputManagerCallback.java:350-351` |
| `parent_component` | string \| null | yes | | `ProcessErrorStateRecord.java:454-457` |
| `error_id` | string \| null | yes | | `ProcessErrorStateRecord.java:458-460` |
| `frozen` | boolean \| null | yes | | `ProcessErrorStateRecord.java:461` |
| `app_flags_hex` | string \| null | yes | `ApplicationInfo.flags`. | `ProcessErrorStateRecord.java:374` arg 4 |
| `processes[]` | array of object | yes | Subject plus bystanders from the trace. | `ProcessErrorStateRecord.java:403-435` |
| `trace_files[]` | array of string | yes | `anr_` files attributable to this incident. | `dumpstate.cpp:395-436` |
| `evidence_channels[]` | array of string | yes | Which channels carry this incident. | — |
| `provenance` | object | yes | | — |

`processes[]` entries:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer | yes | | `backtrace.cpp:49` |
| `cmdline` | string \| null | yes | | `backtrace.cpp:52`; `libdebuggerd_client.cpp:109` |
| `is_subject` | boolean | yes | `true` only for `firstPids[0]`. **Flagged as inferred** when derived from the `----- dumping pid:` marker rather than a `Subject:` header. | `ProcessErrorStateRecord.java:396` |
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
| `count` | integer | yes | 65. | `dumpstate.cpp:203` `ANR_DIR` |
| `listing_total_kb_as_printed` | integer | yes | 20336, as printed by `ls -lt`. | `dumpstate.cpp:1204` |
| `total_bytes_approx` | integer \| null | yes | **Approximate.** Carries `total_bytes_derivation`. | — |
| `oldest_created_at` / `newest_created_at` | string \| null | yes | | `StackTracesDumpHelper.java:69-70` |
| `span_seconds` | number \| null | yes | 15530.631. | derived |
| `all_outside_log_window` | boolean | yes | `true`. | derived |
| `files[]` | array of object | yes | One per file. Below. | — |

`files[]` entries:

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `path` | string | yes | ZIP entry name. | `dumpstate.cpp:203`; zip path `dumpstate.cpp:462` |
| `file_name` | string | yes | Basename. | `StackTracesDumpHelper.java:454-457` |
| `size_bytes` | integer | yes | From the `ls -l` size column. | `dumpstate.cpp:1204` |
| `mtime_local` | string | yes | `ls` mtime, **minute granularity**. | `dumpstate.cpp:1204` |
| `created_at` | string | yes | **Millisecond** creation time decoded from the `anr_` name. | `StackTracesDumpHelper.java:69-70`, `:455` |
| `is_newest` | boolean | yes | `anr_data_[0]` — the one dumped into the main text. | `dumpstate.cpp:432-434`, `:1184` |
| `time_conflict` | boolean | yes | `true` if `mtime_local` and `created_at` disagree by > 60 s (clock changed). | derived |
| `provenance` | object | yes | | — |

##### `last_anr_trace`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `section_header` | string | yes | The literal header line, verbatim. | `DumpstateInternal.cpp:138-152` |
| `source_path` | string | yes | Path parsed out of the header. | same |
| `file_mtime_local` | string | yes | From the header. | `DumpstateInternal.cpp:146-150` |
| `trace_first_stamp` / `trace_last_stamp` | string | yes | Min/max `----- pid N at`. | `backtrace.cpp:49` |
| `process_count` | integer | yes | 31. | — |
| `is_culprit_trace` | boolean | yes | `true` when block 1 is preceded by `----- dumping pid:`. | `StackTracesDumpHelper.java:599-602` |
| `uptime_ms_at_dump` | integer \| null | yes | **Uptime, not wall clock** — and therefore a `cross_boot_reference`. | `StackTracesDumpHelper.java:599-602` |
| `processes[]` | array of object | yes | As above, with `stamped_at` required. | `backtrace.cpp:47-95` |
| `lock_contention_markers` | object | yes | `{waiting_to_lock, held_by_thread}`. | — |
| `provenance` | object | yes | `cross_boot_reference` populated here. | — |

#### Provenance and freshness

This artifact is the reason the whole convention exists. On the reference archive:

| Clock | Value | Where |
| --- | --- | --- |
| File creation (the ANR event) | `2026-05-20 05:13:42.314` | filename, `StackTracesDumpHelper.java:72` |
| File mtime | `2026-05-20 05:13:43` | section header, `DumpstateInternal.cpp:146-150` |
| First process stamp | `2026-05-20 05:13:42.327535982+0700` | `backtrace.cpp:49` |
| Last process stamp | `2026-05-20 05:13:43.782615107+0700` | `backtrace.cpp:49` |
| Archive captured | `2026-09-24 00:27:17` | `== dumpstate:` banner |
| **Age at capture** | **126 d 19 h 13 m 34 s** | derived |
| Cross-boot reference | `31190738` ms = **8 h 39 m 50.738 s of uptime on a previous boot** | `StackTracesDumpHelper.java:599-602` |

Meanwhile **this boot had zero ANRs**, provable three independent ways:

1. `ANR in ` → **0** occurrences in the whole 64 MB archive.
2. `am_anr` → **0** event-log entries.
3. Two dumpsys sections assert it in so many words — archive line **7765**
   (`WINDOW MANAGER LAST ANR (dumpsys window lastanr)`) and line **385577**
   (`ACTIVITY MANAGER LAST ANR (dumpsys activity lastanr)`) both print
   `  <no ANR has occurred since boot>`.

…while the `ANR FILES` listing at archive line **318023** shows 65 files:

```
------ ANR FILES (ls -lt /data/anr/) ------
total 20336
-rw------- 1 system system 714307 2026-05-20 05:13 anr_2026-05-20-05-13-42-314
-rw------- 1 system system 438455 2026-05-20 05:13 anr_2026-05-20-05-13-41-273
...
-rw------- 1 system system   4910 2026-05-20 00:54 anr_2026-05-20-00-54-11-683
------ 0.006s was the duration of 'ANR FILES' ------
```

**65 files, `total 20336` KB ≈ 20.3 MB, spanning 4 h 19 m 30.631 s — all 126 days stale, all
predating this boot entirely.** The correct artifact states both facts and lets
`negative_evidence` carry the authoritative one.

Why the files were not pruned: `maybePruneOldTraces` (`StackTracesDumpHelper.java:512-530`)
keeps `min(64, 1 day)` files but **only runs immediately after a new ANR file is written**
(`:183`). 65 files persist indefinitely because the archive has more than `max+1` survivors and
no new ANR ever arrives to trigger a prune. **The retention rule is event-driven, not
clock-driven.**

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
      "not_emitted_by_platform": 1, "suppressed_noise_note": null,
      "coverage": { "window_start": "2026-09-24T00:26:59.872+07:00",
                    "window_end": "2026-09-24T00:27:20.176+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 0.314 },
      "regions": []
    },
    "anr_file_listing_hits": {
      "pattern": "^anr_[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]{3}$",
      "pattern_kind": "line_regex", "case_sensitive": true,
      "raw_count": 66, "matched_evidence": 65, "suppressed_noise": 1,
      "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the 'total 20336' summary line of 'ls -lt' is not a file",
      "coverage": { "window_start": null, "window_end": null,
                    "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "ANR FILES",
                     "section_ordinal": 0, "line_start": 318025, "line_end": 318089, "line_count": 65 } ]
    }
  },

  "channels": [
    { "channel": "C1", "emitted_by": "ProcessErrorStateRecord.java:563", "present": false,
      "counts_ref": "anr_tag_hits", "completeness": "complete" },
    { "channel": "C2", "emitted_by": "dumpstate.cpp:1204", "present": true,
      "counts_ref": "anr_file_listing_hits", "completeness": "complete" },
    { "channel": "C3", "emitted_by": "dumpstate.cpp:1184", "present": true,
      "counts_ref": "last_anr_trace_present", "completeness": "complete" },
    { "channel": "C4", "emitted_by": "ActivityTaskManagerService.java:4254", "present": true,
      "counts_ref": "lastanr_dump_hits", "completeness": "complete" },
    { "channel": "C5", "emitted_by": "ActivityManagerService.java:9766", "present": false,
      "counts_ref": "dropbox_anr_hits", "completeness": "lossy",
      "rate_limit_per_tag": 6,
      "rate_limit_source": "DropboxRateLimiter.java:39 RATE_LIMIT_ALLOWED_ENTRIES_DEFAULT=6",
      "decode_warning": "dumpstate requests only -p system_server_crash and -p system_app_crash (dumpstate.cpp:1858-1859); ANR tags reach the archive only as ZIP entries under FS/data/system/dropbox, and only on non-user builds (dumpstate.cpp:1957-1960)" },
    { "channel": "C6", "emitted_by": "ProcessErrorStateRecord.java:374", "present": true,
      "counts_ref": "am_anr_hits", "completeness": "lossy",
      "decode_warning": "event-log ring wrapped; buffer covers only 20.304 s of the 64.662 s window" }
  ],

  "anr_incidents": [],

  "trace_files": {
    "count": 65,
    "listing_total_kb_as_printed": 20336,
    "total_bytes_approx": 20824064,
    "total_bytes_derivation": "listing_total_kb_as_printed * 1024; ls -lt prints KB in 1K blocks and the exact byte total is not printed",
    "oldest_created_at": "2026-05-20T00:54:11.683+07:00",
    "newest_created_at": "2026-05-20T05:13:42.314+07:00",
    "span_seconds": 15530.631,
    "all_outside_log_window": true,
    "files": [
      { "path": "FS/data/anr/anr_2026-05-20-05-13-42-314",
        "file_name": "anr_2026-05-20-05-13-42-314",
        "size_bytes": 714307, "mtime_local": "2026-05-20T05:13:00",
        "created_at": "2026-05-20T05:13:42.314+07:00",
        "is_newest": true, "time_conflict": false,
        "provenance": { "evidence_origin": "C2_anr_file_listing", "archive_path": "<main_entry>",
          "section_title": "ANR FILES", "section_ordinal": 0,
          "line_start": 318025, "line_end": 318025,
          "captured_at": "2026-05-20T05:13:42.314+07:00", "captured_at_time_base": "file_name",
          "evidence_time_source": "filename_timestamp",
          "age_seconds_at_capture": 10996814, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "filename timestamp",
            "basis": "ANR file name encodes creation time (StackTracesDumpHelper.java:455); 126.80 days before capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false,
          "collector_perturbed_basis": "0.006 s ls -lt pass" } },
      { "path": "FS/data/anr/anr_2026-05-20-00-54-11-683",
        "file_name": "anr_2026-05-20-00-54-11-683",
        "size_bytes": 4910, "mtime_local": "2026-05-20T00:54:00",
        "created_at": "2026-05-20T00:54:11.683+07:00",
        "is_newest": false, "time_conflict": false,
        "provenance": { "section_title": "ANR FILES", "section_ordinal": 0,
          "line_start": 318089, "line_end": 318089,
          "captured_at": "2026-05-20T00:54:11.683+07:00", "captured_at_time_base": "file_name",
          "age_seconds_at_capture": 10999982, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "filename timestamp",
            "basis": "oldest of 65 retained files; 4h19m30.631s before the newest",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ]
  },

  "last_anr_trace": {
    "section_header": "------ VM TRACES AT LAST ANR (/data/anr/anr_2026-05-20-05-13-42-314: 2026-05-20 05:13:43) ------",
    "source_path": "/data/anr/anr_2026-05-20-05-13-42-314",
    "file_mtime_local": "2026-05-20T05:13:43",
    "trace_first_stamp": "2026-05-20T05:13:42.327535982+07:00",
    "trace_last_stamp": "2026-05-20T05:13:43.782615107+07:00",
    "process_count": 31,
    "is_culprit_trace": true,
    "uptime_ms_at_dump": 31190738,
    "processes": [
      { "pid": 17511, "cmdline": "system_server", "is_subject": true,
        "stamped_at": "2026-05-20T05:13:42.327535982+07:00",
        "java_thread_count": 219, "native_frame_count": null, "waiter_count": 2 },
      { "pid": 448, "cmdline": "/apex/com.google.android.widevine/bin/hw/android.hardware.drm-service.widevine",
        "is_subject": false, "stamped_at": "2026-05-20T05:13:42.721675565+07:00",
        "java_thread_count": null, "native_frame_count": 5, "waiter_count": 0 },
      { "pid": 445, "cmdline": null, "is_subject": false,
        "stamped_at": "2026-05-20T05:13:43.730310899+07:00",
        "java_thread_count": null, "native_frame_count": null, "waiter_count": null }
    ],
    "lock_contention_markers": { "waiting_to_lock": 2, "held_by_thread": 1 },
    "provenance": { "evidence_origin": "C3_last_anr_trace", "archive_path": "<main_entry>",
      "section_title": "VM TRACES AT LAST ANR", "section_ordinal": 0,
      "line_start": 309986, "line_end": 318022,
      "captured_at": "2026-05-20T05:13:42.314+07:00", "captured_at_time_base": "file_section_header_mtime",
      "evidence_time_source": "embedded_trace_stamp",
      "age_seconds_at_capture": 10996814, "capture_kind": "disk_artifact",
      "cross_boot_reference": 31190738,
      "freshness": "stale",
      "freshness_basis": { "input": "section header mtime and all 31 '----- pid N at' stamps",
        "basis": "they agree on 2026-05-20 05:13:4x, 126.80 days before capture; cross-boot reference 31190738 ms of uptime is 1.8e6 x capture uptime 17 s, so the trace is from a previous boot",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "the trace was dumped from disk; dumpstate did not generate it" }
  },

  "negative_evidence": [
    { "scope": "C4 — ACTIVITY MANAGER LAST ANR",
      "statement": "  <no ANR has occurred since boot>",
      "source": { "archive_path": "<main_entry>", "section_title": "DUMP OF SERVICE activity:",
                  "line_start": 385576, "line_end": 385577 },
      "authority": "platform_assertion" },
    { "scope": "C4 — WINDOW MANAGER LAST ANR",
      "statement": "  <no ANR has occurred since boot>",
      "source": { "archive_path": "<main_entry>", "section_title": "DUMPSYS CRITICAL",
                  "line_start": 7764, "line_end": 7765 },
      "authority": "platform_assertion" },
    { "scope": "C1 — logcat 'ANR in ' tag",
      "statement": "0 occurrences of the literal 'ANR in ' across the whole archive",
      "source": { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG",
                  "section_ordinal": 0, "line_start": 9117, "line_end": 53708 },
      "authority": "archive_absence" },
    { "scope": "C6 — EVENT LOG am_anr",
      "statement": "0 'am_anr' entries in the EVENT LOG section",
      "source": { "archive_path": "<main_entry>", "section_title": "EVENT LOG",
                  "section_ordinal": 0, "line_start": 53710, "line_end": 55656 },
      "authority": "archive_absence" }
  ],

  "findings": [
    { "id": "anr-0001", "kind": "anr_now_summary",
      "title": "No ANR occurred during this boot; 65 ANR trace files from 2026-05-20 are present and are 126.80 days stale",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "anr_incidents_count": 0, "anr_files_on_disk": 65,
                      "anr_files_outside_log_window": 65 },
      "metrics": { "newest_anr_file_age_seconds": 10996814, "oldest_anr_file_age_seconds": 10999982,
                   "anr_file_span_seconds": 15530.631, "cross_boot_uptime_ms": 31190738 },
      "evidence": { "archive_path": "<main_entry>", "kind": "section",
                    "locator": { "line_start": 318023, "line_end": 318090 },
                    "excerpt": "------ ANR FILES (ls -lt /data/anr/) ------\ntotal 20336\n-rw------- 1 system system 714307 2026-05-20 05:13 anr_2026-05-20-05-13-42-314",
                    "excerpt_truncated": false, "excerpt_bytes": 175, "excerpt_complete": true },
      "corroborating_evidence": [
        { "archive_path": "FS/data/anr/anr_2026-05-20-05-13-42-314", "kind": "file_entry",
          "locator": { "line_start": 1, "line_end": 1 },
          "excerpt": "----- dumping pid: 17511 at 31190738\nproc 17511\n…",
          "excerpt_truncated": true, "excerpt_bytes": 512, "excerpt_complete": false }
      ],
      "provenance": { "evidence_origin": "C2_anr_file_listing", "archive_path": "<main_entry>",
        "section_title": "ANR FILES", "section_ordinal": 0,
        "line_start": 318023, "line_end": 318090,
        "captured_at": "2026-05-20T05:13:42.314+07:00", "captured_at_time_base": "file_name",
        "age_seconds_at_capture": 10996814, "capture_kind": "disk_artifact",
        "freshness": "stale",
        "freshness_basis": { "input": "filename timestamp + cross-boot reference",
          "basis": "126.80 days old and from a boot that ended before this one; see last_anr_trace.provenance",
          "threshold_seconds": 3600 },
        "cross_boot_reference": 31190738, "collector_perturbed": false,
        "collector_perturbed_basis": "ls -lt pass over an existing directory" } }
  ],

  "notes": [
    "ANR_TAG_ABSENT_BUT_FILES_PRESENT: 'ANR in ' matched 0 times and that is NOT evidence of no ANR; 65 trace files exist on disk from a previous boot. Source ANRs from ANR FILES (dumpstate.cpp:1204) and FS/data/anr/ (dumpstate.cpp:1188), never from a logcat tag.",
    "ANR_RETENTION_EVENT_DRIVEN: maybePruneOldTraces (StackTracesDumpHelper.java:512-530) runs only after a new ANR is written (:183), so 65 files can persist past the 64-file cap indefinitely.",
    "ANRD_NOT_A_DETECTOR: the 'anrd' module is a CPU-starvation ftrace collector (system/extras/ANRdaemon/README:1-14); it is also not built for this product. It contributes no field to this artifact."
  ]
}
```

#### Open questions

- **No ANR subject for the 2026-05-20 trace.** The newest trace's first block is
  `system_server` (pid 17511) preceded by `----- dumping pid: 17511 at 31190738`, so the
  culprit *appears* to have been `system_server`. But the trace has **no `Subject:` header**
  (`StackTracesDumpHelper.java:168-176` writes one only when `subject`/`criticalEventSection`/
  `extraHeaders` are non-null), and no `ANR in` line survives from that boot. **`is_subject`
  is emitted from the `----- dumping pid:` marker only and is flagged as inferred.** No ANR
  subject is attributed for the stale trace.
- **C5 dropbox ANR reachability.** `dumpstate` requests only `-p system_server_crash` and
  `-p system_app_crash` (`dumpstate.cpp:1858-1859`), so ANR dropbox entries reach the archive
  only as ZIP entries under `FS/data/system/dropbox`, and only on non-`user` builds
  (`dumpstate.cpp:1957-1960`). This archive is `userdebug`, so the directory *should* be
  present — but it contains no `*_anr` files, consistent with zero ANRs. The **`user`-build
  behaviour is not verified**, so C5's completeness is `lossy`, not `complete`.
- **`trace_files.total_bytes_approx`.** `ls -lt` prints `total 20336` in 1K blocks; the exact
  byte total is not in the archive. The approximation carries its derivation, per
  [§2.9](#29-null-absent-and-raw) rule 4.
- **ANR file timestamps are well-dated.** An earlier claim that the trace carries "no
  timestamp inside" is **refuted**: every process block carries a nanosecond wall clock —
  `----- pid 17511 at 2026-05-20 05:13:42.327535982+0700 -----` (`backtrace.cpp:49`), 31 such
  lines in the section. **Staleness here is provable, not inferred.**
- **The `ANR FILES` `ls` column is minute-granular** while the filename is millisecond. The
  `time_conflict` field exists to catch a device clock change; it is `false` on both files
  checked.

---

### 4.5 `crashes.json`

#### Purpose

`crashes.json` answers: **what crashed, how often, in what pattern, and how old is the
evidence?** The central modelling problem is that a bugreport records crashes in **five
mutually inconsistent channels with different completeness guarantees**, and a single 64 MB
file can contain one 271-crash loop from this boot and one 50-file native-crash incident from
four months ago. The schema must keep them apart and must not let a lossy channel's count
stand in for the truth.

#### The channels

| Channel | Emitted by | Guarantee | Value in the reference archive |
| --- | --- | --- | --- |
| `C1_crash_buffer` (`FATAL EXCEPTION`) | `RuntimeInit.java:84-86` → `Log.printlns(Log.LOG_ID_CRASH, …)` | ring buffer, complete until wrap | **271 distinct** |
| `C2_am_crash` | `EventLogTags.logtags:56` | ring-wrapped, lossy | **39**, last at `00:27:20.158` — misses 87 % |
| `C3_dropbox_crash` | `AMS:9766`; rate-limited to 6 per `(process, eventType)` (`DropboxRateLimiter.java:39`) | rate-limited | **6** `osdouble` entries, all within `00:27:01`–`00:27:04` |
| `C4_tombstone_files` | `tombstoned.cpp:127-133` | ring of `tombstoned.max_tombstone_count` (default 32) | **50 + 50 `.pb`**, all `2026-05-20` |
| `C5_am_proc_died` | `EventLogTags.logtags:22` | ring-wrapped | 39 (coverage 0.314) |
| `C6_manager_kill` | `AppErrors.java:902` | one per manager decision | **273** |
| `C7_rescueparty` | `RescueParty.java:822` | one per mitigation attempt | **55** |

**Rule adopted: the crash count is always taken from the crash buffer (`C1`), the only complete
channel.** The other six are attached as *corroboration with their own counts and their own
completeness*, never as the headline number.

**Java and native crashes are separated by design.** A consumer never has to filter by
language or date to avoid mixing a 126-day-old native incident with a live Java loop. The
distinguishing test is purely structural:

| Test | Java | Native |
| --- | --- | --- |
| Where it lives | main-entry `SYSTEM LOG` text | **ZIP entry** `FS/data/tombstones/tombstone_NN` |
| Header line | `--------- beginning of crash` | `*** *** *** *** …` (`tombstone_proto_to_text.cpp:610`) |
| Identity line | `AndroidRuntime: Process: <pkg>, PID: <n>` | `pid: <p>, tid: <t>, name: <n>  >>> <proc> <<<` (`:125`) |
| Frame format | `\tat pkg.Class.method(File.java:123)` | `    #00 pc <addr>  <so> (sym+off) (BuildId: …)` |
| Own clock | line timestamp only | `Timestamp:` line, ns + tz offset (`:616`) |

In the reference archive **all 50 native entries** match the tombstone shape and **all 274
`FATAL EXCEPTION` lines** match the Java shape. No overlap, no ambiguity.

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`, plus `pid_max` and `min_crash_interval_ms` if read. |
| `channels` | array of object | yes | One entry per channel, with `authoritative_for_count`. |
| `java_crash_groups` | array of object | yes | Java crash signatures with their occurrence sets. |
| `java_crash_incidents` | array of object | yes | One entry per distinct Java crash. |
| `native_crash_groups` | array of object | yes | Native crash signatures. |
| `native_crash_incidents` | array of object | yes | One entry per distinct native crash. |

#### Grouping key

```
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

A group is a **crash loop** when all five hold:

| Criterion | Field | Threshold | Source of the threshold |
| --- | --- | --- | --- |
| Repeat count | `repeat_count` | ≥ 5 | Above `RATE_LIMIT_ALLOWED_ENTRIES_DEFAULT = 6` (`DropboxRateLimiter.java:39`) — a rate limiter that lets 6 through is evidence the platform itself already saw 5+ as "the same problem". |
| Rate | `rate_per_second` | ≥ 1.0 | The loop must be faster than human retry. |
| Window | `window_seconds` | ≤ 300 | `EXPIRED_REPORT_TIME_MS = 10 s` and `CONSECUTIVE_ANR_TIME_MS = 2 min` (`AnrHelper.java:56,61`) are the platform's own "this is one event, not a trend" bounds. |
| Continuity | `inter_gap_max_seconds` | ≤ 120 (= `MIN_CRASH_INTERVAL`) | The platform's own `quickCrash` test is `now < crashTime + MIN_CRASH_INTERVAL` (`AppErrors.java:896-897`), with `MIN_CRASH_INTERVAL = 2*60*1000` (`ActivityManagerConstants.java:227,636`), dumped as `min_crash_interval=120000`. |
| Escalation | `manager_kill_count` or `rescueparty_failure_count` | ≥ 1 | AMS and RescueParty both act only on repetition: `AppErrors.java:897`, `RescueParty.java:822`. |

The `osdouble` group satisfies all five with margin. The bluetooth tombstone group satisfies
**none** of the window/continuity/escalation criteria — its occurrences span 2 h 35 m and no
manager action is recorded in the log window — so it is correctly classified
`stale_recurring`, **not** `crash_loop`. **That distinction is the whole point.**

#### Field tables

##### `crash_groups[]` (`java_crash_groups`, `native_crash_groups`)

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `group_id` | string | yes | `sha1(signature)`, truncated. | derived |
| `language` | enum | yes | `java` / `native`. | §above |
| `signature.exception_class` | string \| null | yes (java) | | `RuntimeInit.java:84-86` |
| `signature.signal_number` / `signal_name` | integer / string \| null | yes (native) | | `tombstone_proto_to_text.cpp:476` |
| `signature.sigcode_name` | string \| null | yes (native) | e.g. `SI_QUEUE`. | same |
| `signature.top_frame` | string | yes | Normalised, no line numbers. | §grouping key |
| `signature.abort_message` | string \| null | yes (native) | Sub-grouping hint, **not** in the key. | `tombstone_proto_to_text.cpp:491` |
| `packages[]` / `package_count` | array / integer | yes | May exceed 1 for `:remote` loops. | `RuntimeInit.java:94` |
| `repeat_count` | integer | yes | **Distinct** occurrences, de-duplicated. | §274-vs-271 |
| `raw_line_hits` | integer | yes | Every matching line **before** de-duplication. | — |
| `duplicate_hits` | integer | yes | `raw_line_hits − repeat_count`. | §274-vs-271 |
| `duplicate_reason` | string \| null | yes when `duplicate_hits > 0` | `"inlined dropbox logcat copy"` + citation. | `AMS:9890-9900` |
| `first_seen` / `last_seen` | string | yes | Min/max occurrence time. | `logprint.cpp:1552` / `tombstone_proto_to_text.cpp:616` |
| `window_seconds` | number | yes | `last − first`. | derived |
| `rate_per_second` | number \| null | yes | Carries `rate_derivation`. | derived |
| `inter_gap_p50_ms` / `inter_gap_max_ms` | integer \| null | yes | Derived over the gaps between occurrence times. | derived |
| `pid_span` | object | yes | `{first, last, distinct_count, pid_max}`. `null` members when the pids are cross-boot and meaningless against this boot. | `logprint.cpp:1553` |
| `classification` | enum | yes | `crash_loop` / `recurring` / `isolated` / `stale_recurring`. | §loop versus isolated |
| `classification_reason` | string | yes | Names the thresholds met and failed. | derived |
| `manager_kill_count` | integer | yes | | `AppErrors.java:902` |
| `manager_kill_reason` | string \| null | yes | `crashed quickly` / `over process crash limit`. | `AppErrors.java:903` |
| `rescueparty_failure_count` | integer | yes | | `RescueParty.java:822` |
| `rescueparty_disabled` | boolean \| null | yes | | observed in archive |
| `corroboration[]` | array of object | yes | One entry per channel, each with its own count and completeness. | §channels |
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
| `evidence_channels[]` | array of string | yes | | §channels |
| `provenance` | object | yes | | — |

#### The 274-versus-271 finding

Raw `FATAL EXCEPTION` count is **274**. Decomposing by region:

| Region | Line range | `FATAL EXCEPTION` |
| --- | --- | --- |
| A — first `SYSTEM LOG` (`logcat -v threadtime … -d *:v`) | 9116–53708 | 38 |
| B — `DROPBOX SYSTEM APP CRASHES` (inlined logcat) | 53709–653372 | **3** |
| C — trailing `SYSTEM LOG` (`… -T 2026-09-24 00:27:20.000`) | 653373–end | 233 |
| | **total** | **274** |

The 3 in region B are the crashes of pids **5769, 9384, 9408** — each of which *also* appears
once in region C. They are the same three events, printed a second time because
`addErrorToDropBox` inlines logcat into the dropbox body (`ActivityManagerService.java:9890-9900`,
`logcatLines` from `ERROR_LOGCAT_PREFIX`), and dumpstate renders
`dumpsys dropbox -p system_app_crash` at `dumpstate.cpp:1859`.

**Therefore: 271 distinct crashes, 274 raw lines.** This is the "a raw grep count is never a
finding" rule demonstrated rather than asserted — and the naive `PID:` key produces the same
three phantom repeats, because those are exactly the three pids that appear twice.

**PID-recycling claim, corrected.** The earlier "PIDs recycled ~3 times" reading is wrong. 271
crashes produced **271 distinct, monotonic PIDs** (1116 → 9985). The three apparent repeats
were the dropbox duplicates. This *strengthens* the argument against a PID key rather than
weakening it: because the PIDs are monotonic, PID-keying produces 271 singleton findings and
the loop structure is completely invisible.

#### Provenance and freshness

Two incidents live in one archive and must never be conflated:

**Incident 1 — the live Java loop.** Every occurrence is inside `capture.log_window` and
carries `captured_at_time_base: "logcat_line"`. The 271st crash at `00:27:51.970` is the last
line of the crash buffer, so the loop is **still running at capture**.

| Fact | Value | Read from |
| --- | --- | --- |
| Package | `com.android.car.cluster.osdouble` | `AndroidRuntime: Process:` — the only crashing package |
| Exception | `java.lang.IllegalArgumentException` — the only exception type, 274/274 | 3rd line of each block |
| Top frame | `com.android.internal.util.Preconditions.checkArgumentNonnegative(Preconditions.java:347)` | frame 1 |
| Next frames | `android.view.WindowInsets.inset(WindowInsets.java:1155)` → `:1128` → `PhoneWindow.lambda$static$0(PhoneWindow.java:166)` → `PhoneWindow$$ExternalSyntheticLambda1.onContentApplyWindowInsets` | frames 2–5 |
| First / last crash | `00:27:01.751` (pid 1116) / `00:27:51.970` (pid 9985) | |
| Window | **50.219 s** | derived |
| Rate | **5.396 crashes/s** | derived |
| Inter-crash gaps (270 gaps) | min 84 ms, **p50 133 ms**, **max 1851 ms** | derived |
| Distinct pids | **271 of 271**, monotonic 1116 → 9985 | `pid_max = 32768` |
| Manager kills / RescueParty | **273** / **55** | `AppErrors.java:902` / `RescueParty.java:822` |
| Kill reason | `Reason: crashed` (`quickCrash`, `MIN_CRASH_INTERVAL = 120000`) | `ActivityManagerConstants.java:227,636`; `min_crash_interval=120000` at line 365413 |
| uid | 1000 (system) | logcat uid column |

**Incident 2 — the old native group.** Every tombstone is dated 2026-05-20.

| Fact | Value |
| --- | --- |
| File mtimes | `2026-05-20 02:35:42` → `2026-05-20 05:10:56` (span 2 h 35 m 14 s) |
| In-file `Timestamp:` on `tombstone_00` | `2026-05-20 04:00:14.502971422+0700` |
| `Cmdline:` | `com.google.android.bluetooth` — **50 of 50** |
| `signal` | `6 (SIGABRT)` — **50 of 50** |
| Example `Abort message:` | `'system/gd/hci/hci_layer.cc:214 handle_command_response: Waiting for SET_EVENT_MASK(0x0c01), got RESET(0x…)'` |
| Age at capture | **126.80 days** |
| Grouping | **one** incident, 50 occurrences |

This is a completely separate incident from the `osdouble` loop: different language, different
signal class, different date, and it never touched `com.android.car.cluster.osdouble`.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "crashes",

  "capture_ref": { "source": "system.json#/capture", "pid_max": 32768, "min_crash_interval_ms": 120000 },

  "counts": {
    "fatal_exception_hits": {
      "pattern": "FATAL EXCEPTION", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 274, "matched_evidence": 271, "suppressed_noise": 3, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "3 crashes (pid 5769, 9384, 9408) are inlined a second time inside DROPBOX SYSTEM APP CRASHES by fetchLogcatBuffers (ActivityManagerService.java:9890-9900); the prefix uid differs (5769 vs 1000) while the message pid matches, which is the signature of a re-rendered record",
      "coverage": { "window_start": "2026-09-24T00:26:47.418+07:00", "window_end": "2026-09-24T00:27:52.080+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
          "line_start": 27742, "line_end": 53708, "line_count": 38 },
        { "archive_path": "<main_entry>", "section_title": "DROPBOX SYSTEM APP CRASHES", "section_ordinal": 0,
          "line_start": 53709, "line_end": 653372, "line_count": 3 },
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 1,
          "line_start": 653373, "line_end": 682949, "line_count": 233 }
      ]
    },
    "tombstone_hits": {
      "pattern": "tombstone", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 749, "matched_evidence": 0, "suppressed_noise": 749, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "no '*** *** ***' tombstone banner exists anywhere in the main entry; the 50 real tombstones live in FS/data/tombstones/ as ZIP entries, which is where this artifact looks",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
          "line_start": 9117, "line_end": 53709, "line_count": 297 },
        { "archive_path": "<main_entry>", "section_title": "SMAPS OF ALL PROCESSES", "section_ordinal": 0,
          "line_start": 62923, "line_end": 62972, "line_count": 2 }
      ]
    },
    "signal_hits": {
      "pattern": "signal ", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 656, "matched_evidence": 0, "suppressed_noise": 656, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "417 'signal 9' are init teardown (system/core/init/service.cpp:206-207); 40 'signal 3' are inside Signal Catcher thread dumps. Fatal signals use the form '^signal \\d+ \\(SIG[A-Z0-9]+\\), code' (tombstone_proto_to_text.cpp:476), which occurs 0 times in the main entry",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "am_crash_hits": {
      "pattern": " am_crash:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 39, "matched_evidence": 39, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "2026-09-24T00:26:59.872+07:00", "window_end": "2026-09-24T00:27:20.176+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 0.314 },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0,
          "line_start": 53710, "line_end": 55656, "line_count": 39 }
      ]
    }
  },

  "channels": [
    { "channel": "C1_crash_buffer", "emitted_by": "RuntimeInit.java:84-86", "present": true,
      "counts_ref": "fatal_exception_hits", "completeness": "complete", "authoritative_for_count": true },
    { "channel": "C2_am_crash", "emitted_by": "EventLogTags.logtags:56", "present": true,
      "counts_ref": "am_crash_hits", "completeness": "lossy", "authoritative_for_count": false,
      "loss_cause": "event-log ring wrapped; last entry 00:27:20.158 vs crash buffer 00:27:51.970 — 87% of crashes absent",
      "decode_warning": "observed slot order is (PID, User, …) but logtags:56 declares (User, PID, …); no AM_CRASH write site found in frameworks/base — pid is taken from the am_proc_died/am_proc_start pairing" },
    { "channel": "C3_dropbox_crash", "emitted_by": "ActivityManagerService.java:9766", "present": true,
      "counts_ref": "dropbox_crash_hits", "completeness": "lossy", "authoritative_for_count": false,
      "rate_limit_per_tag": 6, "rate_limit_source": "DropboxRateLimiter.java:39" },
    { "channel": "C4_tombstone_files", "emitted_by": "tombstoned.cpp:127-133", "present": true,
      "counts_ref": "tombstone_entry_records", "completeness": "lossy", "authoritative_for_count": false,
      "ring_default": 32, "ring_observed": 50,
      "note": "tombstone_NN is a RING COUNTER (tombstoned.cpp:196-206); tombstone_00 is NOT the newest file and the number carries no time meaning whatsoever" },
    { "channel": "C5_am_proc_died", "emitted_by": "EventLogTags.logtags:22", "present": true,
      "counts_ref": "am_proc_died_hits", "completeness": "lossy", "authoritative_for_count": false },
    { "channel": "C6_manager_kill", "emitted_by": "AppErrors.java:902", "present": true,
      "counts_ref": "crashed_too_many_hits", "completeness": "indicative", "authoritative_for_count": false },
    { "channel": "C7_rescueparty", "emitted_by": "RescueParty.java:822", "present": true,
      "counts_ref": "rescueparty_hits", "completeness": "indicative", "authoritative_for_count": false }
  ],

  "java_crash_groups": [
    { "group_id": "java:java.lang.IllegalArgumentException:Preconditions.checkArgumentNonnegative",
      "language": "java",
      "signature": {
        "exception_class": "java.lang.IllegalArgumentException",
        "signal_number": null, "signal_name": null, "sigcode_name": null,
        "top_frame": "com.android.internal.util.Preconditions.checkArgumentNonnegative",
        "abort_message": null
      },
      "packages": ["com.android.car.cluster.osdouble"], "package_count": 1,
      "repeat_count": 271,
      "raw_line_hits": 274, "duplicate_hits": 3,
      "duplicate_reason": "inlined dropbox logcat copy, ActivityManagerService.java:9890-9900",
      "first_seen": "2026-09-24T00:27:01.751+07:00",
      "last_seen": "2026-09-24T00:27:51.970+07:00",
      "window_seconds": 50.219,
      "rate_per_second": 5.396,
      "rate_derivation": "repeat_count / window_seconds = 271 / 50.219",
      "inter_gap_p50_ms": 133, "inter_gap_max_ms": 1851,
      "pid_span": { "first": 1116, "last": 9985, "distinct_count": 271, "pid_max": 32768 },
      "classification": "crash_loop",
      "classification_reason": "repeat_count 271 >= 5; rate 5.396/s >= 1.0; window 50.219s <= 300s; inter_gap_max 1.851s <= MIN_CRASH_INTERVAL 120s; manager_kill_count 273 >= 1; rescueparty_failure_count 55 >= 1",
      "manager_kill_count": 273, "manager_kill_reason": "crashed",
      "rescueparty_failure_count": 55, "rescueparty_disabled": true,
      "corroboration": [
        { "channel": "C2_am_crash", "count": 39, "completeness": "lossy" },
        { "channel": "C3_dropbox_crash", "count": 6, "completeness": "lossy" },
        { "channel": "C5_am_proc_died", "count": 39, "completeness": "lossy" },
        { "channel": "C6_manager_kill", "count": 273, "completeness": "indicative" },
        { "channel": "C7_rescueparty", "count": 55, "completeness": "indicative" }
      ],
      "provenance": { "evidence_origin": "C1_crash_buffer", "archive_path": "<main_entry>",
        "section_title": "SYSTEM LOG", "section_ordinal": 0,
        "line_start": 9116, "line_end": 53708,
        "captured_at": "2026-09-24T00:27:01.751+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 15.249, "capture_kind": "live_log",
        "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "first and last crash lines both fall inside capture.log_window, and the 271st crash is the last line of the crash buffer, so the loop is still running at capture",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "logcat captures as the crashes occur; the dump neither causes nor suppresses them" } }
  ],

  "native_crash_groups": [
    { "group_id": "native:6:SIGABRT:system/gd/hci/hci_layer.cc",
      "language": "native",
      "signature": {
        "exception_class": null,
        "signal_number": 6, "signal_name": "SIGABRT", "sigcode_name": "SI_QUEUE",
        "top_frame": "bluetooth::hci::HciLayer::*",
        "abort_message": "system/gd/hci/hci_layer.cc:214 handle_command_response: Waiting for SET_EVENT_MASK(0x0c01), got RESET(0x0c00)"
      },
      "packages": ["com.google.android.bluetooth"], "package_count": 1,
      "repeat_count": 50,
      "raw_line_hits": 50, "duplicate_hits": 0, "duplicate_reason": null,
      "first_seen": "2026-05-20T02:35:42+07:00",
      "last_seen": "2026-05-20T05:10:56+07:00",
      "window_seconds": 9314,
      "rate_per_second": 0.0054,
      "rate_derivation": "repeat_count / window_seconds = 50 / 9314",
      "inter_gap_p50_ms": null, "inter_gap_max_ms": null,
      "pid_span": { "first": null, "last": null, "distinct_count": null, "pid_max": 32768 },
      "classification": "stale_recurring",
      "classification_reason": "window 9314s > 300s and rate 0.0054/s < 1.0; no manager_kill or rescueparty corroboration inside the log window; every tombstone predates this boot by 126.80 days",
      "manager_kill_count": 0, "manager_kill_reason": null,
      "rescueparty_failure_count": 0, "rescueparty_disabled": null,
      "corroboration": [ { "channel": "C4_tombstone_files", "count": 50, "completeness": "lossy" } ],
      "provenance": { "evidence_origin": "C4_tombstone_files", "archive_path": "<main_entry>",
        "section_title": "DUMP OF SERVICE dropbox", "section_ordinal": 0,
        "line_start": 652146, "line_end": 652155,
        "captured_at": "2026-05-20T04:00:14.502971422+07:00", "captured_at_time_base": "tombstone_header",
        "evidence_time_source": "embedded_trace_stamp",
        "age_seconds_at_capture": 10996814, "capture_kind": "disk_artifact",
        "freshness": "stale",
        "freshness_basis": { "input": "in-file 'Timestamp:' (tombstone_proto_to_text.cpp:616)",
          "basis": "126.80 days before capture; the tombstone_NN filename index is a ring counter (tombstoned.cpp:205) and carries no time meaning",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "the tombstones were read from disk; dumpstate did not generate them" } }
  ],

  "java_crash_incidents": [
    { "incident_id": "crash-0001",
      "group_id": "java:java.lang.IllegalArgumentException:Preconditions.checkArgumentNonnegative",
      "language": "java",
      "package": "com.android.car.cluster.osdouble", "process": "com.android.car.cluster.osdouble",
      "pid": 1116, "prefix_pid": 1116, "prefix_uid": 1000,
      "thread_name": "main",
      "exception_class": "java.lang.IllegalArgumentException",
      "exception_message": null,
      "top_frames": [
        "com.android.internal.util.Preconditions.checkArgumentNonnegative(Preconditions.java:347)",
        "android.view.WindowInsets.inset(WindowInsets.java:1155)",
        "android.view.WindowInsets.inset(WindowInsets.java:1128)",
        "com.android.internal.policy.PhoneWindow.lambda$static$0(PhoneWindow.java:166)",
        "com.android.internal.policy.PhoneWindow$$ExternalSyntheticLambda1.onContentApplyWindowInsets(D8$$Synthesized)"
      ],
      "occurred_at": "2026-09-24T00:27:01.751+07:00",
      "flags_hex": "0x3088be4d",
      "crash_handler": "com.android.internal.os.RuntimeInit$KillApplicationHandler",
      "evidence_channels": ["C1_crash_buffer", "C2_am_crash", "C3_dropbox_crash", "C5_am_proc_died"],
      "provenance": { "evidence_origin": "C1_crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": 27742, "line_end": 28819,
        "captured_at": "2026-09-24T00:27:01.751+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 15.249, "capture_kind": "live_log",
        "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "inside capture.log_window; corroborated by dropbox 'Timestamp: 2026-09-24 00:27:01.754+0700' agreeing within 3 ms (AMS:9783-9786)",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "negative_evidence": [
    { "scope": "native tombstone search in the main entry",
      "statement": "0 occurrences of the tombstone banner '*** *** *** ***' in the 682,951-line main entry; the 50 native crashes exist only as ZIP entries under FS/data/tombstones/",
      "source": { "archive_path": "<main_entry>", "section_title": null, "section_ordinal": null },
      "authority": "archive_absence" },
    { "scope": "C2_am_crash channel completeness",
      "statement": "the event log ends at 00:27:20.158 while the crash buffer continues to 00:27:51.970; 87% of crashes are absent from am_crash and the channel must not be used to bound anything",
      "source": { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "findings": [
    { "id": "crash-0002", "kind": "crash_loop",
      "title": "com.android.car.cluster.osdouble — 271 IllegalArgumentException in 50.219 s (5.396/s), still crashing at capture",
      "severity": "critical", "unconstrained_severity": "critical", "confidence": "confirmed",
      "process_key": "2026-09-24T00:26:47.33+07:00/274",
      "event_time": "2026-09-24T00:27:51.970+07:00",
      "attributes": { "package": "com.android.car.cluster.osdouble", "language": "java",
                      "still_restarting_at_capture": true },
      "metrics": { "repeat_count": 271, "window_seconds": 50.219, "rate_per_second": 5.396,
                   "inter_gap_max_ms": 1851, "manager_kill_count": 273, "rescueparty_failure_count": 55 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 27742, "line_end": 27742 },
        "excerpt": "09-24 00:27:01.751  1000  1116  1116 E AndroidRuntime: FATAL EXCEPTION: main",
        "excerpt_truncated": false, "excerpt_bytes": 72, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "C1_crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": 27742, "line_end": 28819,
        "captured_at": "2026-09-24T00:27:51.970+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 0, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "last crash logcat line",
          "basis": "the 271st crash is the last line of the crash buffer, so the loop is live at capture",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "id": "crash-0003", "kind": "signal_absence",
      "title": "No native tombstone and no fatal signal appears in the main entry; the 50 real crashes are ZIP-only",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "tombstone_hits": 749, "tombstone_real": 0, "signal_hits": 656, "signal_fatal": 0 },
      "metrics": { "tombstone_files_in_zip": 50 },
      "evidence": { "archive_path": "FS/data/tombstones/tombstone_00", "kind": "file_entry",
        "locator": { "line_start": 1, "line_end": 1 },
        "excerpt": "Timestamp: 2026-05-20 04:00:14.502971422+0700\nCmdline: com.google.android.bluetooth\n…",
        "excerpt_truncated": true, "excerpt_bytes": 512, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "C4_tombstone_files",
        "archive_path": "FS/data/tombstones/tombstone_00",
        "captured_at": "2026-05-20T04:00:14.502971422+07:00", "captured_at_time_base": "tombstone_header",
        "age_seconds_at_capture": 10996814, "capture_kind": "disk_artifact", "freshness": "stale",
        "freshness_basis": { "input": "in-file Timestamp line",
          "basis": "searched and found: 749 raw 'tombstone' matches in the main entry yield 0 real tombstones; the 50 real ones are ZIP entries 126.80 days old",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "notes": [
    "CRASH_COUNT_IS_271: 274 raw 'FATAL EXCEPTION' lines decompose to 38 + 3 + 233 across three regions; the 3 in DROPBOX SYSTEM APP CRASHES are re-rendered copies of pids 5769/9384/9408 (ActivityManagerService.java:9890-9900).",
    "CRASH_COUNT_CHANNEL_IS_C1: only the crash buffer is complete. am_crash sees 39 of 271 (87% missing), dropbox is rate-limited to 6, am_proc_died has 0.314 coverage.",
    "TOMBSTONE_INDEX_IS_A_RING: tombstone_NN is next_artifact_ = (next_artifact_ + 1) % max_artifacts_ (tombstoned.cpp:196-206); tombstone_00 is not the newest file and the number carries no time meaning.",
    "NO_PID_RECYCLING_IN_LOOP: 271 crashes produced 271 distinct monotonic PIDs. The three apparent repeats were dropbox duplicates, not kernel recycling. process_key remains the identity because PID is still an observation attribute."
  ]
}
```

#### Open questions

- **No `AM_CRASH` write site.** `grep -rl AM_CRASH frameworks/base/` returns **zero** files of
  any type, and there is no `AM_CRASH` in any `.java` under `system/` or `packages/modules/`.
  The tag is declared at `EventLogTags.logtags:56` and is demonstrably emitted (39 entries in
  the archive), so the emitter lives outside the searched trees or behind a generated alias.
  **The schema therefore decodes `am_crash` slots 1–2 as `(pid, user_id)` by observation only
  and flags it in `channels[].decode_warning`; the authoritative pid comes from the
  `am_proc_died` / `am_proc_start` pairing.**
- **The `am_crash` decode trap, stated precisely.** `am_crash` is declared
  `(User, PID, …)` at `logtags:56`, but the values actually emitted are `(PID, User, …)`.
  Proven against three matched triplets read from the archive:

  ```
  am_proc_start: [0,1553,1000,com.android.car.cluster.osdouble,restart,…]   ← PID in slot 2
  am_crash:      [1553,0,com.android.car.cluster.osdouble,814267981,…]     ← PID in slot 1
  am_proc_died:  [0,1553,com.android.car.cluster.osdouble,-800,1]           ← PID in slot 2
  ```

  `am_proc_start` and `am_proc_died` match their declarations; `am_crash` does not. Decoding
  `am_crash` from the logtags file yields `user_id = 1553`, `pid = 0`.
- **Java crashes have no native record.** A Java crash is a *sequence of log lines* in the
  crash buffer — there is no per-crash file anywhere. `FATAL EXCEPTION:` from
  `RuntimeInit.java:92`, `Process: …, PID: …` from `:94-97`, the exception class and frames via
  `Clog_e` → `Log.printlns(Log.LOG_ID_CRASH, …)` at `:84-86`. The schema must carry the
  reconstruction's own confidence and the logcat line range, and must state that
  `C1_crash_buffer` is the only authoritative Java channel.
- **The logcat field order was verified against the archive, not just the format string.** A
  `threadtime` line is `MM-DD HH:MM:SS.mmm  <uid>  <pid> <tid> <L> <tag>: <message>`
  (`logprint.cpp:1552-1554`). For uid 1000 the first numeric column is `683` on **6,788** lines
  (system_server's main thread) while the second varies across 7 values (`683, 710, 717, 718,
  862, 1001, 1462` = binder threads); a 1-minute sample confirms the same for uid 1003. Had the
  order been reversed, system_server would show seven different pids.
- **Native top-frame normalisation.** `signature.top_frame` needs demangling and
  `(sym+off)` stripping. The archive's frames are already trimmed at ~120 characters, so a
  frame read from the main entry is truncated. A real run must demangle from the full ZIP
  entry; the *schema* is unaffected.
- **`exception_message` is `null` for all 271.** The archive prints `NULL` in the `am_crash`
  event and logcat carries no message line for this throw. Emitting `null` beats inventing one.
- **`native_crash_incidents[].pid_span` is `null`.** Tombstone pids (e.g. 9452) are 126-day-old
  and meaningless against this boot's pid space. A `null` is better than a number that invites
  comparison.

---

### 4.6 `memory.json`

#### Purpose

`memory.json` answers three separable questions and refuses to let one answer stand in for
another: **(1)** was the device under memory pressure at capture, and was it *reacting* (kills,
OOM); **(2)** who was holding the memory; **(3)** what *kind* of memory it was. It also carries
an explicit verdict on whether the collection itself perturbed the measurement — because in
the reference archive it did, by 207 MiB.

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | Per-channel presence/absence with reason. |
| `collection` | object | yes | The self-perturbation ledger, per [§2.1.5](#215-collection). |
| `system` | object | yes | Global pressure verdict. **Only this object may carry a `status` or `free_pct`.** |
| `swap` | object | yes | Swap and ZRAM, snapshot-indexed. |
| `processes` | array of object | yes | Per-process memory — the **cause**. |
| `categories` | array of object | yes | PSS by category — the **nature**. |
| `kernel_memory` | object | yes | Slab, pagetables, stacks, CMA. |
| `procstats` | array of object | yes | **One entry per sub-section, each with its own window.** Never merged. |
| `pressure_indicators` | object | yes | PSI plus `/proc/vmstat` OOM counters. |
| `kills` | array of object | yes | lmkd kills. **Empty is a finding, not an absence.** |
| `kill_evidence` | object | yes | Structured proof of the *absence* of kills and OOM. |
| `io` | object | yes | IOTOP, storaged, zram block stats. |

**Why four arrays rather than one.** `system`, `processes[]`, `categories[]`, and `kills[]` are
emitted by *one* `dumpsys meminfo` call at *one* instant (`AMS:12035` prints a single
`Uptime: 45590` for the whole dump), so they are mutually consistent and belong in one artifact
— but they answer different questions and aggregate over different keys. A flat per-process list
would force a choice of granularity and would drop either the verdict or the category split. A
single global object would drop attribution entirely, and the crash-loop process in this
archive is only findable from `processes[]`.

#### The self-perturbation design

**The problem.** `dumpsys meminfo` (3.525 s) and `bugreport_procdump` (5.187 s) both walk every
process's page tables. A naive schema that diffs `swap_used` across the archive reports "swap
grew 212,224 K, ~207 MiB, the device is thrashing" — a pure artifact of the collector.

There are **four** swap readings in the reference archive, not two, and the growth tracks the
dump's own page-scanning work:

| # | Channel | Archive line | Section duration | `SwapTotal` | swap used (K) | ZRAM physical (K) |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `MEMORY INFO` (`/proc/meminfo`) | 57416 | 0.002 s | 2,285,624 | **18,688** | not in `/proc/meminfo` |
| 2 | `CPU INFO` (`top` footer) | 57464 | 0.558 s | 2,285,624 | **18,176** | not printed |
| 3 | `PROCRANK` (`bugreport_procdump`) | 192650 | 0.272 s (parent **5.187 s**) | 2,285,624 | **57,088** | **24,312** |
| 4 | `DUMPSYS HIGH meminfo` | 319846 | 3.525 s | 2,285,624 | **230,912** | **62,076** |

Snapshot 1 is derived from the raw file (`SwapTotal: 2285624 kB` − `SwapFree: 2266936 kB`);
snapshots 3 and 4 are printed verbatim. The two steps that scan every page table — 8.712 s
combined — sit exactly where the number moves. `bugreport_procdump` even appears in its own PSS
table at `5350 … 68943K … bugreport_procdump` and in the `dumpsys meminfo` by-OOM-adjustment
list at `2,990K: dumpstate (pid 4149)`. **Growth = 212,224 K ≈ 207 MiB.**

Three design rules follow:

1. **Never diff snapshots implicitly.** Swap lives in `collection.swap_timeline[]`, an explicit
   ordered list, and every delta is an *edge* between two identified nodes. There is **no**
   `swap.used_kb` scalar a consumer can compare against another artifact's scalar.
2. **Make the collector's own cost a measured quantity.** `collection.page_scanning_sections[]`
   is built from the section footers dumpstate prints unconditionally. The two sections that
   caused the growth are identified by their **AOSP invocation sites**, not by pattern-matching
   numbers: `BUGREPORT_PROCDUMP` (`dumpstate.cpp:1693-1694` → `smapinfo.cpp:1244-1248` →
   `get_all_pids` + `call_smaps_of_all_processes` `:1208-1223` + `call_procrank` `:1233-1242`)
   and `DUMPSYS HIGH meminfo` (`dumpstate.cpp:1285-1287`, which AOSP gives a **60 s** timeout
   because *"since 30s is not always enough"*). `PROCRANK`'s own 0.272 s footer is *inside* its
   parent's 5.187 s and is **not** added again.
3. **A consumer that only reads `system` cannot be misled.** `system.provenance.collector_perturbed`
   is `true` for `dumpsys meminfo` and the basis string is mandatory. The same pair, in the same
   place, expresses "clean" in `cpu.json` — which is why it is in the shared convention rather
   than in one artifact.

The classification rule is in [§2.1.5](#215-collection). Applied here: **the entire 212,224 K
of growth falls between the last pre-scan reading and the post-scan reading**, and over that
interval there are zero kills and `oom_kill 0` → `assessment: "collector"`.

#### Field tables

Condensed. The owning file's exhaustive row-level table is preserved at
`tmp/aosp-team/03-memory-and-cpu.md` §4.4 for the detail that did not survive merging.

##### `collection.swap_timeline[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `proc_meminfo` / `top` / `bugreport_procdump` / `dumpsys_meminfo`. | — |
| `ordinal_in_dump` | integer | yes | Section execution order. dumpstate writes sections in statement order. | `dumpstate.cpp:1687-1709` |
| `line_start` | integer | yes | Archive locator. | measured |
| `swap_total_kb` | integer | yes | `SwapTotal`. | `android_os_Debug.cpp:391` |
| `swap_used_kb` | integer \| null | yes | `SwapTotal − SwapFree`; `null` if either absent. **Carries `swap_used_kb_derived`.** | `android_os_Debug.cpp:392` |
| `zram_physical_kb` | integer \| null | yes | `mem_used_total` summed over zram devices. | `sysmeminfo.cpp:178-206`; `AMS:12812-12819`; `smapinfo.cpp:289-291` |
| `section_duration_ms` | integer \| null | yes | The collector's own duration for this reading. | `smapinfo.cpp:1201-1203`; `dumpsys.cpp:563-567` |
| `provenance` | object | yes | | — |

##### `system`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `total_ram_kb` | integer \| null | yes | `Total RAM: 3,047,508K`. | `AMS:12699` → `MEMINFO_TOTAL` `android_os_Debug.cpp:383` |
| `memory_level` | string \| null | yes | `(status normal)` — AMS' own verdict. **Only this object may carry a status.** | `AMS:12700-12701` |
| `free_ram_kb` / `free_ram_components_kb` | integer / object | yes | `{cached_pss, cached_kernel, free}`. | `AMS:12702-12709` |
| `free_pct` | number \| null | yes | `free_ram_kb / total_ram_kb`, 4 dp. **Derived.** | derived |
| `used_ram_kb` / `used_pss_kb` / `kernel_used_kb` | integer \| null | yes | | `AMS:12803-12804` |
| `total_pss_kb` | integer \| null | yes | **Derived** = `used_pss_kb + cached_pss` = 2,007,345. **The literal string `2,007,345` does not appear in the archive**, so this field **must** carry `total_pss_kb_derivation`. | `AMS:12803` + `AMS:12703` |
| `lost_ram_kb` | integer \| null | yes | The accounting residual. | `AMS:12796-12798` |
| `tuning` | object \| null | yes | `{memory_class_mb, large_memory_class_mb, oom_adj_score_kb, restore_limit_kb, low_ram, high_end_gfx}`. | `AMS:12839-12850` |
| `gpu_kb`, `dma_buf` | integer / object | yes | | `AMS:12771-12791`, `AMS:12715-12733` |
| `provenance` | object | yes | | — |

##### `swap`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `swap_total_kb` | integer \| null | yes | | `AMS:12818-12819` |
| `swap_used_kb` | integer \| null | yes | | `AMS:12814-12816` |
| `swap_used_pct` | number \| null | yes | **Derived.** | derived |
| `zram_physical_kb` | integer \| null | yes | | `AMS:12812-12814` → `MEMINFO_ZRAM_TOTAL` `android_os_Debug.cpp:393` |
| `zram_physical_pct_of_ram` | number \| null | yes | **Derived.** | derived |
| `compression_ratio` | number \| null | yes | **Derived** = `swap_used_kb / zram_physical_kb`. Carries `compression_ratio_derived: true`. | derived |
| `zram_block_io` | object \| null | yes | Raw `/sys/block/zram0/stat` counters, **cumulative since boot**. | `dumpstate.cpp:1688,1230-1244` |
| `provenance` | object | yes | | — |

**There is deliberately no `zram_orig_size_kb` and no `zram_compressed_size_kb`.**
`SysMemInfo::MemZramDevice` deliberately skips `mm_stat` fields 1 and 2 and takes field 3,
`mem_used_total` (`sysmeminfo.cpp:178-206`). No bugreport output exposes `orig_data_size` or
`compr_data_size` anywhere, so a field so named would be unpopulatable.

`PROCRANK`'s per-process `ZSwap` is likewise **derived, not measured**: it is a
device-wide compression ratio apportioned per process
(`zram_compression_ratio = mem_zram_kb / (mem_swap_kb − mem_swap_free_kb)`,
`smapinfo.cpp:351-353`; `zswap_ = proportional_swap_ * zram_compression_ratio`,
`processrecord.cpp:114`). Verified on the archive: `1175 … 4K 4K 4K 1K
com.android.car.settings` gives `4 K × (24,312 / 57,088) = 1.70 K`, printed as `1K`. **`ZSwap`
carries no per-process truth and every consumer must be told so** via
`zswap_estimated: true`.

##### `processes[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer | yes | | `AMS:12661-12665` |
| `process_name` | string | yes | As printed, un-normalised. | `AMS:12663` |
| `user_id` | integer \| null | yes | Only printed when the entry is uid-cached. | `AMS:11905-11912` |
| `pss_total_kb` | integer | yes | | `ss[INDEX_TOTAL_PSS]` row, `AMS:11843-11872` |
| `pss_private_dirty_kb` / `pss_shared_clean_kb` / `rss_total_kb` | integer \| null | yes | | `AMS:11851`, `:12362-12370`; `smapinfo.cpp:247-249` |
| `swap_pss_kb` / `swap_uss_kb` | integer \| null | yes | `(1,128K in swap)` / `PSwap`, `USwap`. | `AMS:11909-11914`; `smapinfo.cpp:250,251` |
| `zswap_kb` | integer \| null | yes | **Always `zswap_estimated: true`.** | `smapinfo.cpp:252-254`; `processrecord.cpp:114` |
| `oom_adj` | integer \| null | yes | | `smapinfo.cpp:238-239`; `AMS:12668-12671` |
| `heap_size_kb` / `heap_alloc_kb` / `heap_free_kb` | integer \| null | yes | | `ActivityThread.dumpMemInfoTable` via `AMS:12366` |
| `source_channel` | enum | yes | `dumpsys_meminfo` / `bugreport_procdump`. **A process may appear in both** — the two dumps are at different instants. | — |
| `provenance` | object | yes | | — |

##### `categories[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `category` | string | yes | `Native`, `.apk mmap`, `Dalvik`, `Dalvik Other`, `Unknown`, … | `AMS:12579-12640` |
| `pss_total_kb` | integer | yes | | `AMS:12674-12679` |
| `swap_pss_kb` | integer \| null | yes | | `AMS:11909-11914` |
| `depth` / `parent_category` | integer / string \| null | yes | Nesting level in the printed tree. | printed indentation |
| `provenance` | object | yes | | — |

##### `procstats[]` — never merged

Three sub-sections, three different datasets, three different windows:

| Archive line | Sub-section header | `Start time` | `Aggregated over` | ZRam min/avg/max |
| --- | --- | --- | --- | --- |
| 578715 | `CURRENT STATS:` | `2026-09-24 00:26:56` | **1** | 61 / 61 / 61 MB |
| 584168 | `AGGREGATED OVER LAST 24 HOURS:` | **`2026-05-19 00:16:06`** | **4** | 61 / **110** / 125 MB |
| 590810 | `AGGREGATED OVER LAST 3 HOURS:` | **`2026-05-19 00:16:06`** | **4** | 61 / **110** / 125 MB |

> **The `LAST 3 HOURS` label is a label, not a fact.** Its `Start time` is 2026-05-19, i.e.
> **~128 days stale** in a 2026-09-24 capture. A schema that reads `ZRam: 110MB avg` out of the
> procstats dump and reports it as a device property is wrong by 128 days. `capture_kind` here
> is **`persisted_aggregate`**, and that is the value that catches it.

The header block is `ProcessStats.java:1597-1633`: `Start time:` (`:1598-1600`), `Total uptime:`
(`:1601-1605`), `Total elapsed time:` (`:1606-1609`), the completeness flags
(`(shutdown)`/`(sysprops)`/`(complete)`/`(partial)`, `:1610-1628`), and `Aggregated over: N`
(`:1632-1633`).

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `sub_section` | enum | yes | `CURRENT` / `AGGREGATED_24H` / `AGGREGATED_3H`. | `ProcessStatsService.java:1359/1374/1378` |
| `scope_label_verbatim` | string | yes | Kept **so a consumer can see the label is not a fact**. | same |
| `period_start_clock` | string | yes | | `ProcessStats.java:1598-1600` |
| `period_total_uptime` / `period_total_elapsed` | string \| null | yes | | `ProcessStats.java:1601-1609` |
| `period_complete` / `period_flags` | boolean / array | yes | The `(partial)` suffix. | `ProcessStats.java:1610-1628` |
| `aggregated_over_n` | integer \| null | yes | | `ProcessStats.java:1632-1633` |
| `backing_state_file` | string \| null | yes | `mFile=/data/system/procstats/state-v2-2026-09-24-00-26-56.bin` (archive line 584166). | `ProcessStatsService.java:1362` |
| `sys_mem_usage[]` | array | yes | `{label, sample_count, cached_*, free_*, zram_*, kernel_*, native_*}` min/avg/max. | `SysMemUsageTable.java:142-171` |
| `checkin_period` | object \| null | yes | `{start_clock, start_realtime_ms, end_realtime_ms, complete, window_ms}` parsed from the `period,…` line. | `ProcessStats.java:2149-2166`; archive line 621752 |
| `provenance` | object | yes | `capture_kind: persisted_aggregate`, `freshness: stale` for both `AGGREGATED` entries. | — |

The checkin twin carries the window in machine-readable form (archive line 621752):

```
period,2026-09-24-00-26-56,9373,39318,partial
```

⇒ start `2026-09-24 00:26:56`, `mTimePeriodStartRealtime = 9373 ms`, end `elapsedRealtime =
39318 ms`, **partial** (`FLAG_SHUTDOWN|FLAG_SYSPROPS|FLAG_COMPLETE` all clear). Window length =
**29,945 ms**.

##### `pressure_indicators`, `kills[]`, `kill_evidence`, `io`, `kernel_memory`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `pressure_indicators` | `psi_memory` / `psi_io` | object \| null | yes | `{some:{avg10,avg60,avg300,total_us}, full:{…}}`. `avg*` are exponentially weighted moving averages; `total` is µs. | `dumpstate.cpp:1979-1981`; `system/memory/lmkd/libpsi/psi.cpp` |
| | `vmstat_oom_kill` | integer \| null | yes | `oom_kill 0`. | `dumpstate.cpp:1698` |
| | `vmstat_pswpin` / `vmstat_pswpout` | integer \| null | yes | Cumulative since boot. | `dumpstate.cpp:1698` |
| `kills[]` | `timestamp`, `process_name`, `pid`, `uid`, `oom_score_adj`, `rss_kb`, `swap_kb`, `reason_code`, `reason_text` | — | yes | **Empty is a valid, meaningful result.** `reason_text` comes from lmkd's own `ALOGI`, not from the event log. | `lmkd.cpp:2481-2488`; `statslog.h:57-70` |
| | `killinfo_event` | object \| null | yes | The 41-field `killinfo` payload when the events buffer retains it. | `lmkd.cpp:2136-2180`; `event.logtags:39` |
| `kill_evidence` | `lmkd_alive` / `lmkd_using_psi` | boolean \| null | yes | | `lmkd.cpp:17`, `:3597` |
| | `vmstat_oom_kill` / `am_kill_count` | integer | yes | Both `0`. | `dumpstate.cpp:1698`; `EventLogTags.logtags:34` |
| | `am_low_memory_count` / `am_low_memory_value` / `am_low_memory_meaning` | integer / string | yes | `1` — and this is **not** memory exhaustion. `am_low_memory` fires when `!haveBackgroundProcessLOSP()`, i.e. when the *lowest* oom_adj band was already emptied. A `1` with `status normal` is a lowest-priority-caches-exhausted signal, not a kill trigger. The meaning is carried as mandatory prose so the interpretation is visible rather than implicit. | `EventLogTags.logtags:31`; writer `AppProfiler.java:1607-1624` |
| | `am_meminfo_count` / `am_pss_count` | integer | yes | Both `0` — **structurally**. | `EventLogTags.logtags:67,69`; `am_meminfo` writer `AMS:12691` |
| | `am_meminfo_absence_reason` | string | yes | **Required** when `am_meminfo_count == 0`. | — |
| `io` | `iotop` | object \| null | yes | `{window_ms, row_granularity, sort_column, max_io_delay_pct, max_swap_delay_pct, max_sched_delay_pct, max_mem_delay_pct, resolution_floor_note}`. | `iotop.cpp:36-39,234,247-265` |
| | `storaged` | object \| null | yes | **Cumulative since process start — no window.** | `dumpstate.cpp:1801` |
| | `zram_block_io` | object \| null | yes | Same counters as `swap.zram_block_io`. | `dumpstate.cpp:1688` |
| `kernel_memory` | `slab` / `pagetables` / `stacks` / `cma` | object \| null | yes | From `ZONEINFO` / `BUDDYINFO` / `SLAB INFO`. | `dumpstate.cpp:1700-1704` |

#### Why `am_meminfo` and `am_pss` are structurally zero

This matters because **"0 events" must be distinguishable from "channel broken"**. dumpstate
captures `EVENT LOG` at `dumpstate.cpp:1028` (archive line 53710), **before** `DUMPSYS HIGH
meminfo` at line 319846. `am_meminfo` is written *by the meminfo dump itself* (`AMS:12691`,
inside the same block at `:12686-12695`). So in **every** bugreport whose event log is captured
before the meminfo dump — i.e. every bugreport 2.0 archive — `am_meminfo` is expected to be `0`.
Measured alongside `am_proc_start 121`, `am_proc_bound 121`, `am_proc_died 40`,
`am_uid_running 68`, `am_uid_active 64`.

**A zero here is not evidence of a working memory-telemetry path**, and the schema says so via
`am_meminfo_absence_reason`.

#### Provenance and freshness

Three distinct clock families, each with its own `capture_kind`:

| Source | `capture_kind` | `captured_at_time_base` | Freshness on this archive |
| --- | --- | --- | --- |
| `dumpsys meminfo` summary block | `point_in_time_snapshot` | `dumpsys_section_snapshot` (footer `ending at: 2026-09-24 00:27:36`) | `in_window` |
| `PROCRANK` rows and footer | `point_in_time_snapshot` | `dumpsys_section_snapshot` | `undated` — raw smaps print no clock |
| `MEMORY INFO` (`/proc/meminfo`) | `point_in_time_snapshot` | `undetermined` | `undated` — **deliberately unstamped**; `DumpstateInternal.cpp:143-145` suppresses the mtime for `/proc`, `/sys`, and `/d` paths |
| `dumpsys procstats` `CURRENT` | `background_sampler` | `dumpsys_section_snapshot` | `in_window` |
| `dumpsys procstats` `AGGREGATED_*` | **`persisted_aggregate`** | `dumpsys_section_snapshot` | **`stale`** — 128 days |
| `/proc/vmstat` counters | `command_output` | `undetermined` | `undated` — cumulative since boot, but no clock printed |
| `IOTOP` | `window_delta` → **`command_output`** | `undetermined` | `undated` |
| lmkd kill log line | `live_log` | `logcat_line` | `in_window` |

**The `undated` rows are the honest ones.** A genuine instantaneous reading with **no printed
clock at all** is not `stale`; it is unlabelled. Collapsing `undated` into `stale` would make a
whole class of legitimate evidence look like historical residue and train consumers to ignore
the field. Collapsing it into `now` is the exact bug this convention exists to prevent. Where a
snapshot can only be ordered, the schema records `ordinal_in_dump` (dumpstate's statement order
at `dumpstate.cpp:1687-1709`) rather than inventing a timestamp.

**IOTOP's `0.00%` columns are a resolution floor, not a zero.**
`TimeToTgidPercent` (`iotop.cpp:36-39`) computes `percent = ns / stats.threads() / (time * NSEC_PER_SEC / 100.0f)`
and clamps at 99.99, printed as `%5.2f%%` (`:255-265`). With `delay = 1` (default,
`iotop.cpp:104`), `0.00%` means **under 0.1 ms per thread over 1 s** — *not* "no I/O". The same
rows carry non-zero throughput:

```
   320 kworker/u8:4       4904      0   4904      0      0  0.00%  0.00%  2.36%  0.00%  2.36%
  2630 Blocking Thread       0   2308   2308      0     99  0.00%  0.00% 24.65%  0.00% 24.65%
    54 kswapd0               0      0      0      0      0  0.00%  0.00%  0.72%  0.00%  0.72%
```

The `24.65%` on `Blocking Thread` is **run-queue delay, not I/O** — column 3 of the
`delayed on` group is `sched` (`iotop.cpp:234`). This section answers "was anything *waiting*".
Its own duration footer reads `1.052s` while the measured window is 1 s; the extra 52 ms is
process setup and the `first`-iteration skip (`iotop.cpp:226-273`).

#### Worked example

Abbreviated; every value was measured on the reference archive.

```jsonc
{
  "schema_version": 1,
  "artifact": "memory",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "page_scanning_sections": [
      { "name": "BUGREPORT_PROCDUMP", "line_start": 191000, "line_end": 192895, "duration_ms": 5187,
        "aosp_source": "dumpstate.cpp:1693-1694 -> smapinfo.cpp:1244-1248" },
      { "name": "DUMPSYS HIGH meminfo", "line_start": 319846, "line_end": 326112, "duration_ms": 3525,
        "aosp_source": "dumpstate.cpp:1285-1287 (60s timeout for meminfo)" }
    ],
    "page_scanning_total_ms": 8712,
    "swap_timeline": [
      { "channel": "proc_meminfo", "ordinal_in_dump": 1, "line_start": 57416,
        "swap_total_kb": 2285624, "swap_used_kb": 18688, "swap_used_kb_derived": true,
        "zram_physical_kb": null, "section_duration_ms": 2,
        "provenance": { "evidence_origin": "proc_meminfo", "section_title": "MEMORY INFO", "section_ordinal": 0,
          "line_start": 57416, "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none",
            "basis": "raw /proc/meminfo carries no clock (DumpstateInternal.cpp:143-145 suppresses the mtime for /proc paths); positioned only by dump statement order (dumpstate.cpp:1689)",
            "threshold_seconds": 3600 },
          "collector_perturbed": false,
          "collector_perturbed_basis": "read of a 0.002 s procfs file" } },
      { "channel": "top", "ordinal_in_dump": 2, "line_start": 57471,
        "swap_total_kb": 2285624, "swap_used_kb": 18176, "swap_used_kb_derived": false,
        "zram_physical_kb": null, "section_duration_ms": 558,
        "provenance": { "evidence_origin": "cpu_info_top", "section_title": "CPU INFO", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "top prints no clock", "threshold_seconds": 3600 },
          "collector_perturbed": true,
          "collector_perturbed_basis": "top itself is 0.558s of mid-dump work; column semantics unverified (top is not in this source drop)" } },
      { "channel": "bugreport_procdump", "ordinal_in_dump": 3, "line_start": 192829,
        "swap_total_kb": 2285624, "swap_used_kb": 57088, "swap_used_kb_derived": false,
        "zram_physical_kb": 24312, "section_duration_ms": 272,
        "provenance": { "evidence_origin": "procrank", "section_title": "PROCRANK", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "PROCRANK prints no clock",
            "threshold_seconds": 3600 },
          "collector_perturbed": true,
          "collector_perturbed_basis": "read at the end of a 5.187 s full page-table scan" } },
      { "channel": "dumpsys_meminfo", "ordinal_in_dump": 4, "line_start": 326109,
        "swap_total_kb": 2285624, "swap_used_kb": 230912, "swap_used_kb_derived": false,
        "zram_physical_kb": 62076, "section_duration_ms": 3525,
        "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "DUMP OF SERVICE HIGH meminfo:",
          "section_ordinal": 0, "line_start": 319846,
          "captured_at": "2026-09-24T00:27:36+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
          "freshness_basis": { "input": "dumpsys footer 'ending at: 2026-09-24 00:27:36'",
            "basis": "inside capture.log_window", "threshold_seconds": 3600 },
          "collector_perturbed": true,
          "collector_perturbed_basis": "taken inside a 3.525 s all-process PSS scan that AOSP itself budgets 60 s for (dumpstate.cpp:1285-1287)" } }
    ],
    "swap_growth_kb": 212224,
    "self_inflicted_growth_kb": 212224,
    "assessment": "collector",
    "assessment_basis": "swap used rose 18688K -> 230912K (+212224K) entirely between the last pre-scan reading (top, 18176K) and the post-meminfo reading (230912K); the two page-scanning sections in that interval totalled 8712ms of the dump's own work. am_kill_count=0, /proc/vmstat oom_kill=0, dumpsys meminfo status 'normal'. Classified as collector-induced paging, not device memory pressure."
  },

  "system": {
    "total_ram_kb": 3047508,
    "memory_level": "normal",
    "free_ram_kb": 1173179,
    "free_ram_components_kb": { "cached_pss": 409539, "cached_kernel": 491268, "free": 272372 },
    "free_pct": 0.3850,
    "used_ram_kb": 1983938,
    "used_pss_kb": 1597806,
    "kernel_used_kb": 386132,
    "total_pss_kb": 2007345,
    "total_pss_kb_derivation": "used_pss_kb (AMS:12803) + free_ram_components_kb.cached_pss (AMS:12703) = 1597806 + 409539; the literal '2,007,345' does not appear in the archive",
    "lost_ram_kb": 71072,
    "lost_ram_pct": 0.0233,
    "tuning": { "memory_class_mb": 192, "large_memory_class_mb": 576, "oom_adj_score_kb": 322560,
                "restore_limit_kb": 107520, "low_ram": null, "high_end_gfx": true },
    "gpu_kb": 0,
    "dma_buf": { "total_kb": 0, "mapped_kb": 0, "unmapped_kb": 0, "heaps_kb": 0, "heaps_pool_kb": 0 },
    "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "DUMP OF SERVICE HIGH meminfo:",
      "section_ordinal": 0, "line_start": 319846, "line_end": 326112,
      "captured_at": "2026-09-24T00:27:36+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "dumpsys footer",
        "basis": "body prints 'Uptime: 45590 Realtime: 45590' (AMS:12036); the reading cannot outlive the boot and falls inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "3.525 s of the dump's own all-process PSS scan immediately precedes the summary block" }
  },

  "swap": {
    "swap_total_kb": 2285624, "swap_used_kb": 230912, "swap_used_pct": 0.1010,
    "zram_physical_kb": 62076, "zram_physical_pct_of_ram": 0.0204,
    "compression_ratio": 3.719, "compression_ratio_derived": true,
    "zram_block_io": { "read_ios": 916, "read_sectors": 7328, "write_ios": 5314,
                       "write_sectors": 42512, "io_ticks": 344,
                       "window": "cumulative since boot", "line_start": 57314 },
    "provenance": { "captured_at": "2026-09-24T00:27:36+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": true, "collector_perturbed_basis": "same dumpsys meminfo pass" }
  },

  "processes": [
    { "pid": 2021, "process_name": "com.google.android.carassistant:search", "user_id": 10,
      "pss_total_kb": 212862, "swap_pss_kb": 1128, "zswap_kb": 0, "zswap_estimated": true,
      "rss_total_kb": 401072, "source_channel": "dumpsys_meminfo", "line_start": 325705 },
    { "pid": 2418, "process_name": "com.google.android.tts", "user_id": 10,
      "pss_total_kb": 173260, "swap_pss_kb": 5539, "rss_total_kb": 314704,
      "source_channel": "dumpsys_meminfo", "line_start": 325706 },
    { "pid": 683, "process_name": "system", "user_id": null,
      "pss_total_kb": 136998, "swap_pss_kb": 9651, "rss_total_kb": 275732,
      "source_channel": "dumpsys_meminfo", "line_start": 325707 },
    { "pid": 5350, "process_name": "bugreport_procdump", "user_id": null,
      "pss_total_kb": 68943, "swap_pss_kb": 0, "rss_total_kb": 71900,
      "source_channel": "bugreport_procdump", "line_start": 192650,
      "note": "the collector itself" },
    { "pid": 4149, "process_name": "dumpstate", "user_id": null,
      "pss_total_kb": 2990, "swap_pss_kb": 0, "source_channel": "dumpsys_meminfo", "line_start": 325903 }
  ],

  "categories": [
    { "category": "Native",      "depth": 0, "parent_category": null,        "pss_total_kb": 577687, "swap_pss_kb": 100386 },
    { "category": ".apk mmap",   "depth": 0, "parent_category": null,        "pss_total_kb": 389207, "swap_pss_kb":   3300 },
    { "category": "Dalvik",      "depth": 0, "parent_category": null,        "pss_total_kb": 212420, "swap_pss_kb":  14154 },
    { "category": ".Heap",       "depth": 1, "parent_category": "Dalvik",   "pss_total_kb": 185027, "swap_pss_kb":   5608 },
    { "category": "Dalvik Other","depth": 0, "parent_category": null,        "pss_total_kb": 113592, "swap_pss_kb":  14375 },
    { "category": "Unknown",     "depth": 0, "parent_category": null,        "pss_total_kb": 104125, "swap_pss_kb":  41441 },
    { "category": ".so mmap",    "depth": 0, "parent_category": null,        "pss_total_kb":  84097, "swap_pss_kb":  34991 }
  ],

  "procstats": [
    { "sub_section": "CURRENT", "scope_label_verbatim": "CURRENT STATS:",
      "period_start_clock": "2026-09-24 00:26:56", "period_total_uptime": "+51s928ms",
      "period_complete": false, "period_flags": ["partial", "swapped-out-pss"], "aggregated_over_n": 1,
      "checkin_period": { "start_clock": "2026-09-24-00-26-56", "start_realtime_ms": 9373,
                          "end_realtime_ms": 39318, "complete": false, "window_ms": 29945 },
      "provenance": { "captured_at": "2026-09-24T00:26:56+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "Start time line", "basis": "1 sample over 29.945 s, inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "sample taken by the AMS background collector, not by the dump" } },

    { "sub_section": "AGGREGATED_24H", "scope_label_verbatim": "AGGREGATED OVER LAST 24 HOURS:",
      "period_start_clock": "2026-05-19 00:16:06", "aggregated_over_n": 4,
      "period_complete": false, "period_flags": ["partial"],
      "backing_state_file": "/data/system/procstats/state-v2-2026-09-24-00-26-56.bin",
      "provenance": { "captured_at": "2026-05-19T00:16:06+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "Start time line",
          "basis": "period_start_clock 2026-05-19 is 128 days before the 2026-09-24 capture; read from on-disk state via ProcessStatsService.java:862-866 (getStatsOverTime). The '24 HOURS' label is a query parameter, not a fact about the data.",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "predates the dump by months" } },

    { "sub_section": "AGGREGATED_3H", "scope_label_verbatim": "AGGREGATED OVER LAST 3 HOURS:",
      "period_start_clock": "2026-05-19 00:16:06", "aggregated_over_n": 4,
      "period_complete": false, "period_flags": ["partial"],
      "provenance": { "captured_at": "2026-05-19T00:16:06+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "Start time line",
          "basis": "identical start time to the 24H block; the '3 HOURS' label is not supported by the data",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "predates the dump by months" } }
  ],

  "pressure_indicators": {
    "psi_memory": { "some": { "avg10": 3.54, "avg60": 1.14, "avg300": 0.26, "total_us": 964143 },
                    "full": { "avg10": 0.97, "avg60": 0.29, "avg300": 0.06, "total_us": 275904 } },
    "psi_io":     { "some": { "avg10": 12.36, "avg60": 5.82, "avg300": 1.50, "total_us": 4881350 },
                    "full": { "avg10": 2.84, "avg60": 1.82, "avg300": 0.50, "total_us": 1794988 } },
    "vmstat_oom_kill": 0,
    "vmstat_pswpin": null, "vmstat_pswpout": null,
    "vmstat_absence_reason": "the exact 'pswpin'/'pswpout' key names in the VIRTUAL MEMORY STATS block were not confirmed in this pass",
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "age_seconds_at_capture": null, "freshness": "undated",
      "freshness_basis": { "input": "none",
        "basis": "kernel cumulative counters carry no clock; they cannot outlive the boot",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "kernel counters, not a dump-scanned value" }
  },

  "kills": [],
  "kill_evidence": {
    "lmkd_alive": true, "lmkd_using_psi": true, "lmkd_kill_log_matches": 0,
    "vmstat_oom_kill": 0, "am_kill_count": 0,
    "am_low_memory_count": 1, "am_low_memory_value": 2,
    "am_low_memory_meaning": "emitted at 00:27:00.045 from AppProfiler.java:1624 with value lruSize=2. Fires when !haveBackgroundProcessLOSP() (AppProfiler.java:1611-1612), i.e. the lowest oom_adj band was already emptied. NOT memory exhaustion: memory_level is 'normal' and lmkd killed nothing.",
    "am_meminfo_count": 0, "am_pss_count": 0,
    "am_meminfo_absence_reason": "structural: EVENT LOG is captured at dumpstate.cpp:1028 (archive line 53710) before the meminfo dump at line 319846, and am_meminfo is written by that meminfo dump itself (ActivityManagerService.java:12691).",
    "provenance": { "capture_kind": "live_log", "captured_at": "2026-09-24T00:27:00.045+07:00",
      "captured_at_time_base": "logcat_line", "freshness": "in_window", "collector_perturbed": false }
  },

  "io": {
    "iotop": { "window_ms": 1000, "line_start": 56843, "section_duration_ms": 1052,
      "row_granularity": "thread", "sort_column": "total",
      "max_io_delay_pct": 0.00, "max_swap_delay_pct": 0.00,
      "max_sched_delay_pct": 24.65, "max_mem_delay_pct": 0.00,
      "resolution_floor_note": "io/swap/mem '0.00%' is a %.2f resolution floor (iotop.cpp:36-39, 255-265) meaning <0.1ms per thread over 1s, NOT zero I/O. Rows carry up to 4904 KiB/s read." },
    "storaged": { "line_start": 348900, "basis": "cumulative since process start",
                  "top_reader": "shared:android.uid.system 1134249 fg_rchar / 13627392 fg_rbytes" },
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "freshness": "undated",
      "freshness_basis": { "input": "none", "basis": "iotop prints no clock", "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "iotop runs mid-dump and its own threads appear in the table" }
  },

  "counts": {
    "am_kill_hits": {
      "pattern": "am_kill", "pattern_kind": "word_boundary", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "suppressed_noise_note": null,
      "coverage": { "window_start": "2026-09-24T00:26:59.872+07:00", "window_end": "2026-09-24T00:27:20.176+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 0.314 },
      "regions": []
    }
  },

  "findings": [
    { "id": "mem-001", "kind": "no_pressure",
      "title": "No memory pressure at capture: status normal, 38.50% free RAM, zero lmkd kills, zero OOM kills, PSI memory full avg10 0.97%",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "memory_level": "normal" },
      "metrics": { "free_pct": 0.3850, "psi_memory_full_avg10": 0.97, "vmstat_oom_kill": 0, "lmkd_kill_count": 0 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 326101, "line_end": 326101 },
        "excerpt": "Total RAM: 3,047,508K (status normal)",
        "excerpt_truncated": false, "excerpt_bytes": 37, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "DUMP OF SERVICE HIGH meminfo:",
        "section_ordinal": 0, "line_start": 326101, "line_end": 326110,
        "captured_at": "2026-09-24T00:27:36+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "dumpsys footer", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "see system.provenance" } },

    { "id": "mem-002", "kind": "collector_artifact",
      "title": "Swap grew +212224K (18,688K -> 230,912K) during the capture, attributed to the collector",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "assessment": "collector" },
      "metrics": { "swap_growth_kb": 212224, "page_scanning_total_ms": 8712, "vmstat_oom_kill": 0 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 326109, "line_end": 326109 },
        "excerpt": "     ZRAM:    62,076K physical used for   230,912K in swap (2,285,624K total swap)",
        "excerpt_truncated": false, "excerpt_bytes": 79, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "collection_swap_timeline", "archive_path": "<main_entry>",
        "section_title": "DUMP OF SERVICE HIGH meminfo:", "section_ordinal": 0,
        "line_start": 57416, "line_end": 326109,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "a delta between two snapshots; neither endpoint carries a clock, so the EDGE is undated even though its endpoints are ordered",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "collection.page_scanning_sections" } },

    { "id": "mem-003", "kind": "stale_aggregate",
      "title": "dumpsys procstats AGGREGATED 3H and 24H blocks both start 2026-05-19 00:16:06 — 128 days stale",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "capture_kind": "persisted_aggregate" },
      "metrics": { "age_seconds": 11055606, "aggregated_over_n": 4 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 584169, "line_end": 584172 },
        "excerpt": "          Start time: 2026-05-19 00:16:06\n        Total uptime: +1h30m52s599ms\n  Total elapsed time: +1h30m52s599ms (partial) libart.so\n     Aggregated over: 4",
        "excerpt_truncated": false, "excerpt_bytes": 163, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_procstats", "section_title": "DUMP OF SERVICE HIGH procstats:",
        "section_ordinal": 0, "line_start": 584168, "line_end": 590810,
        "captured_at": "2026-05-19T00:16:06+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 11055606, "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "Start time line",
          "basis": "128 days before capture; capture_kind persisted_aggregate is the deciding input, not the wall clock alone",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "predates the dump" } },

    { "id": "mem-004", "kind": "largest_allocator",
      "title": "Top PSS holder is com.google.android.carassistant:search at 212,862K; category leader is Native at 577,687K — a code/image-heavy, not heap-heavy, profile",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": "2026-09-24T00:26:47.33+07:00/61", "event_time": null,
      "attributes": { "package_name": "com.google.android.carassistant:search" },
      "metrics": { "pss_total_kb": 212862, "total_pss_kb": 2007345, "native_category_kb": 577687 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 325705, "line_end": 325705 },
        "excerpt": "  212,862K: com.google.android.carassistant:search (pid 2021)",
        "excerpt_truncated": false, "excerpt_bytes": 61, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "DUMP OF SERVICE HIGH meminfo:",
        "section_ordinal": 0, "line_start": 325705, "line_end": 326040,
        "captured_at": "2026-09-24T00:27:36+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "dumpsys footer", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "3.525 s all-process scan" } }
  ],

  "notes": [
    "SWAP_GROWTH_IS_COLLECTOR_INDUCED: 212,224 K of swap growth (207 MiB) falls entirely between the last pre-page-scan reading and the post-scan reading, with 8,712 ms of the dump's own page-table walking in between and no kill or OOM. This is NOT a device memory finding.",
    "PROCSTATS_LABELS_ARE_QUERY_PARAMETERS: 'AGGREGATED OVER LAST 3 HOURS' and 'LAST 24 HOURS' both resolve to Start time 2026-05-19 00:16:06 (ProcessStatsService.java:862-866). Their ZRam avg 110 MB and Native avg 766 MB describe May, not this boot.",
    "AM_LOW_MEMORY_IS_NOT_EXHAUSTION: it fires when !haveBackgroundProcessLOSP() (AppProfiler.java:1611-1612), i.e. when the LOWEST oom_adj band is empty. A count of 1 with status 'normal' is a lowest-priority-caches-exhausted signal.",
    "AM_MEMINFO_AND_AM_PSS_ARE_STRUCTURALLY_ZERO: the event log is captured before the meminfo dump that writes them.",
    "ZSWAP_IS_APPORTIONED_NOT_MEASURED: PROCRANK ZSwap = proportional_swap_ * (mem_zram_kb / swap_used_kb) (processrecord.cpp:114, smapinfo.cpp:351-353). Every zswap_kb carries zswap_estimated: true.",
    "NO_ZRAM_COMPRESSED_SIZE: libmeminfo skips mm_stat fields 1 and 2 (sysmeminfo.cpp:184) and takes mem_used_total. No bugreport output exposes orig_data_size or compr_data_size, so those fields do not exist in this schema."
  ]
}
```

#### Open questions

- **`top` and `procrank` binaries are not in this source drop** — no `system/core/toolbox/top.cpp`,
  no `system/core/toybox`, no `system/extras/procrank`. For `top` only the *invocation* is
  citable (`dumpstate.cpp:1690-1691`), so every `top`-derived field's semantics are marked
  **unverified**: whether `%CPU` is scaled by thread count (`-H` is passed), the exact meaning
  of `0%host`, and the denominator behind `400%cpu`. Column names are read off the output
  header, not off a format string.
- **ZRAM `orig_data_size` / `compr_data_size` are not exposed anywhere in a bugreport.**
  Deliberately unpopulatable, so no field was created. If a build exposes the raw `mm_stat` in
  a dumpsys section not found here, this should be revisited.
- **`/proc/uid_stat` does not exist in Android 15.** No producer or consumer in this tree;
  `ProcessCpuTracker` derives per-UID CPU by summing per-process deltas from `/proc/<pid>/stat`
  (`:57-78, :353`). There is therefore **no per-UID CPU channel** in `cpu.json` either — see
  [§4.7](#47-cpujson) open questions.
- **`am_kill`'s `Reason` string vocabulary is unresolved.** `EventLogTags.logtags:34` declares
  `Reason` as a string but no enum or writer populating it was found in this drop. The
  `killinfo` event's `enum kill_reasons` (`statslog.h:57-70`) is a *different*, numeric field.
  `kills[].reason_text` is therefore sourced from lmkd's own `ALOGI` (`lmkd.cpp:2485`), not from
  the event log.
- **`am_pss` has no located writer.** `EventLogTags.logtags:69` declares the tag; the
  `am_meminfo` writer (`AMS:12691`) was found but not `writeAmPs`. The tag count is 0 and the
  event log is captured before the memory dumps, so the schema does not change — but the
  producer cannot be cited.
- **`ProcessStats` internal bucket boundaries are unlocated.** `dumpAggregatedStats` buckets by
  *file age* (`aggregateHours*3600*1000 − COMMIT_PERIOD/2`, `ProcessStatsService.java:865-866`);
  the per-PSS-bucket windows (1/3/15 min) were not found in this tree, so
  `procstats[].aggregation_window` is built from the printed `Start time` / `Total elapsed time`
  / `Aggregated over` fields rather than from a declared bucket width.
- **`am_low_memory` semantics want a second opinion.** The reading encoded here is "the
  lowest-priority band is empty", not "memory exhaustion", because the archive shows
  `am_low_memory: 2` with `status normal` and zero lmkd kills. But the tag is *named*
  `am_low_memory` and a reader could reasonably read it the other way. The interpretation is
  therefore carried as mandatory prose (`am_low_memory_meaning`) rather than left implicit.
- **Which of two `/proc/meminfo` reads fired.** `dumpstate.cpp:1689` and `dumpstate.cpp:1951`
  both read `/proc/meminfo`; the archive contains one `MEMORY INFO` block. The
  `ordinal_in_dump` values above assume `:1689` fired, which is consistent with `:1689`
  appearing immediately before `CPU INFO` in the archive. Not settled.
- **`busy_pct`-style derived percentages need unit tests.** AOSP publishes only raw
  milliseconds (`CpuInfoReader.java:859-880`) and never a busy percentage, so any derived
  percentage needs a unit test against a hand-summed row. This was learned the hard way: a
  first pass mis-added the buckets and shipped wrong values.

---

### 4.7 `cpu.json`

#### Purpose

`cpu.json` answers: **how much CPU was used, by whom, over what window** — and, separately,
**was anything blocked**, which is a different failure mode with different evidence and must
not be folded into a single "CPU problem" number.

#### Top-level keys

The thirteen envelope keys, plus:

| Key | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capture_ref` | object | yes | Points at `system.json#/capture`. |
| `channels` | array of object | yes | Per-channel presence/absence. |
| `collection` | object | yes | The shared ledger, **plus** `cpu_clean_channels` / `cpu_perturbed_channels` / `mechanism`. |
| `utilization` | array of object | yes | Global CPU ratios, **one entry per measurement, each with its own window.** |
| `per_cpu_core` | array of object | yes | Per-core cumulative milliseconds, including **steal**. |
| `processes` | array of object | yes | Per-process CPU over `utilization`'s window. |
| `contention` | object | yes | Lock/blocking evidence — **never merged into `utilization`.** |
| `load` | object | yes | Load averages **with uptime context**. |
| `stall_signals` | object | yes | PSI, IOTOP, process churn. |
| `graphics` | object \| null | yes | Per-process frame stats. |
| `power_history` | object \| null | yes | batterystats / powerstats — **quarantined, always stale.** |

**Four disjoint arrays, and the utilization/contention split is the substantive decision.** In
this archive the two point in opposite directions: `61% TOTAL` with `35% system_server` looks
like a load problem, while `25% irq` with `steal = 0` on all four cores says the guest is being
*interrupted*, not busy. A single score would say "CPU problem". Two arrays say
"compute-bound in `system_server`, host interrupt injection, and no lock pathology worth
reporting".

The other two separations are about **units**, not judgement:

- `utilization` is a **windowed ratio** whose window is arbitrary and must travel with it.
- `per_cpu_core` is **cumulative milliseconds since boot**, read at `mLastReadUptimeMillis =
  11192`. It is *not* a percentage and *not* the same window. Merging them produces a number
  that means nothing.
- `processes[]` shares `utilization`'s window exactly (same `ProcessCpuTracker` delta), so it
  is attributable and gets a share field.
- `contention` is measured in **blocking events**, sampled at an unknown rate, over the **log
  window** — not the CPU window.

#### The `dumpsys cpuinfo` window — the finding that makes the CPU side provably clean

`AppProfiler.updateCpuStatsNow()` re-samples `/proc/stat` **only if ≥ 5 s have elapsed** since
the last sample (`AppProfiler.java:142`, `MONITOR_CPU_MIN_TIME = 5 * 1000`;
gate at `:2156-2160`), driven by a thread that has run since `system_server` started
(`:2358`, `new ProcessCpuThread("CpuTracker")`, loop at `:2285-2313`).
`CpuBinder.dumpCritical` merely prints **the last two samples it already has** — it does not
take a fresh one (`:2332-2335`).

The header carries the window explicitly (`ProcessCpuTracker.java:854-889`), and archive lines
3685–3686 read:

```
CPU usage from 22612ms to 20840ms ago (2026-09-24 00:26:56.813 to 2026-09-24 00:26:58.585):
```

- window length = **1.772 s**
- window ended **20.415 s before** the dump printed it
  (`--------- 0.023s was the duration of dumpsys cpuinfo, ending at: 2026-09-24 00:27:19 ------`,
  line 3710)
- window ended **18.415 s before dumpstate began** (`== dumpstate: 2026-09-24 00:27:17`, line 2)
- `cpuinfo` is `DUMP_FLAG_PRIORITY_CRITICAL` (`AppProfiler.java:2348`), so it is the **first**
  dumpsys block in the whole archive

**The CPU numbers cannot be inflated by the collection, because the sample was taken and frozen
before dumpstate started.** `collector_perturbed: false` with the basis naming the 18.415 s gap.
`capture_kind: background_sampler` is what records *why*.

`top` (`dumpstate.cpp:1690`) is the opposite: it runs mid-dump, and its own `Swap:` line already
shows the collector's effect (18,176 K used versus 18,688 K at the immediately preceding
`/proc/meminfo` read).

#### `steal` is structurally unavailable in the global ratio

`dumpsys cpuinfo`'s `TOTAL` line is
`mRelUserTime + mRelSystemTime + mRelIoWaitTime + mRelIrqTime + mRelSoftIrqTime + mRelIdleTime`
(`ProcessCpuTracker.java:890-892` for the denominator, `:914` for the row) — it **excludes
steal**, because the aggregate `/proc/stat` read uses `SYSTEM_CPU_FORMAT` (`:126-134`) which
stops at `softirq`. Steal is available only per-CPU, via the `cpu_monitor` binder service
(`CpuInfoReader.java:69`, `:859-880`). **That asymmetry is why `cpu.json` needs both channels**,
and it is why `utilization[].pct_steal` is `null` for `dumpsys_cpuinfo` with a stated reason
rather than `0`.

#### Contention: two distinct producers, both real, neither a count

`grep -c "Monitor contention with tid"` → **0**.
`grep -ci "Long monitor contention"` → **33**.
They are different producers and both are cited.

**C1 — ART Java monitor.** Line shape (`art/runtime/monitor.cc:592-598` + `402-420`):

```
Long monitor contention with owner <owner_name> (<owner_tid>) at <owner_method>(<file>:<line>) waiters=<N> in <waiter_method> for <PrettyDuration>
```

`PrettyDuration` (`art/libartbase/base/time_utils.cc:53-75`) chooses ns/µs/ms/**s** and prints
at second granularity with fractions — hence `1.780s`, `181ms`, `12.3us`.

Threshold: `kLongWaitMs = 100 * kDebugThresholdFudgeFactor` (`monitor.cc:54`) and
`kDebugThresholdFudgeFactor = kIsDebugBuild ? 10 : 1` (`monitor.cc:53`) — so **on this userdebug
build the floor is 1000 ms**. That is a real schema caveat: *a 200 ms Java-monitor stall produces
no line here, and its absence is not evidence of health.*

Measured distribution of the 33 (all wait strings): max **1.780 s**, min 105 ms; **6 events
≥ 1 s** — 4 in `com.google.android.gms` (pid 2488) and 2 in `com.google.android.gms.persistent`
(pids 1972, 2433), all inside the `com.google.android.gms@254232109@25.42.32` dex, owner method
`axte.h(java.lang.String)`, with `waiters=` running `0,1,2,3` on four consecutive lines 6 ms
apart. A monotonic waiter queue on one monitor is a `MessageQueue.next()` shape.
**Zero of the 33 are in `system_server`** — the `system_server` contention in this archive is
the *other* kind, C13.

**C13 — `dvm_lock_sample`.** `Monitor::LogContentionEvent` is emitted only with probability equal
to how far the wait exceeded the configured threshold (`monitor.cc:529-536`):

```cpp
if (wait_ms >= lock_profiling_threshold_) { sample_percent = 100; }
else                                        { sample_percent = 100 * wait_ms / lock_profiling_threshold_; }
if (sample_percent != 0 && (static_cast<uint32_t>(rand() % 100) < sample_percent)) { … LogContentionEvent(…) }
```

Field order (`monitor_android.cc:35-108`, tag `20003` at `:31`):

```
1 process_name  2 is_sensitive  3 thread_name  4 wait_ms
5 waiter_file   6 waiter_line   7 waiter_method
8 owner_file    9 owner_line    10 owner_method  11 sample_percent
```

Archive line 53975, verbatim and complete:

```
09-24 00:27:01.455  1000   683   683 I dvm_lock_sample: [system_server,1,main,497,AudioManager.java,7748,int android.media.AudioManager.updateAudioPortCache(…),-,7831,int android.media.AudioManager.updateAudioPortCache(…),99]
```

⇒ `wait_ms = 497`, `sample_percent = 99`, `owner_file = "-"` (the "same file, save log space"
shorthand, `monitor_android.cc:88-93`).

Measured: **168** lines, of which **149** name `system_server` → **88.69 %**.

> **The count is not a count.** A 497 ms wait logged with `sample_percent 99` means the true
> contention count is ≈ 168/0.99 ≈ 170 — but a 100 ms wait at the default threshold logs ~10 %
> of the time, so the *distribution* of unseen short waits is unbounded. **The schema records
> `sampled_event_count` and the observed `sample_percent` values, never a bare
> `contention_count`**, and `counting_caveat` is mandatory prose to that effect.

`lock_profiling_threshold_` comes from `Opt::LockProfThreshold`
(`art/runtime/runtime.cc:1589`; `runtime_options.def:123`) and is **zero on user builds** — in
which case `log_contention` is `false` and the whole branch is skipped (`monitor.cc:458`).

**C12 — ART runtime mutex.** `Mutex::DumpStack` (`art/runtime/base/mutex.cc:567-571`) is
enabled (`mutex.cc:533` called from `:481-499`) but produced **zero** events: no event satisfied
the 50 ms × `try_times` rate limit (`kIntervalMillis = 50`, `:42`; `IsDumpFrequent()` `:576-591`).
**Presence-with-zero is a result, not an absence**, and `zero_reason` states it.

#### Field tables

Condensed. The owning file's exhaustive row-level table is preserved at
`tmp/aosp-team/03-memory-and-cpu.md` §5.4.

##### `utilization[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `dumpsys_cpuinfo` / `top` / `cpu_monitor`. | — |
| `line_start` | integer | yes | Archive locator. | measured |
| `window` | object | yes | `{start_utc, end_utc, duration_ms}`. **Mandatory** — a ratio without its window is meaningless. | `ProcessCpuTracker.java:860-872` |
| `window_source` | enum | yes | `section_body` (cpuinfo prints it) / `unknown` (top does not). | `ProcessCpuTracker.java:862-871` |
| `age_at_print_ms` | object \| null | yes | The `22612ms … 20840ms ago` pair. | `ProcessCpuTracker.java:861-865` |
| `total_pct` | number \| null | yes | | `:914` |
| `pct_user` / `pct_kernel` / `pct_iowait` / `pct_irq` / `pct_softirq` | number \| null | yes | | `:947-948`, `:949-950`, `:953-954`, `:958-959`, `:963-964` |
| `pct_nice` | number \| null | yes | **`null` for `dumpsys_cpuinfo`** — not in `SYSTEM_CPU_FORMAT`. | `:126-134` |
| `pct_steal` | number \| null | yes | **`null` for `dumpsys_cpuinfo`** — structurally unavailable; only per-CPU via `cpu_monitor`. | `:126-134`; `CpuInfoReader.java:69` |
| `null_reasons` | object | yes | **Required** for every `null` percentage, naming the structural cause. | — |
| `online_cpus` | integer \| null | yes | The `400%` denominator. | `dumpstate.cpp:1690`; `CpuMonitorService.java:205` |
| `threads_total` / `running` / `sleeping` / `stopped` / `zombie` | integer \| null | yes | **`top` only — source not in this drop, semantics unverified.** | `top` |
| `provenance` | object | yes | | — |

##### `per_cpu_core[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `core_id` | integer | yes | 0..3. | `CpuInfoReader.java:63-70` |
| `is_online` | boolean \| null | yes | `isOnline = Yes`. | `CpuInfoReader` `CpuInfo` `toString` |
| `user_time_ms` / `nice_time_ms` / `system_time_ms` / `idle_time_ms` | integer \| null | yes | Cumulative since boot. | `CpuInfoReader.java:859-870` |
| `iowait_time_ms` / `irq_time_ms` / `softirq_time_ms` | integer \| null | yes | Cumulative since boot. | `:865-867` |
| `steal_time_ms` | integer \| null | yes | **The only per-CPU source of steal in a bugreport.** | `:868` |
| `guest_time_ms` / `guest_nice_time_ms` | integer \| null | yes | Cumulative since boot. | `:869-870` |
| `busy_pct_since_boot` | number \| null | yes | **Derived** — busy ÷ sum of all buckets. Carries `busy_pct_derivation`. AOSP publishes only raw milliseconds and never this percentage, so an implementation computing it needs a unit test against a hand-summed row. | derived |
| `read_at_uptime_ms` | integer \| null | yes | `mLastReadUptimeMillis = 11192`. | `CpuInfoReader.java:75` |
| `provenance` | object | yes | `capture_kind: command_output`, and **must not be read as a window**. | — |

##### `processes[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `pid` | integer \| null | yes | | `ProcessCpuTracker.java:899-908` |
| `label` | string | yes | | same |
| `pct_total` / `pct_user` / `pct_kernel` | number \| null | yes | | `:939-940`, `:947-948`, `:949-950` |
| `minor_faults` / `major_faults` | integer \| null | yes | | `PROCESS_STAT_MINOR_FAULTS` / `MAJOR_FAULTS` `:75-78` |
| `is_thread_row` | boolean | yes | `true` for the indented rows. | `:899-908` |
| `window_ref` | string | yes | Must point at a `utilization[].window`. | — |
| `provenance` | object | yes | | — |

##### `contention`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `art_java_monitor.event_count` | integer | yes | 33. | measured |
| `art_java_monitor.channel_present` | boolean | yes | `true` — the `log_contention` gate was on. | `monitor.cc:458` |
| `art_java_monitor.threshold_ms` | integer \| null | yes | **1000 on a debuggable build.** Carries `threshold_derivation`. | `monitor.cc:53-54` |
| `art_java_monitor.wait_ms_max` / `wait_ms_min` | integer \| null | yes | 1780 / 105. | measured |
| `art_java_monitor.events_in_system_server` | integer | yes | **0.** | measured |
| `art_java_monitor.events[]` | array | yes | `{timestamp, process, pid, tid, owner_name, owner_tid, owner_method, owner_file, owner_line, waiters, waiter_method, wait_ms, wait_verbatim}`. **`wait_verbatim` is kept so a consumer can see the unit ART chose.** `waiters` is a queue depth and the most diagnostic single field. | `monitor.cc:409-418,597`; `time_utils.cc:53-75` |
| `art_runtime_mutex` | object | yes | `{channel_present, event_count, format, short_format, zero_reason}`. `event_count` **0** — present, enabled, silent. | `mutex.cc:42,533,567-571,576-591` |
| `art_rate_limiting` | object | yes | `{java_monitor_is_sampled, java_monitor_sampling_rule, java_monitor_sample_pct_observed, java_monitor_threshold_source, runtime_mutex_is_rate_limited, runtime_mutex_rate_limit, counting_caveat}`. **`counting_caveat` is mandatory prose: no field in `contention` is a contention count.** | `monitor.cc:529-536`; `monitor_android.cc:105`; `runtime.cc:1589` |
| `dvm_lock_sample` | object | yes | `{sampled_event_count: 168, in_system_server: 149, system_server_pct: 0.8869, field_order_source, example}`. Named `sampled_event_count`, **not** `event_count` — the name is the warning. | `monitor_android.cc:35-108` |
| `provenance` | object | yes | `capture_kind: live_log`. **The window is the log buffer, not the CPU window.** | — |

##### `load`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `sources[]` | array of object | yes | `{source: bugreport_header \| uptime_section \| dumpsys_cpuinfo, load_1m, load_5m, load_15m, uptime_ms_at_read, uptime_qualifier, line_start}`. **All three readouts are published, because on this archive they disagree.** | `ProcessCpuTracker.java:829-838`; `dumpstate.cpp:1687` |
| `uptime_ms_at_read` | integer \| null | yes | **Required whenever `load_1m` is present.** | header / `uptime` output |
| `online_cpus` | integer \| null | yes | Load is normalised by runnable CPUs, so 10.42 on 4 CPUs is 261 % runnable demand. | `ProcessCpuTracker.java:139-144` |
| `boot_in_progress` | boolean | yes | **Computed**: `uptime_ms_at_read < 15 * load_15m * 1000`. | derived |
| `interpretation` | string | yes | **Mandatory prose when `boot_in_progress` is true.** | derived |
| `provenance` | object | yes | | — |

> **Why `uptime_ms_at_read` is required.** Load averages are EWMAs decayed by the kernel's
> `LOAD_FREQ`. On a device with 30 s of uptime the 1-minute average has only ever seen boot
> work, so it is arithmetically meaningless as a load signal. On this archive the header says
> uptime `< 1 minute` while `load_15m = 0.78` implies 0.78 × 900 = **702 s** of history — a
> contradiction explicable only because `/proc/loadavg` is not reset on the first userspace
> processes, or because the 15-minute EWMA is seeded from the previous boot. Either way the
> number cannot be a steady-state load.

##### `stall_signals`, `graphics`, `power_history`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `stall_signals` | `psi_cpu` / `psi_io` | object \| null | yes | `{some:{avg10,avg60,avg300,total_us}, full:{…}}`. | `dumpstate.cpp:1979,1981` |
| | `iotop` | object \| null | yes | Same shape as `memory.json#/io/iotop`, **with** its own `provenance`. | `iotop.cpp:36-39,234,247-265` |
| | `proc_churn` | object | yes | `{am_proc_start_count, am_proc_died_count, am_proc_bound_count, am_uid_running_count, am_uid_active_count, window, rate_per_s, rate_valid, rate_suppressed_reason}`. **The window is the event-log window, not the CPU window.** | `EventLogTags.logtags:25,22`; `dumpstate.cpp:1028` |
| | `proc_churn.rate_valid` | boolean | yes | **Gates `rate_per_s`.** A per-boot churn rate computed over a window shorter than 120 s is dominated by the single burst of zygote/ART initialisation and means nothing. | derived |
| `graphics` | `per_process[]` | array \| null | yes | `{pid, process_name, stats_since_ns, total_frames, janky_frames, janky_pct, p50_ms, p90_ms, p95_ms, p99_ms}`. **gfxinfo windows are per-process and independent of `utilization[].window`.** | `AMS:1931`; `frameworks/base/libs/hwui/ProfileData.cpp:114` |
| | `window_note` | string | yes | | — |
| `power_history` | `batterystats_reset_time` / `batterystats_duration_s` / `powerstats_earliest_data_time` | string / integer | yes | | `dumpstate.cpp:2130,2134` |
| | `quarantine_reason` | string | yes | **These sections describe a *previous boot*.** They are stored here and **never** joined to `utilization` or to any uptime-derived field. `powerstats` reports `Earliest data time : Wed May 20 02:34:12 GMT+07:00 2026`; batterystats resets at `2026-05-19-00-16-18`. | derived |

#### The `61% TOTAL` and `400%cpu` disagreement — correctly

| Line | Source | Value |
| --- | --- | --- |
| 3709 | `dumpsys cpuinfo` (`ProcessCpuTracker.java:890-892` denominator, `:914` the `TOTAL` row) | `61% TOTAL: 9.2% user + 17% kernel + 8.3% iowait + 25% irq + 1% softirq` |
| 57472 | `top` (`dumpstate.cpp:1690`) | `400%cpu  81%user  12%nice 126%sys  88%idle  12%iow  69%irq  12%sirq   0%host` |

Both are correct over **different windows** (1.772 s at 00:26:57 versus `top`'s own sample) and
against **different denominators** (`cpuinfo` divides by a time span in which the device can be
partly idle; `top` normalises to 400 % = 4 online cores).

Note also that the earlier reading of "9.2 user / 17k nice" was a **misreading**: the archive says
**`17% kernel`**, not nice — nice is `12%` in the `top` line. **A schema that merges these two
lines into one "CPU %" is unsound**, so the schema keeps them as two entries of the `utilization`
array, each with its own window.

**On the 25 % `irq`.** `stealTimeMillis = 0` on all four cores and `0%host` in `top` mean the
guest was **not** descheduled by the hypervisor. Combined with the product being an emulator
(`ro.ranchu`, fingerprint `…/emulator_car64_arm64`), the 25 % `irq` in the `cpuinfo` window is
best explained as **host→guest virtio device interrupt injection being accounted in the guest's
`irq` bucket, not by a guest driver defect**. **This is a labelled inference, not a schema
field** — the schema carries only the measured `irq` and `steal`. A finding may carry
`inference: true` and the supporting observations.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "cpu",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "cpu_clean_channels": ["dumpsys_cpuinfo"],
    "cpu_perturbed_channels": ["top", "iotop", "dumpsys_meminfo", "bugreport_procdump"],
    "mechanism": "cpuinfo is DUMP_FLAG_PRIORITY_CRITICAL (AppProfiler.java:2348) and meminfo is DUMP_FLAG_PRIORITY_HIGH (ActivityManagerService.java:1929-1930); dumpstate walks them in priority order (dumpstate.cpp:1262-1310), so the CPU service is dumped FIRST (archive line 3681, inside the block opening at line 47) and the memory service 19 s later (archive line 319846). On top of that, cpuinfo's numbers are not taken during the dump at all - they are the last two samples of a background thread that refreshes at most every 5 s (AppProfiler.java:142, 2156-2159). CPU is therefore measured BEFORE collection; memory is measured INSIDE it.",
    "page_scanning_total_ms": 8712
  },

  "utilization": [
    { "channel": "dumpsys_cpuinfo", "line_start": 3709,
      "window": { "start_utc": "2026-09-24T00:26:56.813+07:00",
                  "end_utc": "2026-09-24T00:26:58.585+07:00", "duration_ms": 1772 },
      "window_source": "section_body",
      "age_at_print_ms": { "start": 22612, "end": 20840 },
      "pct_awake": null,
      "total_pct": 61, "pct_user": 9.2, "pct_kernel": 17, "pct_iowait": 8.3,
      "pct_irq": 25, "pct_softirq": 1,
      "pct_nice": null, "pct_steal": null,
      "online_cpus": 4,
      "null_reasons": {
        "pct_nice": "SYSTEM_CPU_FORMAT (ProcessCpuTracker.java:126-134) stops at softirq",
        "pct_steal": "same; steal is only readable per-CPU via cpu_monitor (CpuInfoReader.java:69)"
      },
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "DUMPSYS CRITICAL",
        "section_ordinal": 0, "line_start": 3681, "line_end": 3710,
        "captured_at": "2026-09-24T00:26:58.585+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 18.415, "capture_kind": "background_sampler",
        "freshness": "in_window",
        "freshness_basis": { "input": "ProcessCpuTracker.java:860-872 window header",
          "basis": "the sample window ended 18.415 s BEFORE dumpstate started (00:27:17), so it cannot include collection cost; it is also inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "sample window ended 18.415 s before dumpstate started; cannot include collection cost" } },

    { "channel": "top", "line_start": 57472,
      "window": { "start_utc": null, "end_utc": null, "duration_ms": null },
      "window_source": "unknown",
      "total_pct": null,
      "pct_user": 81, "pct_nice": 12, "pct_kernel": 126, "pct_iowait": 12,
      "pct_irq": 69, "pct_softirq": 12, "pct_steal": 0,
      "online_cpus": 4,
      "threads_total": 2769, "running": 19, "sleeping": 2750, "stopped": 0, "zombie": 0,
      "null_reasons": {
        "total_pct": "top's aggregate row is normalised to 400% across 4 cores; it is not a 0-100 'busy' figure",
        "window": "top is not in this source drop; its sampling window is not verifiable",
        "pct_steal": "read as '0%host' from the output header; the column's semantics are unverified"
      },
      "semantics_verified": false,
      "provenance": { "evidence_origin": "cpu_info_top", "section_title": "CPU INFO", "section_ordinal": 0,
        "line_start": 57464, "line_end": 57480,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "top prints no clock; positioned only by dump statement order (dumpstate.cpp:1690)",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "mid-dump; its own 'Swap:' line already shows the collector's effect (18176K used vs 18688K at the immediately preceding /proc/meminfo read); source binary not present in the AOSP drop, so its columns are unverified" } }
  ],

  "per_cpu_core": [
    { "core_id": 0, "is_online": true, "user_time_ms": 340, "nice_time_ms": 30,
      "system_time_ms": 5040, "idle_time_ms": 2510, "iowait_time_ms": 490,
      "irq_time_ms": 2370, "softirq_time_ms": 260, "steal_time_ms": 0,
      "guest_time_ms": 0, "guest_nice_time_ms": 0, "busy_pct_since_boot": 0.7726,
      "read_at_uptime_ms": 11192 },
    { "core_id": 1, "is_online": true, "user_time_ms": 460, "nice_time_ms": 20,
      "system_time_ms": 1190, "idle_time_ms": 7450, "iowait_time_ms": 820,
      "irq_time_ms": 1020, "softirq_time_ms": 20, "steal_time_ms": 0,
      "guest_time_ms": 0, "guest_nice_time_ms": 0, "busy_pct_since_boot": 0.3215,
      "read_at_uptime_ms": 11192 },
    { "core_id": 2, "is_online": true, "user_time_ms": 440, "nice_time_ms": 30,
      "system_time_ms": 1320, "idle_time_ms": 7170, "iowait_time_ms": 1010,
      "irq_time_ms": 960, "softirq_time_ms": 20, "steal_time_ms": 0,
      "guest_time_ms": 0, "guest_nice_time_ms": 0, "busy_pct_since_boot": 0.3452,
      "read_at_uptime_ms": 11192 },
    { "core_id": 3, "is_online": true, "user_time_ms": 580, "nice_time_ms": 10,
      "system_time_ms": 1310, "idle_time_ms": 7000, "iowait_time_ms": 980,
      "irq_time_ms": 970, "softirq_time_ms": 20, "steal_time_ms": 0,
      "guest_time_ms": 0, "guest_nice_time_ms": 0, "busy_pct_since_boot": 0.3560,
      "read_at_uptime_ms": 11192 }
  ],

  "processes": [
    { "pid": 683, "label": "system_server", "pct_total": 35, "pct_user": 25, "pct_kernel": 10,
      "minor_faults": 17362, "major_faults": 11, "is_thread_row": false,
      "window_ref": "utilization[0].window" },
    { "pid": 494, "label": "logcat", "pct_total": 15, "pct_user": 0, "pct_kernel": 15,
      "is_thread_row": false, "window_ref": "utilization[0].window" },
    { "pid": 564, "label": "bootanimation", "pct_total": 11, "pct_user": 3.3, "pct_kernel": 7.9,
      "minor_faults": 8623, "is_thread_row": false, "window_ref": "utilization[0].window" },
    { "pid": 416, "label": "android.hardware.graphics.composer3-service.ranchu",
      "pct_total": 6.2, "minor_faults": 3, "is_thread_row": false, "window_ref": "utilization[0].window" },
    { "pid": 55, "label": "kworker/u9:0-fscrypt_read_queue", "pct_total": 1.1,
      "is_thread_row": true, "window_ref": "utilization[0].window" }
  ],

  "contention": {
    "art_java_monitor": {
      "event_count": 33,
      "channel_present": true,
      "threshold_ms": 1000,
      "threshold_derivation": "kLongWaitMs = 100 * kDebugThresholdFudgeFactor; kDebugThresholdFudgeFactor = kIsDebugBuild ? 10 : 1 (art/runtime/monitor.cc:53-54). This is a userdebug build, so 100 * 10 = 1000. A 200 ms stall produces no line here and its absence is NOT evidence of health.",
      "wait_ms_max": 1780, "wait_ms_min": 105,
      "events_over_threshold_1s": 6,
      "events_in_system_server": 0,
      "events_over_threshold_1s_in_system_server": 0,
      "events": [
        { "timestamp_utc": "2026-09-24T00:27:16.117+07:00", "process": "com.google.android.gms",
          "pid": 2488, "tid": 2957, "owner_name": "axte.h", "owner_tid": 0,
          "owner_method": "axte.h(java.lang.String)",
          "owner_file": "com.google.android.gms@254232109@25.42.32", "owner_line": 28,
          "waiters": 0, "waiter_method": "axte.d(java.lang.String)",
          "wait_ms": 1780, "wait_verbatim": "1.780s" },
        { "timestamp_utc": "2026-09-24T00:27:16.117+07:00", "process": "com.google.android.gms",
          "pid": 2488, "tid": 2819, "owner_method": "axte.h(java.lang.String)",
          "owner_line": 28, "waiters": 1, "wait_ms": 1629, "wait_verbatim": "1.629s" },
        { "timestamp_utc": "2026-09-24T00:27:16.123+07:00", "process": "com.google.android.gms",
          "pid": 2488, "tid": 2899, "owner_method": "axte.h(java.lang.String)",
          "owner_line": 28, "waiters": 2, "wait_ms": 1628, "wait_verbatim": "1.628s" },
        { "timestamp_utc": "2026-09-24T00:27:20.176+07:00", "process": "com.google.android.gms.persistent",
          "pid": 2433, "tid": 2696, "owner_line": 185, "waiters": 0,
          "wait_ms": 1396, "wait_verbatim": "1.396s" }
      ],
      "events_truncated_in_example": 29
    },
    "art_runtime_mutex": {
      "channel_present": true, "event_count": 0,
      "format": "Monitor contention with tid <owner_tid>, wait time: <ms>ms, monitor id: <monitor_id>",
      "short_format": "Contention with tid <owner_tid>, monitor id <monitor_id>",
      "zero_reason": "channel enabled (art/runtime/base/mutex.cc:533 called from :481-499) but no event satisfied the 50ms * try_times rate limit (:576-591) during the log window. Presence-with-zero is a result, not an absence."
    },
    "art_rate_limiting": {
      "java_monitor_is_sampled": true,
      "java_monitor_sampling_rule": "sample_percent = 100 * wait_ms / lock_profiling_threshold_, emitted when rand()%100 < sample_percent (art/runtime/monitor.cc:529-536)",
      "java_monitor_sample_pct_observed": [99],
      "java_monitor_threshold_source": "Opt::LockProfThreshold (art/runtime/runtime.cc:1589; runtime_options.def:123)",
      "runtime_mutex_is_rate_limited": true,
      "runtime_mutex_rate_limit": "kIntervalMillis = 50 (art/runtime/base/mutex.cc:42), skipped when interval < 50 * try_times (:576-591)",
      "counting_caveat": "168 dvm_lock_sample events and 33 Long monitor contention events are NOT contention counts. The first is a probability sample whose rate depends on each wait's own duration (a 497 ms wait at sample_percent 99 implies ~170 true events, but a 100 ms wait at the default threshold logs ~10% of the time, so the distribution of unseen short waits is unbounded). The second has a 1000 ms reporting floor on a debuggable build. Short waits below the floor are unobservable."
    },
    "dvm_lock_sample": {
      "sampled_event_count": 168, "in_system_server": 149, "system_server_pct": 0.8869,
      "field_order_source": "art/runtime/monitor_android.cc:35-108, tag 20003 at :31",
      "example": { "timestamp_utc": "2026-09-24T00:27:01.455+07:00", "pid": 683, "tid": 683,
        "process_name": "system_server", "is_sensitive_thread": 1, "thread_name": "main",
        "wait_ms": 497, "waiter_file": "AudioManager.java", "waiter_line": 7748,
        "waiter_method": "int android.media.AudioManager.updateAudioPortCache(java.util.ArrayList, java.util.ArrayList, java.util.ArrayList)",
        "owner_file": "-", "owner_line": 7831,
        "owner_method": "int android.media.AudioManager.updateAudioPortCache(java.util.ArrayList, java.util.ArrayList, java.util.ArrayList)",
        "sample_percent": 99,
        "owner_file_note": "'-' means owner_file == waiter_file (space optimisation, monitor_android.cc:88-93)" }
    },
    "provenance": { "evidence_origin": "art_monitor_contention", "archive_path": "<main_entry>",
      "section_title": "SYSTEM LOG", "section_ordinal": 0, "line_start": 27041, "line_end": 53823,
      "captured_at": "2026-09-24T00:27:16.117+07:00", "captured_at_time_base": "logcat_line",
      "age_seconds_at_capture": 0.883, "capture_kind": "live_log", "freshness": "in_window",
      "freshness_basis": { "input": "logcat line stamp",
        "basis": "contention events are logged as they occur, inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "the dump neither causes nor suppresses contention events" }
  },

  "load": {
    "sources": [
      { "source": "bugreport_header", "load_1m": 10.42, "load_5m": 2.38, "load_15m": 0.78,
        "uptime_ms_at_read": null, "uptime_qualifier": "up 0 weeks, 0 days, 0 hours, 0 minutes",
        "line_start": 43 },
      { "source": "uptime_section", "load_1m": 9.66, "load_5m": 2.36, "load_15m": 0.79,
        "uptime_qualifier": "up 0 min", "captured_at": "2026-09-24T00:27:25+07:00", "line_start": 57013 },
      { "source": "dumpsys_cpuinfo", "load_1m": 0.39, "load_5m": 0.08, "load_15m": 0.03, "line_start": 3685 }
    ],
    "online_cpus": 4,
    "boot_in_progress": true,
    "interpretation": "All three readouts disagree because they are at different instants of a sub-minute boot. 10.42 on 4 CPUs is 261% runnable demand during initial userspace start; the cpuinfo sample (00:26:58, load 0.39) predates the burst that produced it. A load_15m of 0.78 implies ~702 s of history, which cannot exist on a boot this young, so the kernel EWMA is seeded rather than measured. load_1m MUST NOT be reported without uptime_ms_at_read.",
    "provenance": { "evidence_origin": "loadavg", "captured_at": "2026-09-24T00:26:58+07:00",
      "captured_at_time_base": "dumpsys_section_snapshot", "age_seconds_at_capture": 19,
      "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "section body",
        "basis": "a kernel EWMA reading, not a dump-scanned value", "threshold_seconds": 3600 },
      "collector_perturbed": false, "collector_perturbed_basis": "kernel EWMA" }
  },

  "stall_signals": {
    "psi_cpu": { "some": { "avg10": 65.00, "avg60": 25.63, "avg300": 6.26, "total_us": 19879787 },
                 "full": { "avg10": 0.00, "avg60": 0.00, "avg300": 0.00, "total_us": 0 } },
    "psi_io":  { "some": { "avg10": 12.36, "avg60": 5.82, "avg300": 1.50, "total_us": 4881350 },
                 "full": { "avg10": 2.84, "avg60": 1.82, "avg300": 0.50, "total_us": 1794988 } },
    "iotop": { "window_ms": 1000, "row_granularity": "thread", "sort_column": "total",
      "max_io_delay_pct": 0.00, "max_swap_delay_pct": 0.00,
      "max_sched_delay_pct": 24.65, "max_mem_delay_pct": 0.00,
      "resolution_floor_note": "0.00% means <0.1 ms/thread over 1 s (iotop.cpp:36-39 printed as %5.2f%%, :255-265), not zero I/O; the same table shows 4904 KiB/s read",
      "provenance": { "capture_kind": "command_output", "captured_at": null, "freshness": "undated",
        "collector_perturbed": true,
        "collector_perturbed_basis": "runs mid-dump; dumpstate and iotop threads appear in the table" } },
    "proc_churn": {
      "am_proc_start_count": 121, "am_proc_died_count": 40, "am_proc_bound_count": 121,
      "am_uid_running_count": 68, "am_uid_active_count": 64,
      "window": { "start_utc": "2026-09-24T00:26:47.418+07:00", "end_utc": "2026-09-24T00:27:52.080+07:00",
                  "duration_seconds": 64.662, "coverage_ratio": 0.314 },
      "rate_per_s": null,
      "rate_valid": false,
      "rate_suppressed_reason": "The EVENT LOG window is 64.662 s and its buffer covers only 20.304 s of that (coverage 0.314). A per-boot churn rate computed over a window shorter than 120 s is dominated by the single burst of zygote/ART initialisation and means nothing. 121/64.662 = 1.873/s is arithmetically correct and MUST NOT be published; rate_valid gates rate_per_s.",
      "provenance": { "capture_kind": "live_log", "captured_at": "2026-09-24T00:26:59.872+07:00",
        "captured_at_time_base": "logcat_line", "freshness": "in_window",
        "collector_perturbed": false,
        "collector_perturbed_basis": "EVENT LOG captured at dumpstate.cpp:1028, before any heavy scan" } }
  },

  "graphics": {
    "device_uptime_ms_at_capture": 57918,
    "per_process": [
      { "pid": 3037, "process_name": "com.android.car.mapsplaceholder",
        "stats_since_ns": 25682514720, "total_frames": 0, "janky_frames": 0, "janky_pct": 0.0,
        "p50_ms": 4950, "p90_ms": 4950, "p95_ms": 4950, "p99_ms": 4950 }
    ],
    "window_note": "stats_since_ns = 25.68 s of a 57.918 s-old device; zero frames rendered because the process is a placeholder with no surface. gfxinfo windows are per-process and independent of utilization[].window.",
    "provenance": { "captured_at": "2026-09-24T00:27:25+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
      "section_duration_ms": 849, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": false, "collector_perturbed_basis": "frame counters, not a memory scan" }
  },

  "power_history": {
    "batterystats_reset_time": "2026-05-19-00-16-18",
    "batterystats_duration_s": 1537272,
    "batterystats_history_used_pct": 0.03,
    "powerstats_earliest_data_time": "2026-05-20T02:34:12+07:00",
    "freshness": "stale",
    "quarantine_reason": "Battery history starts 2026-05-19 and the powerstats meter starts 2026-05-20; both precede the 2026-09-24 boot by ~3 months. These describe a previous boot and are stored only so a reader can see they exist. They MUST NOT be joined to utilization[] or to any uptime-derived field.",
    "provenance": { "capture_kind": "persisted_aggregate", "captured_at": "2026-05-19T00:16:18+07:00",
      "captured_at_time_base": "dumpsys_section_snapshot", "capture_kind_derivation": "ring buffer since RESET:TIME",
      "freshness": "stale" }
  },

  "counts": {
    "dvm_lock_sample_hits": {
      "pattern": " dvm_lock_sample:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 168, "matched_evidence": 0, "suppressed_noise": 168, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the 168 dvm_lock_sample lines are NOT contention events; they are a duration-weighted probability sample (monitor.cc:529-536) and are reported separately under contention.dvm_lock_sample as sampled_event_count",
      "coverage": { "window_start": "2026-09-24T00:26:47.418+07:00", "window_end": "2026-09-24T00:27:52.080+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": 53824, "line_end": 55656, "line_count": 168 } ]
    },
    "long_monitor_contention_hits": {
      "pattern": "Long monitor contention", "pattern_kind": "substring", "case_sensitive": false,
      "raw_count": 33, "matched_evidence": 33, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "2026-09-24T00:26:47.418+07:00", "window_end": "2026-09-24T00:27:52.080+07:00",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": 27041, "line_end": 53823, "line_count": 33 } ]
    },
    "monitor_contention_with_tid_hits": {
      "pattern": "Monitor contention with tid", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "channel present and enabled (mutex.cc:533 from :481-499); zero events satisfied the 50ms rate limit (:576-591). Presence-with-zero is a result.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "held_by_thread_hits": {
      "pattern": "held by thread", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 1, "matched_evidence": 0, "suppressed_noise": 1, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the single match is at archive line 310026, inside the VM TRACES AT LAST ANR section that starts at line 309986 and is 126.80 days stale. Reporting it as a live lock-contention finding is the exact false positive this schema exists to prevent.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "VM TRACES AT LAST ANR", "section_ordinal": 0,
                     "line_start": 310026, "line_end": 310026, "line_count": 1 } ]
    }
  },

  "findings": [
    { "id": "cpu-001", "kind": "clean_window",
      "title": "CPU utilization window 00:26:56.813-00:26:58.585 (1.772 s) ended 18.415 s before dumpstate started; 61% total is not inflated by the collection",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "channel": "dumpsys_cpuinfo", "capture_kind": "background_sampler" },
      "metrics": { "window_duration_ms": 1772, "age_before_dump_ms": 18415,
                   "pct_total": 61, "pct_user": 9.2, "pct_kernel": 17, "pct_iowait": 8.3,
                   "pct_irq": 25, "pct_softirq": 1 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 3686, "line_end": 3686 },
        "excerpt": "CPU usage from 22612ms to 20840ms ago (2026-09-24 00:26:56.813 to 2026-09-24 00:26:58.585):",
        "excerpt_truncated": false, "excerpt_bytes": 100, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "DUMPSYS CRITICAL",
        "section_ordinal": 0, "line_start": 3681, "line_end": 3710,
        "captured_at": "2026-09-24T00:26:58.585+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 18.415, "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "window header plus the dumpsys footer",
          "basis": "window ended 18.415 s before dumpstate started and inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "sample predates dumpstate by 18.415 s" } },

    { "id": "cpu-002", "kind": "host_interrupt_injection",
      "title": "25% irq with stealTimeMillis=0 on all 4 cores and top reporting 0%host - inferred host-to-guest virtio interrupt injection",
      "severity": "info", "unconstrained_severity": "info", "confidence": "inferred",
      "inference": true,
      "process_key": null, "event_time": null,
      "attributes": { "inference_basis": ["ro.ranchu", "fingerprint …/emulator_car64_arm64",
                                          "stealTimeMillis=0 on cores 0-3", "top 0%host"] },
      "metrics": { "pct_irq": 25, "steal_time_ms": 0, "cores_measured": 4 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 3667, "line_end": 3670 },
        "excerpt": "CPU core id = 0, CpuUsageStats{ userTimeMillis = 340, niceTimeMillis = 30, systemTimeMillis = 5040, idleTimeMillis = 2510, iowaitTimeMillis = 490, irqTimeMillis = 2370, softirqTimeMillis = 260, stealTimeMillis = 0, …",
        "excerpt_truncated": true, "excerpt_bytes": 512, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpu_monitor", "section_title": "DUMPSYS CRITICAL",
        "section_ordinal": 0, "line_start": 3667, "line_end": 3670,
        "captured_at": "2026-09-24T00:26:58+07:00", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": 19, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "dumpsys footer", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "see cpu-001" } },

    { "id": "cpu-003", "kind": "boot_load_not_steady_state",
      "title": "Header load average 10.42 / 2.38 / 0.78 on a device with sub-minute uptime is a boot-in-progress signature, not sustained load",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "boot_in_progress": true },
      "metrics": { "load_1m": 10.42, "load_5m": 2.38, "load_15m": 0.78, "online_cpus": 4 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 43, "line_end": 43 },
        "excerpt": "Uptime: up 0 weeks, 0 days, 0 hours, 0 minutes,  load average: 10.42, 2.38, 0.78",
        "excerpt_truncated": false, "excerpt_bytes": 80, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "captured_at": "2026-09-24T00:27:17+07:00", "captured_at_time_base": "dumpstate_header",
        "age_seconds_at_capture": 0, "capture_kind": "point_in_time_snapshot", "freshness": "now",
        "freshness_basis": { "input": "header uptime", "basis": "printed in the archive header itself",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "kernel EWMA" } },

    { "id": "cpu-004", "kind": "contention_localized",
      "title": "33 ART Java-monitor stalls, max 1.780s, ALL 6 events over 1s inside com.google.android.gms on a single MessageQueue-shaped monitor; zero over 1s in system_server",
      "severity": "medium", "unconstrained_severity": "medium", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "localized_to": ["com.google.android.gms", "com.google.android.gms.persistent"],
                      "owner_method": "axte.h(java.lang.String)" },
      "metrics": { "art_java_monitor_event_count": 33, "wait_ms_max": 1780, "wait_ms_min": 105,
                   "events_over_threshold_1s": 6, "events_in_system_server": 0,
                   "dvm_lock_sample_sampled_event_count": 168, "dvm_lock_sample_max_wait_ms": 497 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 27041, "line_end": 27041 },
        "excerpt": "Long monitor contention with owner axte.h (0) at axte.h(java.lang.String)(com.google.android.gms@254232109@25.42.32:28) waiters=0 in axte.d(java.lang.String) for 1.780s",
        "excerpt_truncated": true, "excerpt_bytes": 512, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "art_monitor_contention", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": 27041, "line_end": 53823,
        "captured_at": "2026-09-24T00:27:16.117+07:00", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": 0.883, "capture_kind": "live_log", "freshness": "in_window",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "logged as they occur, inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "logged as they occur" } },

    { "id": "cpu-005", "kind": "no_io_stall",
      "title": "IOTOP over its 1s window shows max IO/swap/mem delay 0.00% (below the resolution floor) but max run-queue delay 24.65% on three Blocking Thread entries",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "note": "0.00% is a %.2f resolution floor, not zero I/O; the same table carries 4904 KiB/s read" },
      "metrics": { "max_sched_delay_pct": 24.65, "max_io_delay_pct": 0.0,
                   "psi_io_full_avg10": 2.84, "psi_cpu_full_avg10": 0.0, "psi_memory_full_avg10": 0.97 },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": 56843, "line_end": 56850 },
        "excerpt": "  2630 Blocking Thread       0   2308   2308      0     99  0.00%  0.00% 24.65%  0.00% 24.65%",
        "excerpt_truncated": false, "excerpt_bytes": 87, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "iotop", "section_title": "IOTOP", "section_ordinal": 0,
        "line_start": 56843, "line_end": 56860,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "iotop prints no clock; the measured window is 1 s from its own sample loop (iotop.cpp:226-273)",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "mid-dump 1 s sample" } }
  ],

  "negative_evidence": [
    { "scope": "ART runtime-mutex contention channel",
      "statement": "channel enabled and searched; 0 'Monitor contention with tid' events satisfied the 50 ms rate limit during the log window",
      "source": { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "notes": [
    "CPUINFO_WINDOW_PREDATES_DUMP: the dumpsys cpuinfo window is 1.772 s ending 18.415 s BEFORE dumpstate started, because CpuBinder.dumpCritical prints pre-existing background samples (AppProfiler.java:2332-2335) from a thread that refreshes at most every 5 s (:142, :2156-2160). collector_perturbed is false for this channel and true for top, iotop, dumpsys_meminfo and bugreport_procdump.",
    "NO_STEAL_IN_GLOBAL_RATIO: dumpsys cpuinfo's TOTAL structurally cannot contain steal because SYSTEM_CPU_FORMAT (ProcessCpuTracker.java:126-134) stops at softirq. Steal is only available per-CPU via the cpu_monitor binder service. pct_steal is null for dumpsys_cpuinfo with null_reasons stating why; it is never 0 by default.",
    "TOTAL_LINES_DISAGREE_CORRECTLY: '61% TOTAL' (cpuinfo, 1.772 s window) and '400%cpu' (top, own sample) are both correct over different windows against different denominators. They are two entries of utilization[], never merged.",
    "CPU_BREAKDOWN_CORRECTED: the archive says '17% kernel', not nice; nice is 12% in the top line.",
    "CONTENTION_IS_NEVER_A_COUNT: 33 Long monitor contention events have a 1000 ms reporting floor on a debuggable build (monitor.cc:53-54), and 168 dvm_lock_sample events are a duration-weighted probability sample (monitor.cc:529-536). contention.art_rate_limiting.counting_caveat is mandatory prose.",
    "HELD_BY_THREAD_IS_STALE: the single 'held by thread' match in the archive is at line 310026, inside the 126.80-day-old VM TRACES AT LAST ANR section. counts.held_by_thread_hits records it as stale_region, not as live contention.",
    "PROC_CHURN_RATE_GATED: am_proc_start=121 over a 64.662 s event-log window with 0.314 buffer coverage gives 1.873/s arithmetically, which MUST NOT be published. rate_valid is false and rate_per_s is null.",
    "POWER_HISTORY_QUARANTINED: batterystats and powerstats describe a previous boot and are never joined to utilization[] or to any uptime-derived field."
  ]
}
```

#### Open questions

- **`top` and `procrank` semantics are unverified.** Neither binary is in this source drop. For
  `top` only the invocation is citable (`dumpstate.cpp:1690-1691`), so column names are read off
  the output header rather than a format string. Specifically unverified: whether `%CPU` is
  scaled by thread count (`-H` is passed), the exact meaning of `0%host`, and the denominator
  behind `400%cpu`. Every `top`-derived field therefore carries `semantics_verified: false`.
- **No per-UID CPU channel exists.** `/proc/uid_stat` does not exist in Android 15 — no producer
  or consumer in this tree — and `ProcessCpuTracker` derives per-UID CPU by summing per-process
  deltas from `/proc/<pid>/stat` (`:57-78, :353`). The task's "per-UID accounting" does not exist
  as a separate evidence source here. **No `per_uid[]` array was created.** The nearest thing is
  `am_uid_running` (68) / `am_uid_active` (64), which are *state* events, not time accounting.
- **`rate_per_s` gating is a hard-won lesson.** A first draft computed `121 / 64.662 = 1.873`
  and set `rate_valid: true`, which is wrong. The `EVENT LOG` window is 64.662 s, well under the
  120 s floor the field's own rule requires. It is recorded here because it is exactly the kind
  of error a "compute every available number" implementation makes silently, and exactly what
  the `matched_evidence` / `suppressed_noise` / `coverage_ratio` requirements exist to catch.
- **`busy_pct_since_boot` is pure derivation and its denominator is a construction.** busy
  (user+nice+system+iowait+irq+softirq) ÷ sum of all buckets including `guest_time_ms` and
  `guest_nice_time_ms`. Both guest buckets are 0 on this device. A first pass mis-added the
  buckets and shipped wrong values; the corrected values are what the worked example carries.
  AOSP publishes only raw milliseconds (`CpuInfoReader.java:859-880`) and never a busy
  percentage, so **any implementation computing it needs a unit test against a hand-summed row.**
- **`per_cpu_core[].is_online` and the frequency fields** are parsed from a C++ `CpuInfo{…}`
  `toString` whose formatter was not located in this drop. The field names are read off the
  output.
- **The ART monitor threshold depends on `kIsDebugBuild`, not on `ro.build.type`.** A 1000 ms
  floor on this `userdebug` build. Whether a `user` build of the same image would show a 100 ms
  floor **was not verified**, and since `lock_profiling_threshold_` is zero on user builds the
  whole branch is skipped anyway (`monitor.cc:458`) — so on a `user` build this channel emits
  nothing at all, and `channel_present: false` is the correct status there.
- **`is_debuggable` flags were raised by the Build System Developer and not cross-checked here.**
  The related, verifiable fact is that `ro.debuggable` and `ro.secure` are never consulted by
  dumpstate — see [§3.1](#31-isuserbuild-is-the-single-primitive).

---

## 5. Divergences Resolved

The five team deliverables were written in parallel and could not see each other. This section
records what was decided and **why**, so a future reader can see the reasoning rather than just
the outcome.

### 5.1 The D1–D5 decisions

| # | Question | Decision | Why |
| --- | --- | --- | --- |
| **D1** | Capture-instant key name | **`provenance.captured_at`**, RFC 3339 with an explicit offset (`Z` qualifies). Either the `== dumpstate:` banner or the archive filename may supply it; record which in `captured_at_time_base`. | The Technical Lead wrote `capture.captured_at_utc` derived from the **archive filename**; the Architect wrote `provenance.captured_at` derived from the **`== dumpstate:` banner**. Both are `ds.now_` from `dumpstate.cpp:3805` — **the same instant**. Renaming buys nothing, and `captured_at_time_base` exists precisely to record which source was used. The Tech Lead's nested `capture` wrapper was also flattened to `provenance` (that is D4). |
| **D2** | Freshness threshold | **3,600 s (1 hour)**, and the cross-boot escalation is **mandatory, not advisory**. | The Platform Developer proposed 24 h. On the reference archive both give the same answer — the gap is 126 days, so anything from 1 min to 1 day separates them — so neither is *wrong* on this evidence. They diverge on a 3-hour-old artifact. **The only distinction that must be made is "same boot" vs "previous boot", and the previous boot here is 8 h 39 m of uptime away** (`cross_boot_reference = 31190738`). A 1 h threshold catches that; 24 h would not and would have labelled the stale ANR set current. |
| **D3** | Process identity | **`process_key = "<boot_epoch>/<start_seq>"`** as primary. Fall back to the Architect's `"<process_name>#<uid>@<start_ordinal>"` **only** when `start_seq` is absent (a `dumpsys`-only sighting). `pid` is an observation attribute, never a key. | The App Developer's form is strictly stronger where available. `start_seq` is **AMS's own monotonic counter** (`ProcessList.java:2155-2157`), so it is globally unique for the life of the AMS process — and **AOSP itself uses it as the instance identity** (`AMS:930`, `:4475`, `:4814`). The Architect's ordinal is computed by us, is window-local, and is `null` for a process that started before the window. `start_seq` must be paired with `boot_epoch` because it resets to 0 on a `system_server` restart (`ProcessList.java:1858`) — which is exactly what `system.json#/capture/boot/at` supplies. |
| **D4** | Provenance nesting | **Flat `provenance` object on every finding**, not nested under a `capture` wrapper. | The Technical Lead nested it under `capture.*`. Per-finding provenance is the whole point; a wrapper that repeats capture context on every finding invites exactly the "one derived value, two answers" problem rule 6 of [§2.9](#29-null-absent-and-raw) forbids. Every other key in the wrapper has a 1:1 counterpart in the flat block, so **no data is lost** by flattening. |
| **D5** | Time-base enum | **Keep BOTH**: `captured_at_time_base` (the 11-value enum) *and* `capture_kind` (the measurement-kind enum). | The Architect's D5 recommendation was to emit one alongside the other, and the Platform Developer's proposed-changes section made the same argument independently. **They answer different questions.** The 11-value enum is what makes the year-less-logcat and cross-boot cases nameable; `capture_kind` is what distinguishes `persisted_aggregate` (which catches the 128-day-old `procstats` blocks) from `background_sampler` (which rescues `dumpsys cpuinfo`). Collapsing either into the other loses a distinction that turned out to be load-bearing in practice. |

### 5.2 Corrected facts

Several team files and one earlier brief carried wrong versions of these. All are measured, not
inferred, and each is carried into the spec above.

| Fact | Earlier claim | Correct value |
| --- | --- | --- |
| Crash count | 274 | **271 distinct crashes.** The 3 extras are dropbox-inlined copies of logcat (`ActivityManagerService.java:9890-9900`). Decomposition: 38 in the first `SYSTEM LOG`, **3** in `DROPBOX SYSTEM APP CRASHES`, 233 in the trailing `SYSTEM LOG`. |
| PID recycling | ~3 cycles in 274 crashes | 271 crashes → **271 distinct monotonic PIDs** (1116 → 9985), no recycling *within* the loop. The earlier reading conflated the PID range span with the crash count and mistook the dropbox duplicates for kernel recycling. |
| `dumpsys procrank` | absent | **Present and populated** — archive line 192650, 178 rows. It is a `bugreport_procdump` sub-section, **not** a dumpsys section (`dumpstate.cpp:1693-1694` → `smapinfo.cpp:1233-1242`). |
| CPU breakdown | 9.2 user / 17k *nice* | `17% **kernel**`. Nice is `12%` in the `top` line (archive line 57472). |
| Swap snapshots | 2 (18,688 K → 230,912 K) | **4:** 18,688 / 18,176 / 57,088 / 230,912 K. The two intermediate readings are what make the collector-induced growth visible. |
| `Cmd line:` | from AMS | from `art/runtime/signal_catcher.cc:55-69`, chosen to match debuggerd's format. **All 251 hits are inside VM traces** (184 in `VM TRACES JUST NOW`, 67 in `VM TRACES AT LAST ANR`) — the ANR team's evidence, not `apps.json`'s. |
| `Current user ids: [10]` | the device user list | A **SystemUI app-op monitor** string at archive line 1957. The user list is `dumpsys user`: `Current user: 10` at line 613374, `UserInfo{0:null:813}` / `UserInfo{10:Driver:412}` at 613377 / 613416. |
| Car packages | 13 | 13 is the count of `com.android.car.*` **processes** (`ProcessRecord.java:1396`); the package dump has **100+** distinct `com.android.car.*` packages. Both are stored, as `car_service_process_names` and `car_service_packages`. |
| Per-package entries | 660 | **282 `Package [` blocks** + **594 per-user state blocks** measured in the `DUMP OF SERVICE package:` section. The expected arithmetic `282×2 + 14×2 = 592` falls two short — see [`system.json`](#42-systemjson) open questions. |
| `DumpState.java` | the section registry | **Does not exist in Android 15.** The registry moved to `ServiceManager.addService(...)` in `ActivityManagerService.setSystemProcess()` (`:1926-1937`); the native `dumpsys` resolves names by asking the service manager (`dumpsys.cpp:261`). |
| `am/ProcessStats.java` | — | is now **`am/ProcessStatsService.java`**. Any citation of `am/ProcessStats.java` is stale. |
| `anrd` | the ANR detector | A **CPU-starvation ftrace collector** (`system/extras/ANRdaemon/README:1-14`; `ANRdaemon.cpp:236-260`). It declares no ANR, writes no `/data/anr/` entry, **and is not built for this product**. Contributes no field. |
| ANR trace timestamps | "none inside the trace" | The trace **does** carry them — `backtrace.cpp:49`, ns + tz, 31 `----- pid N at <ts> -----` lines. **Staleness here is provable, not inferred.** |

### 5.3 Design decisions this document made beyond D1–D5

Places where merging required a judgement the contract did not dictate:

| # | Decision | Rationale |
| --- | --- | --- |
| M1 | **`source` carries no time.** The capture instant lives in `capture`; evidence times live in `provenance`. | The Architect's `source` block held `captured_at`, `uptime_at_capture_seconds`, and `captured_at_time_base` alongside `archive_sha256`. Splitting them keeps *archive identity* and *capture instant* separable, which is the same separation as `provenance.captured_at` vs `source.captured_at`. |
| M2 | **`collection` is a top-level envelope key with one schema in all six JSON artifacts.** | The Platform Developer recommended a cross-artifact ledger and the contract adopted it. It is the only mechanism that answers "is this number self-inflicted?" from one artifact read in isolation, and it expresses *both* directions with the same `collector_perturbed` / `collector_perturbed_basis` pair. |
| M3 | **`anr.json` drops the `timeout_ms` field.** | `DEFAULT_ANR_TIMEOUT` and `DEFAULT_INPUT_ANR_TIMEOUT` do not exist anywhere in `android-15.0.0_r17`. The input-dispatch timeout is a reason *string* only (`InputManagerCallback.java:350-351`). A field that cannot be sourced does not go in the spec. |
| M4 | **`memory.json` drops `zram_orig_size_kb` and `zram_compressed_size_kb`.** | `SysMemInfo::MemZramDevice` skips `mm_stat` fields 1 and 2 and takes `mem_used_total` (`sysmeminfo.cpp:178-206`). No bugreport output exposes the compressed/original sizes, so the fields would be permanently unpopulatable. A derived `compression_ratio` with `*_derived: true` is legitimate. |
| M5 | **`cpu.json` keeps `utilization` and `contention` separate, and `utilization` and `per_cpu_core` separate.** | They differ by *units* and *window*, not by judgement. A `windowed ratio`, a `cumulative-since-boot millisecond counter`, and a `blocking-event count over the log window` are three different quantities. Merging any two produces a number that means nothing. |
| M6 | **`dvm_lock_sample` is `sampled_event_count`, not `event_count`.** | `Monitor::LogContentionEvent` fires with probability proportional to wait duration (`monitor.cc:529-536`). The name is the warning: a consumer who sees `sampled_event_count` cannot mistake 168 for a contention count. |
| M7 | **`contention.art_rate_limiting.counting_caveat` and `kill_evidence.am_low_memory_meaning` are mandatory prose fields.** | Two values that a reader will otherwise interpret the wrong way round. Carrying the interpretation as data makes it visible instead of implicit. |
| M8 | **`README.md` has no envelope; the seven-artifact list includes it.** | `requirements.md` lists it as an artifact. It is Markdown, references the JSON structures, and its "top-level keys" are its nine numbered sections. |

---

## 6. Known Limitations

What this specification deliberately does **not** specify, and why. Each entry is a decision,
not an omission.

### 6.1 Deliberately unspecified

| Limitation | Why |
| --- | --- |
| **A dumpsys section allowlist.** | Sections are resolved at runtime by binder-service enumeration (`dumpsys.cpp:261`, `dumpsys.cpp:250-274`). The reference archive holds ~250 distinct `dumpsys <name>` lines. **Any schema with a fixed section table is wrong by construction** — see [§3.4](#34-dumpsys-sections-are-an-open-set). |
| **A `userdebug` vs `eng` distinction.** | `PropertiesHelper::IsUserBuild()` is a literal compare against `"user"` and **both take the same branch**. There is no such distinction anywhere in dumpstate. |
| **A "capture succeeded" guarantee.** | `DropRootUser()` failure returns early from the whole dump (`dumpstate.cpp:1999`, `:2068`, `:2179`) and the archive still looks well-formed. The spec requires truncation to be *detectable*; it cannot make it impossible. |
| **Per-UID CPU accounting.** | `/proc/uid_stat` does not exist in Android 15. `ProcessCpuTracker` sums per-process deltas instead. There is no separate evidence source to model. |
| **An ANR `timeout_ms`.** | The constants do not exist in this tree and the reason string does not carry a duration (M3). |
| **ZRAM compressed/original sizes.** | Not exposed in any bugreport output (M4). |
| **A calendar or locale library.** | Timestamps are RFC 3339 with an explicit offset; anything locale-formatted is `*_raw` plus a parsed sibling plus `parsed_confidence`. The spec does not define the locale. |
| **Severity thresholds as tunable parameters.** | The five levels and their definitions are fixed in [§2.6](#26-severity). Making them per-run configurable would make artifacts incomparable, which defeats the point of a shared convention. |
| **The 3600 s threshold as a configurable value.** | Same reason. Fixed so artifacts are comparable across runs and tools. |
| **A per-line `evidence` for every field of every domain object.** | Evidence is one locator per finding, plus `corroborating_evidence[]`. Making it per-field would multiply the artifact size by two orders of magnitude for no additional traceability — the finding's `line_start`/`line_end` already spans the record. |
| **`README.md` byte-level layout.** | Its nine sections, their order, their mandatory/optional status, and the cold-boot and cross-boot banners are specified. Exact line breaks and table column widths are not; they are presentation, not contract. |

### 6.2 Known gaps in the evidence base

These are limits of what the reference archive and this AOSP tree can prove. They are listed
per artifact in [§4](#4-the-seven-artifacts) and collected here.

| Gap | Consequence |
| --- | --- |
| **`top` and `procrank` binaries are not in this source drop.** | Every `top`-derived field's semantics is unverified — whether `%CPU` is scaled by thread count, what `0%host` means, the denominator behind `400%cpu`. Only the *invocation* is citable. |
| **No `AM_CRASH` write site in `frameworks/base`, `system/`, or `packages/modules/`.** | `am_crash` slot order is decoded **by observation**, flagged in `channels[].decode_warning`. The tag is declared and emitted; the emitter was not found. |
| **No `dumpstate` section-size budget.** | None was found in `dumpstate.cpp`, `dumpstate.h`, `DumpstateUtil.{h,cpp}`, or `bugreportz.{cpp,h}`. The 64 MB main entry is **not** evidence of a cap. Timeouts are the only verified truncation mechanism. |
| **`dumpstate.strict_run` / `parallel_run` / `dry_run` / `unroot` were not verified on the device.** | Defaults are `dry_run=false`, `unroot=false`, `parallel_run=true`, `strict_run=true`. If any is overridden on the target, section presence and shape change for reasons unrelated to build type. |
| **~60 AIDL dumpsys sections unresolved.** | Each is a per-HAL `publishBinderService` in the owning HAL's source, and this product uses *example/mock* implementations. They are treated as open-set, as required. |
| **The archive's timezone offset is recoverable only indirectly.** | `localtime()` is used at `dumpstate.cpp:802`, `:2744`, and `DumpstateInternal.cpp:148`. The offset appears only in the `----- pid N at <ts>+0700 -----` lines. `ro.runtime.firstboot` disagrees with the AMS boot derivation by 6.5 s and is recorded but not preferred. |
| **Logcat has no year.** | The year is taken from `capture.captured_at` and the wrap heuristic is undetectable from inside the archive. A bugreport spanning 31 Dec → 1 Jan gets one year on all lines. Mitigated by `notes` `LOGCAT_YEAR_UNRESOLVED`, not solved. |
| **Uptime at capture is a derived figure, not a printed one.** | It resolves to **30 s ± 0.6 s** from the solved boot instant `B ≈ 00:26:47.33` ± 0.10 s. No archive-internal source states seconds-level uptime *directly*: the `== dumpstate:` banner is second-truncated, and `header_raw` / `section_raw` are minute-granular. The figure therefore rests on the derived bracket, and its ± 0.6 s uncertainty is dominated by the 1-second truncation of those wall clocks. (Two earlier figures, 27 s and 17.118 s, were both wrong — 27 s misread a wall clock as an elapsed time and 17.118 s was a process age. See [`system.json`](#42-systemjson).) |
| **`am_kill` `Reason` vocabulary and `Rss\|2\|2` decoding are spec-derived, not observation-derived.** | `am_kill` is zero on this archive, so neither could be validated against a real sample. |
| **`dumpsys usagestats` user coverage is unconfirmed.** | If it is user-0-only, every `activity_events[]` row describes the wrong profile on a multi-user device. This must be checked before `activity_events` is trusted. |
| **`ProcessStats` per-PSS-bucket window widths are unlocated.** | `aggregation_window` is built from printed fields, not a declared bucket width. |
| **`maybePruneOldTraces` did not run for 126 days on this device.** | Either the retention cap was raised by device config, or `if (i > max)` (`StackTracesDumpHelper.java:522`) leaves index 64. Unresolved which. It does not change the schema — it is a fact for `README.md` §8. |
| **The `SharedUser [` header printer was not isolated.** | `apps.json#/inventory/shared_uids[]` ids are read from the archive, not from a cited printer. |

### 6.3 Where the detail lives

The per-field tables in this document are **complete in coverage but condensed in rows**. The
owning team's exhaustive row-level field tables are preserved and were not edited:

| Owning file | Preserved detail |
| --- | --- |
| `tmp/aosp-team/01-architect-conventions-and-readme.md` | Part B field tables (§B.1–B.8), the full `captured_at_time_base` mapping table, the closed suppression-reason vocabulary with per-archive-instance counts, the `README.md` section-by-section specification, and Appendix A's AOSP citation index (~200 `path:line` pairs). |
| `tmp/aosp-team/02-anr-and-crashes.md` | §6 and §13 field tables, the six ANR channels and seven crash channels with completeness ratings, the `ANR FILES` listing verbatim, and §15's consolidated citation list. |
| `tmp/aosp-team/03-memory-and-cpu.md` | §4.4 and §5.4 field tables, the twenty memory channels (M1–M20) and eighteen CPU channels (C1–C18) with presence and window/basis, and §6's citation index. |
| `tmp/aosp-team/04-apps-and-system.md` | §1.2 and §2.4 field tables, the full `PackageEntry` / `ProcessEntry` / evidence-channel tables, the build-type dependency table, and §6's citation list. |
| `tmp/aosp-team/05-module-and-provenance-map.md` | §1's module → source path → installed-artifact table (including the three cases where the installed name differs from the module name), §2.2's gated-section table, and §3.2's section → emitting-source table. |

**Where this document and a team file disagree on a value, this document wins** — it is the
merged, binding specification. Where a team file carries a value this document does not repeat,
the team file's Appendix B or its worked example is the authority, and it was verified against
the reference archive during this merge.

