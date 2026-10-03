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
counts, severity_summary, freshness_summary, negative_evidence, evidence_gaps,
findings, notes
```

`README.md` is Markdown and has no envelope; it references these structures but does not
serialise them.

`system.json` is the one artifact that owns `capture`. In the other five, `capture` is
**replaced** by `capture_ref`, a pointer to the canonical object ([§2.9](#29-null-absent-and-raw)
rule 6). Both forms are counted in the thirteen, so every artifact has the same key count.

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

**On empty arrays.** `findings: []` is a legal and meaningful result — but it is not a
silent one. An artifact with no findings must carry at least one `negative_evidence` entry
or a `notes` entry explaining why there are none. See [§2.1.6](#216-negative_evidence).

**On the `findings` element shape.** Every element of `findings[]` carries the same core
fields, plus whatever its artifact defines:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | yes | `<artifact_prefix>-<4-digit ordinal>`, assigned in archive order so ids are stable across runs on the same archive. Prefix: `anr`, `crash`, `mem`, `cpu`, `app`, `sys`. Ids start at `0001`. |
| `kind` | string enum | yes | A noun for a record type, never a symptom. Artifact-specific; the closed set is listed in each artifact's entry. |
| `title` | string | yes | One line, ≤ 200 characters, no newline, no unescaped control character. **Removed from JSON artifacts.** A title is prose that summarises a finding, and prose is what `README.md` §4.1 is for. `README.md` §3 synthesises titles from `kind`, `severity`, `metrics`, and the evidence excerpt; no JSON artifact carries one, so a title can never contradict the numbers beside it. |
| `severity` | enum | yes | See [§2.6](#26-severity). Produced by the artifact's published `severity_rule`. |
| `unconstrained_severity` | enum | yes | The severity this finding would have absent the freshness cap. Always present. Equals `severity` when no cap applied. |
| `confidence` | enum | yes | `confirmed` (a validated record), `inferred` (a derived conclusion), `reported` (a third-party assertion we did not verify). `reported` **must** be used when the only evidence is an event-log line, and **must** be used when the only evidence is an `archive_absence` entry — an absence can never be `confirmed`. |
| `process_key` | string \| null | yes | See [§2.8](#28-identity). `null` only when the finding is genuinely device-wide. |
| `event_time` | string \| null | yes | When the **condition** happened. Distinct from when the evidence was captured. |
| `attributes` | object | yes | Record-type-specific **scalars only** — string, integer, number, or boolean. A nested object or array here is a schema violation; anything structured belongs in `metrics`. |
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
| `captured_at_time_base` | enum \| null | yes | Which clock supplied `captured_at`. `null` when `captured_at` is `null` — a clock cannot be named when no clock was found. See [§2.4](#24-provenance). | — |
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

`system.json` extends `scope` with a per-section `sections[]` array carrying a `status` from
[§2.2](#22-section-status-enum). Every other artifact inherits the section list from
`system.json#/scope` via `capture_ref` rather than repeating it.

#### 2.1.5 `collection`

Built from dumpstate's own per-section duration footers — the
`------ N.NNNs was the duration of 'TITLE' ------` lines emitted at
`dumpstate.cpp:3831-3835`. Same schema in every artifact.

The question this answers is narrow and important: **did the collection process change the
number it is reporting?** A schema that diffs two swap snapshots must be able to show the
reader whether any page-scanning section ran inside the interval between those snapshots, so
that a large delta is not read as device behaviour by default.

**What this section does not do is decide.** There is no `assessment` field, and there is no
statement-order field. Both would require claiming that a delta was *caused* by the collector,
and the archive records swap values and section durations with no per-process page-fault
accounting connecting them. The measurements are published; the causal reading belongs in
`README.md` §4.1, where it is explicitly labelled as prose.

> **There is no statement-order field because dumpstate has none.** It runs sections on a
> thread pool (`dumpstate_log.txt` records `Start thread pool:1` … `shutdown thread pool` …
> `Start thread pool:3`), and the logcat `Duration of 'TITLE'` lines show sections whose
> intervals overlap. Archive order is **completion** order under parallelism. Any field
> claiming to recover statement order is unproducible from any archive.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `page_scanning_sections` | array of object | yes | Sections whose body contains a per-PID row set carrying a swap column, or a `/sys/block/zram*/stat` block. Membership is decided by **what the section prints**, never by AOSP source knowledge; each entry carries `name`, `line_start`, `line_end`, `duration_ms`. |
| `page_scanning_total_ms` | number | yes | Sum of the above, from the per-section duration footers. |
| `swap_timeline` | array of object | yes | Swap readings across the dump, **ordered by `line_start`**. Every delta is an **edge between two identified nodes**; there is no scalar a consumer can diff naively. Each entry carries `line_start` pointing at the payload line the value was read from. |
| `swap_growth_kb` | integer \| null | yes | `last − first` across the timeline. `null` if fewer than two usable points. |
| `page_scanning_ms_in_interval` | number \| null | yes | Sum of the page-scanning section durations whose spans overlap the interval between the first and last timeline reading. **A measured overlap, not an attribution.** `null` when `swap_growth_kb` is `null`. |

Whether overlapping page scanning *explains* the delta is a claim about causation. An artifact
that made the claim would need per-process page-fault data the archive does not contain, so the
claim is not a field. Where a collector's influence is nonetheless directly checkable — the
`dumpsys cpuinfo` window closing before the dump began — it is carried per-provenance as
`collector_perturbed`, which §2.4 already requires.

#### 2.1.6 `negative_evidence`

A required array. Each entry:

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

**An empty `negative_evidence` is itself a defect in the artifact.** This array is the
mechanism that stops "65 ANR files on disk" from being read as "an ANR happened". The
reference archive contains 65 ANR trace files, all 126 days old, from a boot that ended
before this one — and simultaneously two dumpsys sections asserting in so many words that
no ANR has occurred since boot. Both facts are true. Only an artifact that carries both can
be read correctly.

#### 2.1.7 `evidence_gaps`

A required array in **every** artifact. It is the mechanism that lets a schema declare a limit
instead of inventing a value, and it is required precisely because its absence is what makes
fabrication attractive.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `field` | string | yes | Dotted path of the field or object the gap concerns. |
| `reason` | enum | yes | A code from the closed table below. |
| `detail` | string \| null | yes | Human-readable explanation. **Never load-bearing** — a consumer must not have to parse it, and a validator must never depend on its wording. |

`reason` is a **closed vocabulary**, not prose:

| `reason` | Meaning |
| --- | --- |
| `NOT_PRINNED` | The source field exists in the platform but the archive does not print a value for it. |
| `SECTION_ABSENT` | The section that would carry this was not in the archive. |
| `SECTION_FAILED` | The section is present but its command failed or timed out. |
| `REGION_EXCLUDED` | The section was read but the relevant span was outside the read region. |
| `PARSE_INDETERMINATE` | The bytes are present but do not determine the value. |
| `AMBIGUOUS_SOURCE` | Two or more archive sources disagree and nothing in the archive adjudicates. |
| `UNVERIFIABLE` | The value would require knowledge outside the archive — platform source, build configuration, or live device state. |

**The distinction that matters:** `evidence_gaps[]` is about *the archive*. It says "these bytes do
not determine this value". It is **not** about the analyzer. A field the tool failed to parse is
a bug in the tool, fixed in the tool; it does not become a gap in the artifact. And `UNVERIFIABLE`
is a legitimate, frequent value: a schema that has no `UNVERIFIABLE` entries has usually hidden a
platform-knowledge assumption in a field instead.

> **An empty `evidence_gaps` array is a claim.** It asserts that every field this artifact
> publishes was derivable from this archive. That is almost never true, so an empty array should
> be as rare as a `findings: []` with no explanation.

#### 2.1.8 `notes`

Free-text caveats, each prefixed `"<CODE>: <text>"`. Notes are the escape hatch for a caveat
that does not attach to any one field.

`notes[]` is **not** where platform knowledge should move to. A note saying "the tombstone
index is a ring counter, so `tombstone_00` is not the newest file" is a statement about AOSP
that the tool cannot verify per archive; it belongs in this specification's prose, where a
reader can see it once, rather than in every artifact as an un-actionable string. Notes carry
facts about *this run* — a truncation was hit, a clock skew was seen, a duplicate rendering was
found.

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
| `age_seconds_at_capture` | integer \| null | yes | `capture.captured_at − provenance.captured_at`. `null` if either side is `null`. Never negative; if the arithmetic goes negative, set `0` and emit `notes` `CLOCK_SKEW`. |
| `capture_kind` | enum | yes | **Required, not optional.** See below. |
| `freshness` | enum | yes | See [§2.5](#25-freshness). |
| `freshness_basis` | object | yes | `{ input, basis, threshold_seconds }`. All three required. |
| `cross_boot_reference` | integer \| null | yes | Milliseconds of uptime on a **different boot epoch**. `null` otherwise. |
| `collector_perturbed` | boolean \| null | yes | Did the collection plausibly change this number? |
| `collector_perturbed_basis` | enum \| null | yes | **Now a closed code, not a sentence.** `section_measured_before_dump`, `section_measured_during_dump`, `read_from_disk`, `live_log_capture`, `undated_source`. A tool picks the code by comparing the evidence's own clock against `capture.captured_at`; where that comparison is impossible because the evidence is undated, the value is `null` and an `evidence_gaps[]` entry records it. The prose form was unusable: it either restated what the code already said, or it asserted a mechanism ("the dumpstate pass did not generate these") that the archive cannot confirm. |

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

**Each artifact must state its severity rule, and that rule is what the artifact implements.**
The five definitions above are prose, not a mapping, and a prose definition is not
machine-writable. §4.2–§4.7 therefore each carry an explicit `severity_rule` table: a closed
set of machine-checkable conditions over the artifact's own fields, each naming the metrics
that satisfy it. A finding's `severity` is the highest level whose rule fires. Two consequences
follow, and both are deliberate:

- A finding that matches **no** rule is a schema defect, not an `info`. It must be raised
  against the rule table, not emitted with a default severity.
- `info` is reachable **only** by a rule that states what fact about the archive or its
  absence is being reported — never as a fallback.

`severity` and `confidence` are the only two judgment fields any artifact carries. `title` is
not one of them: it is narrative, it is generated in `README.md` §4.1, and no JSON artifact
carries it.

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
| `locator.line_start` / `line_end` | integer \| null | yes | 1-based inclusive. Required for `kind == "line_range"` **and** `kind == "section"` — a `------ … ------` section has a definite span in the file. `null` for `file_entry`, `property`, and `record`, which are not line-bounded. |
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
4. **Every derived number carries its derivation — unless the formula is already in the
   schema.** If a value is computed rather than read, the schema must state the formula, and
   that statement *is* the derivation. A sibling `*_derivation` string is emitted only when the
   formula **cannot** be re-derived from the field table alone (for example when it depends on
   a value the artifact does not carry). Prose that restates a formula already written down is
   noise, and prose that adds an interpretation beyond the formula is fabrication.
5. **Zero counts carry a match block, never an excuse.** A count of `0` is published with its
   `pattern`, `coverage`, and `regions[]` per [§2.3](#23-counts-are-match-blocks), and nothing
   else. **No artifact carries an `*_absence_reason` key.** Explaining *why* a count is zero
   requires a cause the archive does not record — whether the platform omitted it, a build
   gate suppressed it, or the analyzer failed to look. Only the third of those is knowable, and
   it belongs in the generator's own diagnostics, never in the artifact. A tool that has not
   finished looking has not produced a finding.
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
| `incremental` | string \| null | yes | The incremental field of `display_id`. | `display_id` |
| `version.release` / `version.sdk` | string / integer | yes | The release name and API level. | `dumpstate.cpp:797,818` |
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
| `name` | string \| null | yes | `ro.product.name`. | `getprop` |
| `device` | string \| null | yes | `ro.product.device`. **Differs from `name`** — the `Build:` header carries one, the fingerprint the other. Store both, never one from the other. | `getprop`; `dumpstate.cpp:808,810` |
| `brand` / `manufacturer` / `model` | string \| null | yes | `ro.product.*`. | `getprop` |
| `first_api_level` | integer \| null | yes | `ro.product.first_api_level`. | `getprop` |
| `characteristics` | array of string | yes | `ro.build.characteristics`. On the reference archive the only *in-band* statement that the device is automotive. | `getprop` |
| `cpu_abi` / `cpu_abilist` | string \| null | yes | `ro.product.cpu.abi` / `.abilist`. | `getprop` |

##### `device`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `is_emulator` | boolean \| null | yes | `true` when any `is_emulator_basis` entry matched a published rule; `false` when none matched and every basis property was readable. **⇒ `null` on a `user` build, never `false`** ([§2.9](#29-null-absent-and-raw) rule 2). Rules `R1`–`R3` below. | `getprop` |
| `is_emulator_basis` | array of object | yes | One entry per property actually read: `{property, value, matched_rule}`. `matched_rule` is `R1`/`R2`/`R3` or `null`. Empty when `is_emulator` is `null`. | `getprop` |
| `avd_name` | string \| null | yes | `ro.boot.qemu.avd_name`. | `getprop` |
| `serial_no` | string \| null | yes | `ro.boot.serialno`. | `getprop` |
| `boot_hardware` | string \| null | yes | `androidboot.hardware` from `/proc/bootconfig`. **Available on all builds.** | `dumpstate.cpp:826-827` |
| `bootconfig_excerpt` | object | yes | The small set of keys that change device class: `boot_devices`, `hardware.vulkan`, `qemu`, `qemu.avd_name`, `qemu.skin`, `qemu.vsync`, `veritymode`, `vbmeta.digest`, `vbmeta.hash_alg`, `vbmeta.size`, `logcat`. | `dumpstate.cpp:826-827` |
| `cmdline_has_kvm_arm_protected` | boolean \| null | yes | Derived from `/proc/cmdline`. | `dumpstate.cpp:825` |

> **The `is_emulator` rule is fixed by this schema, not learned per archive.** A tool implements
> the table below and nothing else; it does not decide "does this look like an emulator" per run.
> The rule is small and published on purpose, because the alternative — an unstated heuristic —
> is exactly the kind of judgment this schema exists to exclude.
>
> | Condition | Property | Value | Effect |
> | --- | --- | --- | --- |
> | `R1` | `ro.hardware` | `ranchu` | `is_emulator: true` |
> | `R2` | `ro.kernel.qemu` | `1` | `is_emulator: true` |
> | `R3` | `ro.boot.qemu` | `1` | `is_emulator: true` |
>
> `true` if any of `R1`–`R3` matched; `false` only if all three properties were readable and none
> matched; `null` if `getprop` was gated. No other rule may set this field.

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
| `current_user_id` | integer | yes | The `Current user: N` value. | `UserManagerService.java:7311-7317` |
| `list[]` | array of object | yes | One per `UserInfo{…}` line. | `UserManagerService.java:7322-7331` |
| `list[].id` | integer | yes | The user id. | `UserManagerService.java:7464` |
| `list[].name` | string \| null | yes | The profile name, `null` where the `UserInfo` carries none. | `UserInfo.java:528-533` |
| `list[].flags` | array of string | yes | Parsed from the symbol list, not the number. | `UserManagerService.java:7483-7485` |
| `list[].flags_hex` | integer | yes | The hexadecimal `UserInfo{…}` third field, parsed as hex. Store it verbatim alongside `flags`; never convert one into the other — the two use different conventions and conflating them produces two wrong numbers. | `UserInfo.java:532` |
| `list[].is_primary` | boolean | yes | From the `PRIMARY` flag symbol. | `UserManagerService.java:7466` |
| `list[].parent_id` | integer \| null | yes | `null` when no parent line is printed for this `UserInfo`. **No reason is published for the `null`** — whether the platform omits it for a full secondary user or for some other reason is AOSP behaviour the analyzer cannot confirm from the archive. Recorded in `evidence_gaps[]` as `UNVERIFIABLE`. | `UserManagerService.java:7467-7470` |
| `list[].type` | string | yes | The printed `Type:` line verbatim. | `UserManagerService.java:7482` |
| `list[].state` | string | yes | The printed `State:` line verbatim. | `UserManagerService.java:7486-7492` |
| `list[].serial_no` | integer | yes | The serial number from the `UserInfo` header. | `UserManagerService.java:7465` |
| `list[].start_time_ago` | string \| null | yes | The `+NmSmmm`-form "ago" string — an **uptime-relative** value, not a wall clock. Same clock as `ProcessRecord.startUpTime`. | `UserManagerService.java:7495-7497` |
| `list[].last_logged_in_ago` / `unlock_time_ago` / `last_entered_foreground_ago` | string \| null | yes | Same encoding and same clock. | `UserManagerService.java:7499-7510` |
| `list[].created_ago` | string \| null | yes | The creation "ago" string. `<unknown>` is a **literal in the archive**, normalised to `null` here per [§2.9](#29-null-absent-and-raw) rule 1. | `UserManagerService.java:7493-7494` |
| `cross_check` | object | yes | **Reduced to raw counts only.** `package_dump_user_ids` (array of integer), `package_install_state_blocks`, `shared_user_blocks`, `shared_user_state_blocks`, `other_user_header_blocks`. All five are literal counts over the `DUMP OF SERVICE package:` section. **The `arithmetic` prose and the `agrees_with_dumpsys_user` verdict are removed** — a reconciliation sentence and a conclusion are judgment, and neither is needed for a consumer to see that the counts do or do not line up. |
| `cross_check` gap | — | — | Where the five counts do not close, the shortfall is published as an `evidence_gaps[]` entry with `reason: AMBIGUOUS_SOURCE` naming which blocks were unclassified. The tool does not choose a story that makes them close. |

##### `displays[]`, `hal`, `storage`

| Object | Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- | --- |
| `displays[]` | `display_id` | integer | yes | The SurfaceFlinger display handle. | SurfaceFlinger CRITICAL dump, `dumpstate.cpp:1382` |
| | `hwc_display` / `port` | integer | yes | The HWC display index and the output port. | same |
| | `pnp_id` | string | yes | The plug-and-play id, `GGL` on a SwiftShader/emulated target. | same |
| | `display_name` | string | yes | The display name, `EMU_display_N` on an emulator. The only stable handle for an output. | same |
| `hal` | `hidl_services[]` | array of object | yes | Rows of `lshal --all --types=all`: `interface::instance`, `transport`, `server_pid`, `arch`, `threads_used`/`total`, `registered`. | `dumpstate.cpp:1421-1423` |
| | `hidl_debug_entries[]` | array of string | yes | ZIP entries under `lshal-debug/`, one per interface whose `lshal debug -E` produced non-empty output within 2 s. | `dumpstate.cpp:1434-1466` |
| `storage` | `mountinfo_pids` | array of integer | yes | PIDs whose `/proc/<pid>/mountinfo` was read. **A PID appearing here was alive at capture** — a cheap independent liveness signal. | `dumpstate.cpp:498-504` |
| | `archived_trees` | array of string | yes | Directory prefixes copied under `FS/`. | `dumpstate.cpp:1950-1952,1983,948,1106` |
| `storage` | `anr_files_archived` | array of object | yes | `path`, `mtime`, `bytes` per `FS/data/anr/anr_*`. **Freshness lives here.** `system.json` records the files as *device state*; `anr.json` decides staleness. | `dumpstate.cpp:1931-1933` |
| | `tombstone_count_archived` | integer \| null | yes | Count of tombstone ZIP entries. `crashes.json` owns their content. | `dumpstate.cpp:1931` |
| | `dynamic_partition_section_status` | enum | yes | The [§2.2](#22-section-status-enum) status of the `LPDUMP` section. **Not** a boolean: the section can be present *and* have failed (`*** command 'lpdump --all' failed`), and a boolean cannot express that. | `dumpstate.cpp:1953` |

##### `archive_entries`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `proto[]` | array of object | yes | `path`, `bytes`. **`bytes: 0` is normal, not corruption** — `RunDumpsysProto` adds an entry for every service claiming proto support even if the dump wrote nothing. | `dumpstate.cpp:1340-1380` |
| `proto[].status` | enum | yes | `ok` or `declined`. `declined` ⇔ `bytes == 0`. | same |
| `proto.suffix_meaning` | object | yes | **Removed.** The `_CRITICAL` / `_HIGH` / `""` suffix scheme is a property of dumpstate's priority selection, not of any archive. A tool that needs it reads it from here, in prose; it never emits it as data. |
| `fs_paths` | array of string | yes | Every `FS/**` entry. | `dumpstate.cpp:375` |
| `fs_user_build_gated` | array of string | yes | **Removed.** The gated-path *list* is AOSP control flow. Presence or absence of a given path on a given archive is already visible in `fs_paths`; the enumeration of which paths *could* be gated is not a per-run observation. |
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
| `status` | enum | yes | See [§2.2](#22-section-status-enum). | — |
| `start_line` / `end_line` | integer | yes | Line span in the main entry. Enables re-reading without rescanning. | — |
| `duration_seconds` | number \| null | yes | From the paired duration trailer. A long duration with no content means a timeout. | `dumpstate.cpp:3831-3835` |
| `note` | string \| null | yes | **Removed.** A free-text "why the status is what it is" is prose about a collector the analyzer cannot inspect. The `status` enum from [§2.2](#22-section-status-enum) already carries the machine-readable reason; where that is not enough, the fact belongs in `evidence_gaps[]`. |

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

Abbreviated from the reference archive. **Values are placeholders.** The reference archive is a
worked fixture, not the specification: a concrete fingerprint, serial, package name, or timestamp
would date this document and mislead anyone reading it as a schema. What is preserved is the
*shape* — field names, types, nesting, enum membership, and the parts that carry a rule
(the closed-set membership, the `[<lower>, <upper>]` bound form, the `"…"` elision).

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
    "zip_entry_count": 398,            // read from the ZIP central directory
    "device": {
      "product_name": "<product_name>",
      "build_id": "<build_id>",
      "build_display_id": "<product>-<type> <release> <build_id> <incremental> <tags>",
      "build_fingerprint": "google/<product>/<device>:<release>/<build_id>/<incremental>:<type>/<tags>",
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
    "fingerprint": "google/<product>/<device>:<release>/<build_id>/<incremental>:<type>/<tags>",
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
        "threads_used": <n>, "threads_total": <n>, "registered": true },
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
    "proto_declined_count": <count>,
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
    { "field": "capture.log_window.truncated", "reason": "NOT_PRINNED",
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

- **Uptime at capture — resolved, by derivation rather than by reading.** No archive-internal
  source states seconds-level uptime directly, so the published value rests on a solved boot
  bracket: several `(wall clock, uptime)` pairs are read from inside sections that also print
  their own end time and duration, and the kernel `Booting Linux` message fixes an upper bound.
  The artifact publishes `uptime.bounds_seconds` as an interval rather than a point estimate
  with an error bar, and records the limit as an `evidence_gaps[]` entry with
  `reason: PARSE_INDETERMINATE`.
  **What remains genuinely open:** the bounds are dominated by the 1-second truncation of the
  `== dumpstate:` banner clock and of the `uptime` wall clocks in `header_raw` / `section_raw`.
  A future run that can read `/proc/uptime` or an equivalent seconds-precision source should
  replace the derivation with a direct read and drop the gap.
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
| `incomplete_reason` | enum \| null | yes | Why the channel cannot cover the log window. Closed set: `event_log_buffer_retention`, `gated_by_user_build`, `permission_denied`, `declined_by_service`. **`not_emitted_by_platform` is removed** — "the platform never produced this" is a claim about dumpstate's emission contract, not an archive observation. A channel that is `present: false` and has no window simply has `coverage_ratio: null`; where a consumer needs to know the section was *expected*, that is `scope.sections_not_found` in `system.json`. |
| `snapshot_at` | string \| null | yes | For point-in-time channels, when the dump ran. |
| `blocks[]` | array of object | no | For channels split across duplicate sections: `{ index, command, line_start, line_end }`. `index` disambiguates the two `SYSTEM LOG` blocks. |
| `carries` | array of string | yes | What the channel contains. Where a count appears it is the **channel-wide** count, never a filtered subset; a filtered subset belongs in `inventory`, where the filter is visible. |
| `present` when false | — | — | A channel with `present: false` carries no window and no `coverage_ratio`. Every key is still present, `null` per [§2.9](#29-null-absent-and-raw) rule 1, and the gap is recorded in `evidence_gaps[]` with `reason: SECTION_ABSENT`. |

##### `inventory.counts`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `packages` | integer | yes | `Package [` block count. | `Settings.java:5118` |
| `processes` | integer | yes | `*APP*` + `*PERS*` record count. | `ProcessList.java:4454-4477,4469` |
| `processes_persistent` / `processes_app` | integer | yes | The two record kinds, summing to `processes`. | `ProcessList.java:4469` |
| `distinct_process_names` | integer | yes | Distinct `process_name` values across `processes[]`. | derived |
| `installed_packages_with_live_process` | integer | yes | Distinct package names appearing in `inventory.index.process_name_to_packages` ∩ installed packages. **Renamed from `packages_with_live_process`**: it is a *package* count, and the previous name invited exactly the confusion with `distinct_process_names` that this artifact exists to prevent. A consumer can compute the coverage ratio as this ÷ `packages`. | derived |
| `users` | integer | yes | `UserInfo` count. | `UserManagerService.java:7311-7331` |
| `shared_uids` | integer | yes | `SharedUser [` block count. | `Settings.java` shared-user dump |
| `package_user_state_blocks` | integer | yes | `User N:` block count inside the package section. | `Settings.java:5471-5510` |
| `lru` | object | yes | `{ total, non_activity, non_service }`, read verbatim off the LRU header. | `ProcessList.java:4323-4331` |
| `car_service_process_names` | integer | yes | Distinct process names under the automotive-service prefix. | `ProcessRecord.java:1396` |
| `car_service_packages` | integer | yes | Distinct package names under the same prefix. Stored alongside `car_service_process_names` because the two are **not** equal, and the difference is the signal. | `Settings.java:5118` |
| `lifecycle_counts` | object | yes | **Removed as a bare-integer object.** Per [§2.3](#23-counts-are-match-blocks) no artifact may report a bare number, and this one was the clearest violation: five integers with no `pattern`, no `raw_count`, no `coverage`, and no `suppressed_noise`. Each count is now a `counts.am_proc_*_hits` match block, and the event-log window they were measured over is carried once on `lifecycle_channel_window`. | — |
| `lifecycle_channel_window` | object | yes | The event-log buffer's own window and coverage. See below. | `dumpstate.cpp:1028` |
| `sources_available` | array of string | yes | Evidence sources found in this archive. | — |
| `sources_absent` | array of string | yes | Evidence sources searched for and not found. **The absence is the observation; no reason is published for it.** Where the reason is known from the build (a `user`-build gate) it is recorded as an `evidence_gaps[]` entry with `reason: SECTION_ABSENT`; where it is a platform property no archive records, `reason: UNVERIFIABLE`. | — |

`lifecycle_channel_window` is load-bearing: it is the single place a reader learns that the
event-log counts cover a fraction of the log window. Every count that depends on it must carry
its own `coverage` block too, per invariant 2 of [§2.3](#23-counts-are-match-blocks).

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `first_event_at` / `last_event_at` | string \| null | yes | The first and last `am_proc_*` stamp. `null` when the channel is empty. |
| `duration_seconds` | number \| null | yes | Window length. `null` when fewer than two events. |
| `covers_full_log_window` | boolean | yes | `duration_seconds >= capture.log_window.duration_seconds`. |
| `coverage_ratio` | number \| null | yes | `duration_seconds ÷ capture.log_window.duration_seconds`. **`null` when the window is empty.** |
| `section_titles` | array of string | yes | Which sections the events were found in, with `section_ordinal` disambiguating repeats. Replaces `all_in_first_system_log_block`: that boolean encoded a claim about *where* events sit that is wrong on the reference archive — every `am_proc_*` line lives in the `EVENT LOG`, not in either `SYSTEM LOG` — and a boolean cannot be corrected without rewriting it. Naming the sections is both true and self-correcting. |
| `sections_scanned` | array of object | yes | One entry per duplicate section that was searched: `{title, ordinal, start_line, end_line, matched}`. This is what makes the §2.10 Trap 1 de-duplication checkable rather than assumed. |

##### `inventory.packages[]` — `PackageEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `package_name` | string | yes | **Primary key.** | `Settings.java:5118-5122` |
| `identity_hash` | string | yes | The parenthesised `System.identityHashCode`. **Not an identity**; per-boot and allocator-dependent. Stored so a human can cross-reference a section. | `Settings.java:5120-5122` |
| `app_id` | integer | yes | **Join accelerator only** — reassigned on reinstall, and shared (`android.uid.system` apps all share one `app_id`). | `Settings.java:5129` |
| `shared_uid_name` | string \| null | yes | `android.uid.system`. **This is what makes `app_id=1000` interpretable.** | `Settings.java:5131-5133` |
| `version_name` | string \| null | yes | | `Settings.java:5178` |
| `version_code` / `min_sdk` / `target_sdk` | integer | yes | | `Settings.java:5154-5160` |
| `code_path` / `resource_path` | string | yes | | `Settings.java:5136,5141` |
| `code_path_prefix` | enum | yes | Derived: `/data/app`, `/system`, `/system_ext`, `/product`, `/vendor`, `/apex`, `other`. **Every prefix maps to a `tier`** — the table below is exhaustive by construction, so no package can be left unclassified. | derived from `Settings.java:5136` |
| `tier` | enum | yes | `shared_system` / `system` / `product` / `vendor` / `apex` / `third_party`. **A label, never a filter.** Computed from `code_path_prefix` and `shared_uid_name` by the table below — there is no other input and no fallback judgement. | derived |
| `tier_basis` | object | yes | **A fixed-length array of `(field, value)` pairs, not prose.** The previous form was a sentence, which meant the reader had to trust a natural-language gloss to recompute the label. Now `tier` is mechanically derivable from `tier_basis` plus the table, and a validator can assert they agree. | derived |
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
| `user_id` | integer | yes | The user this install-state block belongs to. | `Settings.java:5471` |
| `installed` | boolean | yes | The printed `installed=` flag. | `Settings.java:5476-5478` |
| `data_dir` | string \| null | yes | `null` when not installed. | `Settings.java:5503-5505` |
| `first_install_time` | string \| null | yes | The printed timestamp, **normalised to `null` when `installed == false`**, because the platform then prints a fixed epoch-era sentinel rather than a real time. | `Settings.java:5507-5509` |
| `first_install_time_raw` | string | yes | The printed value verbatim, always. The normalisation rule keys off the same-line `installed` flag, which *is* archive-observable, so `first_install_time_sentinel` is unnecessary — the previous field required recognising a hardcoded platform literal, and `first_install_time_raw` lets a consumer see the bytes the decision was made on. |
| `enabled` | integer | yes | `0` = `COMPONENT_ENABLED_STATE_DEFAULT`. | `Settings.java:5488-5489` |
| `stopped` / `not_launched` / `hidden` / `suspended` / `quarantined` | boolean | yes | | `Settings.java:5479-5493` |
| `ce_data_inode` / `de_data_inode` | integer \| null | yes | **`null` when `installed == false`**, because a not-installed user prints `0` for both and `0` is indistinguishable from a real inode. The `installed` flag on the same object is what distinguishes them; no separate reason field is needed. | `Settings.java:5472` |
| `install_reason` / `install_reason_name` | integer / string | yes | `0` = `INSTALL_REASON_UNKNOWN`. | `Settings.java:5500-5501`; `PackageManager.java:1920` |

##### `inventory.processes[]` — `ProcessEntry`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `process_key` | string | yes | **Primary key.** `"<boot_epoch>/<start_seq>"`. | `ProcessList.java:2155-2157` |
| `start_seq` | integer | yes | The `ProcessRecord` start sequence. | `ProcessRecord.java:540` |
| `record_kind` | enum | yes | `persistent` / `app`, from `*PERS*` / `*APP*`. | `ProcessList.java:4469` |
| `record_identity_hash` | string | yes | Diagnostic only. A hash that stays constant across a crash loop is a cheap cross-check that the platform reuses the record. | `ProcessRecord.java:1402-1403` |
| `pid` | integer | yes | **Observation attribute, not identity.** | `ProcessRecord.java:517-518` |
| `process_name` | string | yes | The *process* name, not the package name. | `ProcessRecord.java:471-472` |
| `runtime_key` | string | yes | `"<user_id>:<process_name>"`. **Not unique** — many PIDs share one `runtime_key`. | derived |
| `attribution_key` | string | yes | `"<primary package>/<user_id>"` — the human-facing identity. | derived |
| `user_id` / `uid` | integer | yes | The user and uid from the record. | `ProcessRecord.java:465-466` |
| `uid_formatted` | string | yes | `u10a250` style. Decoding: `uid = user * PER_USER_RANGE(100000) + (appId + FIRST_APPLICATION_UID(10000))`; a trailing `iN` is an isolated uid offset. | `ProcessRecord.java:1378-1391` |
| `app_id` | integer | yes | | `UserHandle.java:402` |
| `gids` | array of integer | yes | | `ProcessRecord.java:469-478` |
| `isolated_uid` | integer \| null | yes | Present only when `uid != info.uid`. | `ProcessRecord.java:467-469` |
| `packages[]` | array of string | yes | From `packageList={…}`. **The authoritative process→package link.** | `ProcessRecord.java:508-509` |
| `package_dependencies[]` | array of string | yes | `packageDependencies={…}`. Rare but real. | `ProcessRecord.java:510-517` |
| `start_uptime_ago` | string | yes | The `startUpTime` value. **Uptime-relative, not wall clock.** | `ProcessRecord.java:522-523` |
| `start_elapsed_time_ago` | string \| null | yes | Same on the elapsed-realtime clock. | `ProcessRecord.java:524-525` |
| `start_wall_clock` | string \| null | yes | **Derived**: `snapshot_instant − start_uptime_ago`, i.e. the wall clock of the dumpsys snapshot that printed `startUpTime`, *minus* the process's age. **Not** `boot.at + start_uptime_ago`: `startUpTime` is a relative age (`ProcessRecord.java:522` formats `nowUptime − mStartUptime`), so it must be anchored to the snapshot instant, not to boot. | derived |
| `start_wall_clock_evidence` | object | yes | **An evidence locator, not a quoted string.** `{archive_path, section_title, section_ordinal, line_start, line_end, excerpt}` — a `corroborating_evidence`-shaped object per [§2.7](#27-evidence). The previous form was a bare quoted log line with no `line_start`, so it was unverifiable under §2.7 despite claiming to be evidence. When the only source is the `Start proc` logcat line, the locator points there; when no such line exists, `start_wall_clock` is `null` and an `evidence_gaps[]` entry records it. |
| `last_activity_time_ago` | string | yes | | `ProcessRecord.java:520-521` |
| `persistent` / `removed` | boolean | yes | Only printed when set. | `ProcessRecord.java:527-529` |
| `start_seq_conflict` | boolean | yes | `true` if two entries share a `start_seq`. Impossible by construction, so `true` means the two `dumpsys` snapshots disagree. Cheap integrity check. | derived |
| `code_path` / `data_dir` | string | yes | | `ProcessRecord.java:503-505` |
| `instruction_set` / `required_abi` | string \| null | yes | | `ProcessRecord.java:496-499` |
| `class_name` | string \| null | yes | | `ProcessRecord.java:500-502` |
| `compat` | string \| null | yes | The printed `compat={…}` braces, verbatim. | `ProcessRecord.java:518-520` |
| `oom_adj` | object | yes | `{max, cur_raw, set_raw, cur, set}`. | oom-adj dump |
| `oom_adj_name` | string | yes | From the LRU line: `pers`, `fg`, `vis`, `psvc`, `cch`, `sys`. | `ProcessList.java:1161-1180,4341` |
| `cur_proc_state` / `set_proc_state` | integer | yes | | process-state dump |
| `capability` | string | yes | `LCMNFUA`. **7 chars, positional, `-` = absent.** Carried as the raw string; no per-character decode, because that decode was not traced to a source. | `ProcessList.java:4343` |
| `lru` | object | yes | `{index, oom_adj_name, capability, flags, fixed, service, top_activity, client, cch_empty}`. | `ProcessList.java:4333-4400` |
| `services[]` / `providers[]` / `receivers[]` / `connections[]` | array | yes | Component wiring. **`crashes.json` reads these; it does not re-parse them.** | `ProcessRecord.java:559-563` |
| `lifecycle_refs` | array of object | yes | `{event_kind, at, event_seq_index}` linking to `lifecycle_events[]`. | derived |
| `provenance` | object | yes | See [§2.4](#24-provenance). **No `lifecycle_unresolved_reason`.** A ~380-character paragraph inside a `ProcessEntry` is prose, and on the reference artifact it disagreed with `attribution.unresolved[]` about the vocabulary. There is now one place to look — `attribution.unresolved[]`, keyed by `reason` + `count` — and it carries no sentence. | — |

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
| `process_key` | string \| null | yes | `null` when no `ProcessRecord` interval contains `at`, with the cause recorded in `attribution.unresolved[]` rather than in a prose field on the event. | derived |
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
| `range_raw` | string | yes | Verbatim. The locale form contains U+202F narrow no-break spaces and an en dash — **do not regex-split on a space.** | same |
| `user_id` | integer \| null | yes | The user the enclosing `usagestats` block declares. **The `ACTIVITY_*` line itself carries no `user=` field**, so this is a block-level join; where the enclosing block declares no user, the value is `null` and an `evidence_gaps[]` entry records it. The open question this field carried is **closed**: the archive does contain a block for a second user, so coverage is not user-0-only. | — |
| `range_header_form` | enum | yes | **Moved to the `channels[]` entry for this section.** It is a property of *which dumpsys invocation produced the section*, not of an individual event, and two invocations render it differently — modelling it per-event let a parser merge a locale-formatted window with an epoch-millisecond one and produce a 24-hour span. | `UserUsageStatsService.java:1157-1163` |
| `range_window_seconds` | number \| null | yes | The interval parsed from the section's range header, so a consumer can tell whether these events describe the log window or something far wider. `null` when the header carries no year or no offset and the interval therefore cannot be reconstructed — on the reference archive the offset is recoverable only from the `----- pid N at … -----` stamps inside the block, which is a reconstruction, not a read. | derived |

##### `attribution`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kill_sources` | object | yes | `{ am_kill_events, ams_crash_loop_kill_lines, app_errors_am_process_crashed_too_much }`. **Three disjoint buckets, never summed into one `kills` number.** The second key is renamed from `ams_crash_loop_kills` to `…_kill_lines`: the bytes are a logcat line, not a kill notification, and calling them *kills* asserted an attribution the archive does not carry. |
| `kill_source_coverage` | object | yes | **Renamed from `kill_source_completeness`, and the verdict strings are removed.** One entry per bucket: `{ channel, window_start, window_end, coverage_ratio, covers_log_window }` — the same numbers `channels[]` already carries. A prose verdict like `"complete_for_the_window"` or a 100-character `"PARTIAL — …"` is a judgment about completeness that no archive byte settles: a zero in a channel covering a third of the log window cannot establish absence over the rest of it. |
| `pid_reuse` | object | yes | `{worst_process, pid_first, pid_last, distinct_pid_count, pid_max, window_first, window_last, window_seconds, start_proc_events}`. **`pid_delta` and `estimated_full_pid_cycles` are removed** — a PID *range span* is not a count, and dividing it by an assumed cycle size found nowhere in the archive produced a number [§5.2](#52-corrected-facts) has already retracted. `distinct_pid_count` and `pid_max` answer the recycling question with two measured values: if `distinct_pid_count` approaches the event count, PIDs are not being reused. `conclusion` is removed with the rest; the identity model above already states it. |
| `unresolved[]` | array of object | yes | Events that could not be joined, each with a `reason` from **one closed enum** shared by `unresolved[]` and `lifecycle_events[].unresolved_reason`: `no_process_record`, `outside_snapshot_range`, `isolated_uid`, plus `count`. The reference artifact used two different vocabularies for the same field and prefixed prose onto the enum value, which made the value unusable as a join key. |

`am_kill: 0` is a **bounded negative**, not a proof. The event-log buffer covers a fraction of
the log window, so the honest claim is *"no `am_kill` event in the covered interval"*, and
`kill_source_coverage` is what makes that bound visible. Any consumer wanting to assert no
memory-pressure kill across the whole window must read `/proc/vmstat oom_kill` from `memory.json`,
which is a different channel with a different window.

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
    "boot_epoch": "<boot.at>",
    "boot_epoch_uncertainty_seconds": <uncertainty_seconds>,
    "uptime_seconds_at_start": <seconds>,
    "at_snapshot_dumpsys_activity_seconds": <seconds>
  },

  "channels": [
    { "name": "logcat_main", "present": true,
      "first_at": "MM-DD HH:MM:SS.mmm", "last_at": "MM-DD HH:MM:SS.mmm",
      "duration_seconds": <log_window_seconds>, "covers_log_window": true, "coverage_ratio": 1.0,
      "incomplete_reason": null, "snapshot_at": null,
      "blocks": [
        { "index": 0, "command": "<logcat argv, first block>", "line_start": <line>, "line_end": <line> },
        { "index": 1, "command": "<logcat argv, -T bounded second block>", "line_start": <line>, "line_end": <line> }
      ],
      "carries": ["Start proc", "has crashed too many times, killing!", "am_* events"] },
    { "name": "am_proc_event_log", "present": true,
      "first_at": "MM-DD HH:MM:SS.mmm", "last_at": "MM-DD HH:MM:SS.mmm",
      "duration_seconds": <event_log_seconds>, "covers_log_window": false, "coverage_ratio": <coverage_ratio>,
      "incomplete_reason": "event_log_buffer_retention", "snapshot_at": null,
      "blocks": [{ "index": 0, "command": "<section_title>", "line_start": <line>, "line_end": <line> }],
      "carries": ["am_proc_start", "am_proc_bound", "am_proc_died", "am_kill", "am_uid_running", "am_uid_active"] },
    { "name": "dumpsys_activity_processes", "present": true, "snapshot_at": "<captured_at>",
      "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["PID mappings", "Process LRU list", "*APP*/*PERS* records"] },
    { "name": "dumpsys_package", "present": true, "snapshot_at": "<captured_at>",
      "duration_seconds": <duration_seconds>, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["Package [ blocks", "SharedUser [ blocks", "per-user state blocks"] },
    { "name": "usagestats", "present": true,
      "snapshot_at": "<captured_at>", "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": false, "coverage_ratio": null,
      "incomplete_reason": "permission_denied", "blocks": [],
      "carries": ["ACTIVITY_RESUMED", "ACTIVITY_STOPPED"],
      "range_header_form": "timeRange_locale",
      "range_raw": "<verbatim range header, U+202F and en dash intact>",
      "range_window_seconds": null },
    { "name": "fs_archive", "present": true, "snapshot_at": null,
      "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null, "blocks": [],
      "carries": ["FS/** ZIP entries"] },
    { "name": "pm_list_packages", "present": false,
      "snapshot_at": null, "duration_seconds": null, "first_at": null, "last_at": null,
      "covers_log_window": null, "coverage_ratio": null, "incomplete_reason": null,
      "blocks": [], "carries": [] }
  ],

  "inventory": {
    "counts": {
      "packages": <n>, "processes": <n>, "processes_persistent": <n>, "processes_app": <n>,
      "distinct_process_names": <n>, "installed_packages_with_live_process": <n>,
      "users": <n>, "shared_uids": <n>, "package_user_state_blocks": <n>,
      "lru": { "total": <n>, "non_activity": <n>, "non_service": <n> },
      "car_service_process_names": <n>,
      "car_service_packages": <n>,
      "lifecycle_channel_window": {
        "first_event_at": "MM-DD HH:MM:SS.mmm", "last_event_at": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <event_log_seconds>, "covers_full_log_window": false,
        "coverage_ratio": <coverage_ratio>,
        "section_titles": ["<section_title>"],
        "sections_scanned": [
          { "title": "<section_title>", "ordinal": 0, "start_line": <line>, "end_line": <line>, "matched": <n> },
          { "title": "<section_title>", "ordinal": 1, "start_line": <line>, "end_line": <line>, "matched": 0 }
        ]
      },
      "sources_available": ["dumpsys package", "dumpsys activity processes", "am_proc event log",
                            "usagestats", "logcat main", "FS/** ZIP entries"],
      "sources_absent": ["pm list packages -f -U -i", "per-package dumpsys section"]
    },
    "counts_am_proc_hits": {
      "_note": "each am_proc_* count is a match block per §2.3; abbreviated here to two keys",
      "am_proc_died_hits": {
        "pattern": "am_proc_died", "pattern_kind": "tag", "case_sensitive": true,
        "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
        "coverage": { "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
                      "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
        "regions": [{ "archive_path": "<main_entry>", "section_title": "<section_title>",
                      "section_ordinal": 0, "line_start": <line>, "line_end": <line>, "line_count": <n> }]
      },
      "am_kill_hits": {
        "pattern": "am_kill", "pattern_kind": "tag", "case_sensitive": true,
        "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
        "coverage": { "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
                      "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
        "regions": []
      }
    },

    "packages": [
      { "package_name": "<package_name>",
        "identity_hash": "<hex>", "app_id": <n>, "shared_uid_name": null,
        "tier": "third_party",
        "tier_basis": [["code_path_prefix", "/data/app"], ["package_source", 1],
                       ["apk_signing_version", 2], ["flags_has_SYSTEM", false]],
        "version_name": "<n>", "version_code": <n>, "min_sdk": <n>, "target_sdk": <n>,
        "code_path": "/data/app/~~<hash>==/<package_name>-<n>-<hash>==",
        "code_path_prefix": "/data/app",
        "flags": ["DEBUGGABLE","HAS_CODE","ALLOW_CLEAR_USER_DATA","TEST_ONLY","ALLOW_BACKUP"],
        "apk_signing_version": 2,
        "package_source": 1, "package_source_name": "PACKAGE_SOURCE_OTHER",
        "initiating_package_name": "<initiating_package_name>",
        "last_update_time": "<YYYY-MM-DD> HH:MM:SS",
        "install_permissions": ["<package_name>.<permission>", "<vendor.permission>.<name>"],
        "users": [
          { "user_id": 0, "installed": false, "data_dir": null, "enabled": 0,
            "first_install_time": null, "first_install_time_raw": "<epoch-era sentinel, verbatim>",
            "ce_data_inode": null, "de_data_inode": null,
            "install_reason": 0, "stopped": true },
          { "user_id": 10, "installed": true,
            "data_dir": "/data/user/10/<package_name>",
            "ce_data_inode": <n>, "de_data_inode": <n>, "enabled": 0,
            "first_install_time": "<YYYY-MM-DD> HH:MM:SS",
            "install_reason": 0, "stopped": false, "not_launched": false }
        ],
        "provenance": { "evidence_origin": "dumpsys_package", "archive_path": "<main_entry>",
          "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "now",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "package updated within the same boot as capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "package_name": "<automotive_system_package>",
        "identity_hash": "<hex>", "app_id": <n>, "shared_uid_name": "android.uid.system",
        "tier": "shared_system",
        "tier_basis": [["shared_uid_name", "android.uid.system"], ["code_path_prefix", "/system"],
                       ["flags_has_SYSTEM", true]],
        "version_name": "<n>", "version_code": <n>, "min_sdk": <n>, "target_sdk": <n>,
        "code_path": "/system/app/<AppDir>", "code_path_prefix": "/system",
        "flags": ["SYSTEM","HAS_CODE","PERSISTENT","ALLOW_CLEAR_USER_DATA","ALLOW_BACKUP"],
        "apk_signing_version": 3,
        "package_source": 0, "package_source_name": "PACKAGE_SOURCE_UNSPECIFIED",
        "last_update_time": "<YYYY-MM-DD> <HH:MM:SS>",
        "users": [
          { "user_id": 0, "installed": true,
            "data_dir": "/data/user/0/<package_name>",
            "ce_data_inode": <inode>, "de_data_inode": <inode>,
            "first_install_time": "<YYYY-MM-DD> <HH:MM:SS>", "enabled": 0, "stopped": false }
        ],
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
          "freshness": "stale",
          "freshness_basis": { "input": "lastUpdateTime",
            "basis": "system package last updated 128 days before capture",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "processes": [
      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "persistent", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<process_name>",
        "runtime_key": "<user_id>:<process_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": 0, "uid": <uid>, "uid_formatted": "<uid_formatted>", "app_id": <app_id>,
        "gids": [<gids>],
        "isolated_uid": null,
        "packages": ["<package_name>"], "package_dependencies": [],
        "start_uptime_ago": "-<N>ms", "start_elapsed_time_ago": "-<N>ms",
        "start_wall_clock": "<captured_at>",
        "start_wall_clock_evidence": { "archive_path": "<main_entry>", "kind": "line_range",
          "section_title": "<section_title>", "section_ordinal": 0,
          "locator": { "line_start": <line>, "line_end": <line> },
          "excerpt": "<verbatim Start proc line>", "excerpt_truncated": false,
          "excerpt_bytes": <n>, "excerpt_complete": true },
        "last_activity_time_ago": "-<N>ms",
        "persistent": true, "removed": false, "start_seq_conflict": false,
        "code_path": "/system/app/<AppDir>/<App>.apk",
        "data_dir": "/data/user/<user_id>/<package_name>",
        "instruction_set": "<isa>", "required_abi": "<abi>",
        "class_name": "<package_name>.<ApplicationClass>",
        "compat": "<compat braces, verbatim>",
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "cur_proc_state": <n>, "set_proc_state": <n>, "capability": "<7-char raw>",
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "fixed": true },
        "providers": ["<provider_class>"],
        "lifecycle_refs": [],
        "provenance": { "evidence_origin": "dumpsys_activity_processes",
          "archive_path": "<main_entry>",
          "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer 'ending at: HH:MM:SS'",
            "basis": "process liveness is only true at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "persistent", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<system_process_name>", "runtime_key": "<user_id>:<process_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": 0, "uid": <uid>, "app_id": <app_id>,
        "packages": ["<package_name>"], "package_dependencies": ["<package_name>"],
        "start_uptime_ago": "-<N>s<N>ms",
        "start_wall_clock": "<captured_at>",
        "persistent": true,
        "code_path": "/system/priv-app/<AppDir>/<App>.apk",
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "fixed": true },
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer", "basis": "liveness at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } },

      { "process_key": "<boot_epoch>/<start_seq>", "start_seq": <n>,
        "record_kind": "app", "record_identity_hash": "<hex>", "pid": <pid>,
        "process_name": "<package_name>",
        "runtime_key": "<user_id>:<package_name>",
        "attribution_key": "<package_name>/<user_id>",
        "user_id": <n>, "uid": <uid>, "uid_formatted": "<uid_formatted>", "app_id": <app_id>,
        "gids": [<gids>],
        "packages": ["<package_name>"],
        "start_uptime_ago": "-<N>s<N>ms",
        "start_wall_clock": "<captured_at>",
        "last_activity_time_ago": "-<N>s<N>ms",
        "persistent": false,
        "oom_adj": { "max": <n>, "cur_raw": <n>, "set_raw": <n>, "cur": <n>, "set": <n> },
        "lru": { "index": <n>, "oom_adj_name": "<name>", "capability": "<7-char raw>",
                 "flags": "<flags column>", "top_activity": true },
        "provenance": { "captured_at": "<captured_at>",
          "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
          "freshness": "now",
          "freshness_basis": { "input": "dumpsys footer", "basis": "liveness at the snapshot instant",
            "threshold_seconds": 3600 },
          "collector_perturbed": false } }
    ],

    "index": {
      "package_to_process_keys": { "<package_name>": ["…/<start_seq>"] },
      "process_name_to_packages": {
        "system": ["android", "com.android.providers.<name>", "com.android.server.<name>"],
        ".<suffix>": ["com.<vendor>.<suffix>.api"]
      },
      "unmapped_process_names": { "count": <n>,
        "examples": ["system", "android.process.<media>", "android.car.<usb>.handler",
                     ".<suffix>", "<WebViewLoader>"] }
    }
  },

  "lifecycle_events": [
    { "kind": "am_proc_start", "at": "MM-DD HH:MM:SS.mmm",
      "uid_column_raw": "<uid column, verbatim>", "pid": <pid>, "tid": <tid>,
      "user_id": <n>, "start_uid": <uid>, "process_name": "<process_name>",
      "hosting_type": "service",
      "hosting_component": "{<package>/<package>.<ServiceClass>}",
      "process_key": "<boot_epoch>/<start_seq>",
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "kind": "am_proc_start", "at": "MM-DD HH:MM:SS.mmm",
      "uid_column_raw": "<uid column, verbatim>", "pid": <pid>, "tid": <tid>,
      "user_id": <n>, "start_uid": <uid>,
      "process_name": "<WebViewLoader process>", "hosting_type": "", "hosting_component": "",
      "process_key": null,
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp", "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "activity_events": [
    { "at": "<captured_at>", "type": "ACTIVITY_RESUMED",
      "package_name": "<package_name>",
      "class_name": "<package_name>.<ActivityClass>",
      "user_id": <n>,
      "range_window_seconds": null,
      "provenance": { "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "captured_at": "<captured_at>",
        "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot",
        "freshness": "now",
        "freshness_basis": { "input": "usagestats event timestamp",
          "basis": "second granularity; inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "attribution": {
    "kill_sources": {
      "am_kill_events": 0,
      "ams_crash_loop_kill_lines": <n>,
      "app_errors_am_process_crashed_too_much": <n>
    },
    "kill_source_coverage": {
      "am_kill_event_log": { "channel": "am_proc_event_log",
        "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <event_log_seconds>, "covers_log_window": false,
        "coverage_ratio": <coverage_ratio> },
      "ams_logcat": { "channel": "logcat_main",
        "window_start": "MM-DD HH:MM:SS.mmm", "window_end": "MM-DD HH:MM:SS.mmm",
        "duration_seconds": <log_window_seconds>, "covers_log_window": true,
        "coverage_ratio": 1.0 }
    },
    "pid_reuse": {
      "worst_process": "<process_name>",
      "pid_first": <n>, "pid_last": <n>, "distinct_pid_count": <n>, "pid_max": <n>,
      "window_first": "MM-DD HH:MM:SS.mmm", "window_last": "MM-DD HH:MM:SS.mmm",
      "window_seconds": <seconds>,
      "start_proc_events": <n>
    },
    "unresolved": [
      { "reason": "outside_snapshot_range", "count": <n> },
      { "reason": "isolated_uid", "count": <n>, "example": "<uid_formatted>" }
    ]
  },

  "evidence_gaps": [
    { "field": "inventory.shared_uids[].*", "reason": "UNVERIFIABLE",
      "detail": "the SharedUser header prints a name and an identity hash but no id; the id is read from the archive where present and left absent where not." },
    { "field": "lifecycle_events[].uid_column_raw", "reason": "PARSE_INDETERMINATE",
      "detail": "the logcat uid column renders as a getpwuid name when short, a decimal uid otherwise, and blanks below zero. uid_column_raw preserves it verbatim and no coercion is attempted at parse time; consumers must resolve it." },
    { "field": "attribution.kill_sources.am_kill_events", "reason": "REGION_EXCLUDED",
      "detail": "zero am_kill events in the event-log interval. This is a bounded negative over the covered window only, not an assertion that no memory-pressure kill occurred in the rest of the log window." },
    { "field": "activity_events[].range_window_seconds", "reason": "PARSE_INDETERMINATE",
      "detail": "the range header carries no year and no timezone offset; the interval is only reconstructible from the pid stamps inside the block, which is a reconstruction rather than a read, so the field is null." },
    { "field": "activity_events[]", "reason": "UNVERIFIABLE",
      "detail": "whether these events cover a second user profile is settled here: the archive does contain a second-user block in the same section, so the earlier open question is closed." }
  ],

  "findings": [],

  "notes": [
    "EMPTY_FINDINGS_BY_DESIGN: apps.json is inventory and attribution, not diagnosis. Process churn, crash loops and memory pressure are findings belonging to cpu.json, crashes.json and memory.json respectively; duplicating them here would let two artifacts carry two answers to one question. This note satisfies §2.1's rule that an empty findings array must say why.",
    "UNATTRIBUTED_EVENTS_ARE_DECLARED: lifecycle events that cannot be joined to a process instance are counted in attribution.unresolved[] with a reason from a closed enum, never dropped and never guessed."
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
| `provenance` | object | yes | See [§2.4](#24-provenance). |

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
| `total_bytes` | integer | yes | The sum of the printed per-file size column. **Exact**, and therefore needs no `_derivation` sibling per [§2.9](#29-null-absent-and-raw) rule 4. The previous `total_bytes_approx` derived from the `ls` total was both less exact than the archive allowed and ~507 KB larger than the true byte total. |
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
  needed: the formula is stated in the field table, per [§2.9](#29-null-absent-and-raw) rule 4.
- **ANR file timestamps are well-dated.** An earlier claim that the trace carries "no
  timestamp inside" is **refuted**: every process block carries a nanosecond wall clock in its
  `----- pid N at … -----` header. **Staleness here is provable, not inferred.**
- **The `ANR FILES` `ls` column is minute-granular** while the filename is millisecond. The
  `mtime_ls_minute_differs_from_filename` field reports where the two disagree. It makes no
  causal claim — both are written by the same device clock, so it cannot detect a clock change.

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

| Channel | Emitter kind | Guarantee | What it bounds |
| --- | --- | --- | --- |
| `crash_buffer` (`FATAL EXCEPTION`) | `logcat_tag` | ring buffer, complete until wrap | **the authoritative Java crash count** |
| `am_crash_event_log` | `event_log_tag` | ring-wrapped, lossy | a bounded fraction of the loop; the event log's buffer covers well under the whole log window |
| `dropbox_crash` | `dumpsys_section` | rate-limited per `(process, eventType)` | first occurrences only; the rate limit is not published by any archive |
| `tombstone_files` | `zip_entry_listing` | ring of `tombstoned.max_tombstone_count` | the retained tombstone set, which may predate this boot |
| `am_proc_died` | `event_log_tag` | ring-wrapped | process death over the event-log window only |
| `manager_kill_lines` | `logcat_tag` | one line per manager decision | manager actions taken, distinct-matched |
| `rescueparty_lines` | `logcat_tag` | one line per mitigation attempt | mitigation attempts, distinct-matched |

**The guarantee column is platform behaviour, not an observation.** A tool cannot read any of
it from a bugreport: no archive records a ring size, a rate limit, or a writer's intent. That is
why the same information appears twice in this document — as prose here, where a reader can see
the reasoning once, and as a measured `coverage_ratio` on each `counts.<name>_hits` block, which
*is* derivable from the bytes. The artifact carries only the latter.

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
| `channels` | array of object | yes | One entry per channel. Carries `emitter_kind`; `authoritative_for_count` is replaced by a single top-level `count_authority`. |
| `count_authority` | object | yes | `{channel, rule}`. **One top-level object naming which channel bounds the count and the published rule for choosing it.** The previous form was a per-channel `authoritative_for_count` boolean, which restated a policy decision on every row and let two channels disagree about it. |
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

A group is a **crash loop** when all five hold. **The five thresholds are configurable inputs,
not constants baked into the tool.** Each is published in `classification.thresholds`, so an
artifact states the bar it applied and a reader can re-apply it differently:

| Criterion | Field | Default threshold | Where the default comes from |
| --- | --- | --- | --- |
| Repeat count | `repeat_count` | ≥ 5 | Chosen so the bar sits just under the platform's own rate limiter (`DropboxRateLimiter.java:39`, `RATE_LIMIT_ALLOWED_ENTRIES_DEFAULT = 6`) — a limiter that lets 6 through is evidence the platform already treated 5+ as the same problem. **The limit itself is not published by any archive**, so it is a default, not a measurement. |
| Rate | `rate_per_second` | ≥ 1.0 | The loop must be faster than human retry. |
| Window | `window_seconds` | ≤ 300 s | Sized against the platform's own "this is one event, not a trend" bounds (`AnrHelper.java:56,61`). |
| Continuity | `inter_gap_max_ms` | ≤ 120000 | The platform's `quickCrash` test is `now < crashTime + MIN_CRASH_INTERVAL` (`AppErrors.java:896-897`), and `min_crash_interval` **is** printed in the archive's dumpsys output — so this one is read, not assumed. |
| Escalation | `manager_kill_count` or `rescueparty_failure_count` | ≥ 1 | Both subsystems act only on repetition. |

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
| `signature.abort_message_hash` / `abort_message_varies` | string \| null / boolean | yes (native) | The hash of the tombstone's `Abort message:` line, plus whether that message varies within the group. **The message is never published as free text and never used as a grouping key**, because it is unstable across builds: a build-to-build reword would silently split one bug into two. `abort_message_varies: true` forces a sub-grouping by `top_frame`. The reference archive's tombstones split 47 / 3 on this field, so a single group would have hidden a second root cause. |
| `packages[]` / `package_count` | array / integer | yes | May exceed 1 for `:remote` loops. | `RuntimeInit.java:94` |
| `repeat_count` | integer | yes | **Distinct** occurrences, de-duplicated. | §274-vs-271 |
| `raw_line_hits` | integer | yes | Every matching line **before** de-duplication. | — |
| `duplicate_hits` | integer | yes | `raw_line_hits − repeat_count`. | §274-vs-271 |
| `duplicate_reason` | **Removed**, replaced by `duplicate_detection`. | — | The previous value named an AOSP `path:line` and asserted a collector the tool cannot inspect — *no archive byte records which collector re-rendered a line*. Worse, its stated discriminator was a mis-parse: it read a blank uid column as a uid value. |
| `duplicate_detection` | object | yes | `{method, duplicate_pids}`. `method` ∈ `same_pid_same_second` — the actual rule, decided from bytes — and `duplicate_pids` lists the pids seen more than once. **A statement about what matched, not about who produced it.** | derived |
| `first_seen` / `last_seen` | string | yes | Min/max occurrence time. | `logprint.cpp:1552` / `tombstone_proto_to_text.cpp:616` |
| `window_seconds` | number | yes | `last − first`. | derived |
| `rate_per_second` | number \| null | yes | `repeat_count ÷ window_seconds`. **No `rate_derivation` string**: the formula is stated here, so per [§2.9](#29-null-absent-and-raw) rule 4 a sibling string would only restate it. | derived |
| `inter_gap_p50_ms` / `inter_gap_max_ms` | integer \| null | yes | Derived over the gaps between occurrence times. **Milliseconds in both the field name and the loop criterion** — the reference artifact's criterion table said `inter_gap_max_seconds` while the field table said `_ms`, and an implementer following the first would emit the wrong unit. | derived |
| `pid_span` | object | yes | `{first, last, distinct_count, pid_max}`. `null` members when the pids are cross-boot and meaningless against this boot. | `logprint.cpp:1553` |
| `classification` | enum | yes | `crash_loop` / `recurring` / `isolated` / `stale_recurring`. | §loop versus isolated |
| `classification.thresholds` | object | yes | The five values actually applied. Published so `classification` is re-derivable. | — |
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

Raw `FATAL EXCEPTION` count is **274**. Decomposing by region:

| Region | Line range | `FATAL EXCEPTION` |
| --- | --- | --- |
| A — first `SYSTEM LOG` (`logcat -v threadtime … -d *:v`) | 9116–53708 | 38 |
| B — `DROPBOX SYSTEM APP CRASHES` (inlined logcat) | 652155–652449 | **3** |
| C — trailing `SYSTEM LOG` (`… -T <timestamp>`) | 653373–682949 | 233 |
| | **total** | **274** |

> **Region B's line range was wrong in the previous revision.** It was given as `53709–653372`,
> but `53709` is the *duration footer* of the first `SYSTEM LOG` and the `DROPBOX SYSTEM APP
> CRASHES` section actually spans `652155–652449` — and that section contains **zero**
> `FATAL EXCEPTION` lines. The three hits live in the dumpsys/inline-logcat region, not in the
> section the range named. A locator that points at a section which does not contain the evidence
> is worse than no locator, because a consumer who checks it concludes the evidence is absent.

The duplicates in region B are found by **`(pid, second)`** — the three pids that appear twice
are exactly the three that appear once in region A and once in region C. That rule is decided from
bytes alone and needs no knowledge of which collector re-rendered them.

**Therefore: 271 distinct crashes, 274 raw lines.** This is the "a raw grep count is never a
finding" rule demonstrated rather than asserted — and the naive `pid` key produces the same
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

**Incident 2 — the old native group.** Every tombstone predates the capture by months.

| Fact | Value |
| --- | --- |
| File mtimes | two stamps from the ZIP directory, with the span derived |
| In-file `Timestamp:` on one entry | the entry's own header stamp |
| `Cmdline:` | one package name, present in every tombstone |
| `signal` | one signal, present in every tombstone |
| `Abort message:` | **two distinct values across the 50 tombstones** |
| Grouping | **two** groups, 47 and 3 occurrences |

> **The previous revision reported this as one group of 50.** The `Abort message:` field splits
> the 50 tombstones 47 / 3 — the three minority entries abort from `stack_manager.cc` at startup
> rather than from `hci_layer.cc` on a command response. Fusing them hid a second root cause. The
> spec's own grouping table already names the abort message as the discriminator for native groups;
> the worked example simply did not apply it.

This is a completely separate incident from the Java loop: different language, different signal
class, different date, and it never touched the crashing package.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "crashes",

  "capture_ref": { "source": "system.json#/capture", "pid_max": <pid_max>, "min_crash_interval_ms": <ms> },

  "counts": {
    "fatal_exception_hits": {
      "pattern": "FATAL EXCEPTION", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the suppressed lines are second renderings of crashes already counted: each duplicates another occurrence on (pid, second), and its logcat prefix uid differs from the uid in the message body",
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 1,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    },
    "tombstone_hits": {
      "pattern": "tombstone", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "no tombstone banner exists anywhere in the main entry; the real tombstones live in FS/data/tombstones/ as ZIP entries, which is where this artifact looks",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> },
        { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    },
    "signal_hits": {
      "pattern": "signal ", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the suppressed matches are teardown and thread-dump lines whose 'signal' token is not a fatal-signal banner; fatal signals use the tombstone banner form, which this artifact reads from the tombstone ZIP entries instead",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "am_crash_hits": {
      "pattern": " am_crash:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
      "regions": [
        { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0,
          "line_start": <line>, "line_end": <line>, "line_count": <n> }
      ]
    }
  },

  "channels": [
    { "channel": "crash_buffer", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "fatal_exception_hits" },
    { "channel": "am_crash_event_log", "emitter_kind": "event_log_tag", "present": true,
      "counts_ref": "am_crash_hits" },
    { "channel": "dropbox_crash", "emitter_kind": "dumpsys_section", "present": true,
      "counts_ref": "dropbox_crash_hits" },
    { "channel": "tombstone_files", "emitter_kind": "zip_entry_listing", "present": true,
      "counts_ref": "tombstone_entry_records" },
    { "channel": "am_proc_died", "emitter_kind": "event_log_tag", "present": true,
      "counts_ref": "am_proc_died_hits" },
    { "channel": "manager_kill_lines", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "crashed_too_many_hits" },
    { "channel": "rescueparty_lines", "emitter_kind": "logcat_tag", "present": true,
      "counts_ref": "rescueparty_hits" }
  ],

  "count_authority": {
    "channel": "crash_buffer",
    "rule": "the crash count is taken from the crash buffer, the only channel whose window spans the whole log window with no rate limit and no ring retention"
  },

  "java_crash_groups": [
    { "group_id": "<java_group_id>",
      "language": "java",
      "signature": {
        "exception_class": "<exception_class>",
        "signal_number": null, "signal_name": null, "sigcode_name": null,
        "top_frame": "<normalised top frame symbol, no line number>"
      },
      "packages": ["<crashing_package>"], "package_count": 1,
      "repeat_count": 271,
      "raw_line_hits": 274, "duplicate_hits": 3,
      "duplicate_detection": { "method": "same_pid_same_second", "duplicate_pids": [5769, 9384, 9408] },
      "first_seen": "<first_seen>",
      "last_seen": "<last_seen>",
      "window_seconds": <window_seconds>,
      "rate_per_second": <rate_per_second>,
      "inter_gap_p50_ms": <ms>, "inter_gap_max_ms": <ms>,
      "pid_span": { "first": <pid>, "last": <pid>, "distinct_count": <n>, "pid_max": <pid_max> },
      "classification": "crash_loop",
      "classification.thresholds": { "repeat_count": <n>, "rate_per_second": <n>,
                                      "window_seconds": <n>, "inter_gap_max_ms": <ms>,
                                      "manager_kill_count": <n> },
      "classification_met": ["repeat_count", "rate_per_second", "window_seconds",
                             "inter_gap_max_ms", "manager_kill_count"],
      "manager_kill_count": <n>, "manager_kill_reason": "<printed Reason: token, verbatim>",
      "rescueparty_failure_count": <n>, "rescueparty_disabled": <boolean>,
      "corroboration": [
        { "channel": "am_crash_event_log", "count": <n>, "count_kind": "event_log_rows" },
        { "channel": "dropbox_crash", "count": <n>, "count_kind": "logcat_lines_distinct" },
        { "channel": "am_proc_died", "count": <n>, "count_kind": "event_log_rows" },
        { "channel": "manager_kill_lines", "count": <n>, "count_kind": "logcat_lines_distinct" },
        { "channel": "rescueparty_lines", "count": <n>, "count_kind": "logcat_lines_distinct" }
      ],
      "provenance": { "evidence_origin": "crash_buffer", "archive_path": "<main_entry>",
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
    { "group_id": "<native_group_id>",
      "language": "native",
      "signature": {
        "exception_class": null,
        "signal_number": <n>, "signal_name": "<SIGXXX>", "sigcode_name": "<SI_XXX>",
        "top_frame": "<demangled top frame, symbol only>",
        "abort_message_hash": "<hex>",
        "abort_message_varies": false
      },
      "packages": ["<package>"], "package_count": 1,
      "repeat_count": <n>,
      "raw_line_hits": <n>, "duplicate_hits": 0,
      "duplicate_detection": { "method": "same_pid_same_second", "duplicate_pids": [] },
      "first_seen": "<first_seen>",
      "last_seen": "<last_seen>",
      "window_seconds": <window_seconds>,
      "rate_per_second": <rate_per_second>,
      "inter_gap_p50_ms": null, "inter_gap_max_ms": null,
      "pid_span": { "first": null, "last": null, "distinct_count": null, "pid_max": <pid_max> },
      "classification": "stale_recurring",
      "classification.thresholds": { "repeat_count": <n>, "rate_per_second": <n>,
                                      "window_seconds": <n>, "inter_gap_max_ms": <ms>,
                                      "manager_kill_count": <n> },
      "classification_met": [],
      "manager_kill_count": 0, "manager_kill_reason": null,
      "rescueparty_failure_count": 0, "rescueparty_disabled": null,
      "corroboration": [ { "channel": "tombstone_files", "count": <n>, "count_kind": "logcat_lines_distinct" } ],
      "provenance": { "evidence_origin": "tombstone_files", "archive_path": "<main_entry>",
        "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "tombstone_header",
        "evidence_time_source": "embedded_trace_stamp",
        "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact",
        "freshness": "stale",
        "freshness_basis": { "input": "the tombstone's own in-file Timestamp line",
          "basis": "every tombstone in this group predates the capture by months; the tombstone_NN filename index is a ring counter and carries no time meaning, so only the in-file stamp is read",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "the tombstones were read from disk; dumpstate did not generate them" } }
  ],

  "java_crash_incidents": [
    { "incident_id": "crash-0001",
      "group_id": "<group_id>",
      "language": "java",
      "package": "<crashing_package>", "process": "<crashing_process>",
      "pid": <pid>, "prefix_pid": <pid>, "prefix_uid": <uid>,
      "thread_name": "<thread_name>",
      "exception_class": "<exception_class>",
      "exception_message": null,
      "top_frames": [
        "<frame_1_symbol>(<File>:<line>)",
        "<frame_2_symbol>(<File>:<line>)",
        "<frame_3_symbol>(<File>:<line>)",
        "<frame_4_symbol>(<File>:<line>)",
        "<frame_5_symbol>(<File>:<line>)"
      ],
      "occurred_at": "<occurred_at>",
      "flags_hex": "<hex>",
      "crash_handler": "<ApplicationInfo.Crash-Handler>",
      "evidence_channels": [
        { "channel": "crash_buffer", "join_basis": "same_logcat_line" },
        { "channel": "am_crash_event_log", "join_basis": "same_pid_and_window" },
        { "channel": "dropbox_crash", "join_basis": "same_pid_and_window" },
        { "channel": "am_proc_died", "join_basis": "same_pid_and_window" }
      ],
      "provenance": { "evidence_origin": "crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log",
        "freshness": "now",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "inside capture.log_window; the inlined dropbox copy carries a Timestamp agreeing with the logcat line stamp to within a few ms",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "negative_evidence": [
    { "scope": "native tombstone search in the main entry",
      "statement": "0 occurrences of the tombstone banner in the main entry; the native crashes exist only as ZIP entries under FS/data/tombstones/",
      "source": { "archive_path": "<main_entry>", "section_title": null, "section_ordinal": null },
      "authority": "archive_absence" },
    { "scope": "am_crash_event_log channel coverage",
      "statement": "the event-log buffer ends before the crash buffer does, so this channel observes a bounded fraction of the loop and must not be used to bound anything",
      "source": { "archive_path": "<main_entry>", "section_title": "EVENT LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "findings": [
    { "id": "crash-0002", "kind": "crash_loop",
      "severity": "critical", "unconstrained_severity": "critical", "confidence": "confirmed",
      "process_key": "<boot_epoch>/<start_seq>",
      "event_time": "<last_seen>",
      "attributes": { "package": "<crashing_package>", "language": "java",
                      "still_restarting_at_capture": true },
      "metrics": { "repeat_count": <n>, "window_seconds": <window_seconds>, "rate_per_second": <rate_per_second>,
                   "inter_gap_max_ms": <ms>, "manager_kill_count": <n>, "rescueparty_failure_count": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim crash-buffer line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "crash_buffer", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "now",
        "freshness_basis": { "input": "last crash logcat line",
          "basis": "the final occurrence is the last line of the crash buffer, so the loop is live at capture",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } },
    { "id": "crash-0003", "kind": "signal_absence",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "tombstone_hits": <n>, "tombstone_real": 0, "signal_hits": <n>, "signal_fatal": 0 },
      "metrics": { "tombstone_files_in_zip": <n> },
      "evidence": { "archive_path": "FS/data/tombstones/tombstone_00", "kind": "file_entry",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "Timestamp: <YYYY-MM-DD HH:MM:SS.nnnnnnnnn+ZZZZ>\nCmdline: <package>\n…",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "tombstone_files",
        "archive_path": "FS/data/tombstones/tombstone_00",
        "captured_at": "<captured_at>", "captured_at_time_base": "tombstone_header",
        "age_seconds_at_capture": <age>, "capture_kind": "disk_artifact", "freshness": "stale",
        "freshness_basis": { "input": "in-file Timestamp line",
          "basis": "searched and found: the raw tombstone matches in the main entry yield 0 real tombstones; the real ones are ZIP entries far older than this boot",
          "threshold_seconds": 3600 },
        "collector_perturbed": false } }
  ],

  "evidence_gaps": [
    { "field": "counts.am_crash_hits.coverage.coverage_ratio", "reason": "REGION_EXCLUDED",
      "detail": "the event-log buffer retains only a bounded interval of the log window, so this channel observes a fraction of the loop. The ratio is published on the match block; no field may bound the crash count from it." },
    { "field": "java_crash_incidents[].evidence_channels[].join_basis", "reason": "UNVERIFIABLE",
      "detail": "the event-log and dropbox records carry no key that joins them to a logcat crash line. Join basis is therefore same_pid_and_window, which is a heuristic, and any finding resting only on it must carry confidence: inferred." },
    { "field": "am_crash_event_log slot layout", "reason": "UNVERIFIABLE",
      "detail": "the payload is positional and its field order is declared in the platform tag file, which no bugreport contains. pid is taken from the crash buffer and cross-checked against the logcat prefix; no slot is decoded on the strength of a declaration the analyzer cannot read." },
    { "field": "native_crash_groups[].pid_span", "reason": "UNVERIFIABLE",
      "detail": "tombstone pids belong to a previous boot and are meaningless against this boot's pid space, so every member is null. Publishing the numbers would invite a comparison that has no meaning." },
    { "field": "native_crash_groups[].signature.abort_message_hash", "reason": "PARSE_INDETERMINATE",
      "detail": "the abort message is free text that changes between builds, so it is hashed rather than compared literally. Where the hash varies within a group, abort_message_varies is true and the group is sub-divided by top frame; the message itself is never published as a grouping key." }
  ],

  "notes": [
    "CRASH_COUNT_IS_DISTINCT: the raw FATAL EXCEPTION count decomposes across three archive regions; the duplicates in the middle region are found by (pid, second) and suppressed. The published repeat_count is the de-duplicated count, and raw_line_hits minus duplicate_hits equals it.",
    "CRASH_COUNT_CHANNEL_IS_CRASH_BUFFER: only the crash buffer spans the whole log window with no rate limit. The event-log channel observes a fraction of it; dropbox is rate limited; the count authority is published as count_authority.",
    "NO_PID_RECYCLING_IN_LOOP: the distinct crashes produced as many distinct, monotonic pids as there were crashes. The apparent repeats were second renderings, not kernel pid recycling. process_key remains the identity because pid is still an observation attribute."
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
  reconstruction's own confidence and the logcat line range, and must name `crash_buffer` as
  the only authoritative Java channel via `count_authority`.
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
emitted by *one* `dumpsys meminfo` call at *one* instant (the block prints a single uptime
figure for the whole dump), so they are mutually consistent and belong in one artifact — but
they answer different questions and aggregate over different keys. A flat per-process list
would force a choice of granularity and would drop either the verdict or the category split. A
single global object would drop attribution entirely, and a process implicated by a crash loop
elsewhere in the output set is only findable from `processes[]`.

#### The self-perturbation design

**The problem.** The two page-scanning sections — an all-process PSS dump and the
`bugreport_procdump` walk — both traverse every process's page tables. A naive schema that
diffs `swap_used` across the archive reports "swap grew ~200 MiB, the device is thrashing" — a
pure artifact of the collector.

There are **four** swap readings in the reference archive, not two, and the growth tracks the
dump's own page-scanning work. The two steps that scan every page table sit exactly where the
number moves, and the collector's own processes appear in its own PSS table.

Three design rules follow:

1. **Never diff snapshots implicitly.** Swap lives in `collection.swap_timeline[]`, an explicit
   ordered list, and every delta is an *edge* between two identified nodes. There is **no**
   `swap.used_kb` scalar a consumer can compare against another artifact's scalar.
2. **Make the collector's own cost a measured quantity.** `collection.page_scanning_sections[]`
   is built from the section footers dumpstate prints unconditionally. Membership is decided by
   **what the section prints** — a per-PID row set carrying a swap column — never by AOSP
   invocation-site knowledge, so the same rule works on a build whose source you cannot read.
   One section's own footer is *inside* its parent's span and is **not** added again.
3. **A consumer that only reads `system` cannot be misled.** `system.provenance.collector_perturbed`
   is `true` for any reading taken inside such a scan. The same pair, in the same place,
   expresses "clean" in `cpu.json` — which is why it is in the shared convention rather than in
   one artifact.

The classification rule is in [§2.1.5](#215-collection). Applied here: the entire measured
growth falls between the last pre-scan reading and the post-scan reading, and over that
interval there are zero kills and `oom_kill 0` — the shape a reader must be able to see before
concluding anything. **Whether that overlap caused the growth is not a field**, because the
archive contains no per-process page-fault accounting that would settle it.

#### Field tables

Condensed. The owning file's exhaustive row-level table is preserved at
`tmp/aosp-team/03-memory-and-cpu.md` §4.4 for the detail that did not survive merging.

##### `collection.swap_timeline[]`

| Field | Type | Required | Meaning | Evidence |
| --- | --- | --- | --- | --- |
| `channel` | enum | yes | `proc_meminfo` / `top` / `bugreport_procdump` / `dumpsys_meminfo`. | — |
| `line_start` | integer | yes | Archive locator, **and the ordering key** per [§2.1.5](#215-collection). **There is deliberately no statement-order field**: dumpstate runs sections on a thread pool, so archive order is *completion* order and no archive recovers the intended order. | measured |
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

Three sub-sections, three different datasets, three different windows. The shape that matters
is not the numbers but the mismatch between them:

| Sub-section header | What its start time actually is | `Aggregated over` |
| --- | --- | --- |
| `CURRENT STATS:` | inside the capture window | 1 |
| `AGGREGATED OVER LAST 24 HOURS:` | **months before this boot** | N |
| `AGGREGATED OVER LAST 3 HOURS:` | **the same value as the 24-hour block** | N |

> **The window named in the header is a label, not a fact.** In the reference archive the
> 3-hour and 24-hour blocks resolve to an identical start time that predates the capture by
> months. A schema that reads an average out of the aggregate blocks and reports it as a device
> property is wrong by the staleness of that data. `capture_kind` here is
> **`persisted_aggregate`**, and that is the value that catches it.

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
| `backing_state_file` | string \| null | yes | The `mFile=` path the aggregate block was read from, verbatim. It carries a timestamp in its name, which is a **filename convention, not a measured clock**. | `ProcessStatsService.java:1362` |
| `sys_mem_usage[]` | array | yes | `{label, sample_count, cached_*, free_*, zram_*, kernel_*, native_*}` min/avg/max. | `SysMemUsageTable.java:142-171` |
| `checkin_period` | object \| null | yes | `{start_clock, start_realtime_ms, end_realtime_ms, complete, window_ms}` parsed from the `period,…` line. | `ProcessStats.java:2149-2166` |
| `provenance` | object | yes | `capture_kind: persisted_aggregate`, `freshness: stale` for both `AGGREGATED` entries. | — |

The checkin twin carries the window in machine-readable form:

```
period,<YYYY-MM-DD-HH-mm-ss>,<realtime_ms>,<realtime_ms>,partial
```

⇒ start clock, `mTimePeriodStartRealtime` ms, `elapsedRealtime` ms, **partial** (the
shutdown / sysprops / complete flags all clear), and a derived window length.

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
captures `EVENT LOG` early in the dump sequence, **before** the memory dumps that write these
tags. The `am_meminfo` tag is written *by the meminfo dump itself*, so in **every** bugreport
whose event log is captured before its memory sections — i.e. every bugreport 2.0 archive —
`am_meminfo` is structurally expected to be `0`. Measured alongside it, the event log does
carry other process-lifecycle tags at non-zero counts, which is what distinguishes "the log was
read" from "this one tag could not fire yet".

**A zero here is not evidence of a working memory-telemetry path**, and the schema says so via
`am_meminfo_absence_reason`.

#### Provenance and freshness

Three distinct clock families, each with its own `capture_kind`:

| Source | `capture_kind` | `captured_at_time_base` | Freshness |
| --- | --- | --- | --- |
| `dumpsys meminfo` summary block | `point_in_time_snapshot` | `dumpsys_section_snapshot` (from the section footer's `ending at:` stamp) | `in_window` |
| `PROCRANK` rows and footer | `point_in_time_snapshot` | `undetermined` | `undated` — raw smaps print no clock |
| `MEMORY INFO` (`/proc/meminfo`) | `point_in_time_snapshot` | `undetermined` | `undated` — **deliberately unstamped**; dumpstate suppresses the mtime for `/proc`, `/sys`, and `/d` paths |
| `dumpsys procstats` `CURRENT` | `background_sampler` | `dumpsys_section_snapshot` | `in_window` |
| `dumpsys procstats` `AGGREGATED_*` | **`persisted_aggregate`** | `dumpsys_section_snapshot` | **`stale`** — months |
| `/proc/vmstat` counters | `command_output` | `undetermined` | `undated` — cumulative since boot, but no clock printed |
| IOTOP | `command_output` | `undetermined` | `undated` |
| lmkd kill log line | `live_log` | `logcat_line` | `in_window` |

**The `undated` rows are the honest ones.** A genuine instantaneous reading with **no printed
clock at all** is not `stale`; it is unlabelled. Collapsing `undated` into `stale` would make a
whole class of legitimate evidence look like historical residue and train consumers to ignore
the field. Collapsing it into `now` is the exact bug this convention exists to prevent. Where a
snapshot can only be ordered, the schema records `line_start` — and says nothing about the
collector's intended order, because dumpstate runs sections on a thread pool and archive order
is **completion** order. Any field claiming to recover statement order is unproducible from any
archive.

**IOTOP's `0.00%` columns are a resolution floor, not a zero.**
`TimeToTgidPercent` (`iotop.cpp:36-39`) computes a percentage per thread over the sample window
and clamps at the top of the range, printed with two decimal places (`:255-265`). A `0.00%`
therefore means *below the print resolution*, **not** "no I/O". The same rows carry non-zero
throughput:

```
   <n> <thread_name>       <read>   <write>   <read>   <write>  <n>  0.00%  0.00% <sched_delay>  0.00% <sched_delay>
  <n> Blocking Thread      0   2308   2308      0     99  0.00%  0.00% 24.65%  0.00% 24.65%
    <n> <kernel_thread>         0      0      0      0      0  0.00%  0.00%  0.72%  0.00%  0.72%
```

The non-zero figure on `Blocking Thread` is **run-queue delay, not I/O** — column 3 of the
`delayed on` group is `sched` (`iotop.cpp:234`). This section answers "was anything *waiting*".
Its own duration footer exceeds the nominal sampling window; the excess is process setup and
the sampling loop's first iteration (`iotop.cpp:226-273`).

#### Worked example

Abbreviated; every value was measured on the reference archive.

```jsonc
{
  "schema_version": 1,
  "artifact": "memory",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "page_scanning_sections": [
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true },
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true }
    ],
    "page_scanning_total_ms": <page_scanning_total_ms>,
    "swap_timeline": [
      { "channel": "proc_meminfo", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": true,
        "zram_physical_kb": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "proc_meminfo", "section_title": "MEMORY INFO", "section_ordinal": 0,
          "line_start": <line>, "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none",
            "basis": "a raw /proc/meminfo read carries no clock and dumpstate suppresses the mtime for procfs paths, so this reading is undated",
            "threshold_seconds": 3600 },
          "collector_perturbed": false, "collector_perturbed_basis": "undated_source" } },
      { "channel": "top", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": null, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "top", "section_title": "CPU INFO", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock", "threshold_seconds": 3600 },
          "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "bugreport_procdump", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": <zram_physical_kb>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "procrank", "section_title": "PROCRANK", "section_ordinal": 0,
          "captured_at": null, "captured_at_time_base": "undetermined",
          "age_seconds_at_capture": null, "capture_kind": "point_in_time_snapshot", "freshness": "undated",
          "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock",
            "threshold_seconds": 3600 },
          "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },
      { "channel": "dumpsys_meminfo", "line_start": <line>,
        "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_kb_derived": false,
        "zram_physical_kb": <zram_physical_kb>, "section_duration_ms": <ms>,
        "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
          "section_ordinal": 0, "line_start": <line>,
          "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
          "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
          "freshness_basis": { "input": "the section footer's ending-at stamp",
            "basis": "inside capture.log_window", "threshold_seconds": 3600 },
          "collector_perturbed": true,
          "collector_perturbed_basis": "section_measured_during_dump" } }
    ],
    "swap_growth_kb": <swap_growth_kb>,
    "page_scanning_ms_in_interval": <page_scanning_ms_in_interval>
  },

  "system": {
    "total_ram_kb": <total_ram_kb>,
    "memory_level": "<status, verbatim>",
    "free_ram_kb": <free_ram_kb>,
    "free_ram_components_kb": { "cached_pss": <kb>, "cached_kernel": <kb>, "free": <kb> },
    "free_pct": <free_pct>,
    "used_ram_kb": <used_ram_kb>,
    "used_pss_kb": <used_pss_kb>,
    "kernel_used_kb": <kernel_used_kb>,
    "total_pss_kb": <total_pss_kb>,
    "lost_ram_kb": <lost_ram_kb>,
    "lost_ram_pct": <lost_ram_pct>,
    "tuning": { "memory_class_mb": <n>, "large_memory_class_mb": <n>, "oom_adj_score_kb": <kb>,
                "restore_limit_kb": <kb>, "low_ram": null, "high_end_gfx": true },
    "gpu_kb": <gpu_kb>,
    "dma_buf": { "total_kb": <kb>, "mapped_kb": <kb>, "unmapped_kb": <kb>, "heaps_kb": <kb>, "heaps_pool_kb": <kb> },
    "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
      "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
      "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "the section footer's ending-at stamp",
        "basis": "the body prints an uptime and realtime reading, so the reading cannot outlive this boot and falls inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "swap": {
    "swap_total_kb": <swap_total_kb>, "swap_used_kb": <swap_used_kb>, "swap_used_pct": <pct>,
    "zram_physical_kb": <zram_physical_kb>, "zram_physical_pct_of_ram": <pct>,
    "compression_ratio": <compression_ratio>, "compression_ratio_derived": true,
    "zram_block_io": { "read_ios": <n>, "read_sectors": <n>, "write_ios": <n>,
                       "write_sectors": <n>, "io_ticks": <n>,
                       "window": "cumulative since boot", "line_start": <line> },
    "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "processes": [
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "zswap_kb": <zswap_kb>, "zswap_estimated": true,
      "rss_total_kb": <rss_total_kb>, "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": <user_id>,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "rss_total_kb": <rss_total_kb>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "<process_name>", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb>, "rss_total_kb": <rss_total_kb>,
      "source_channel": "dumpsys_meminfo", "line_start": <line> },
    { "pid": <pid>, "process_name": "bugreport_procdump", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": 0, "rss_total_kb": <rss_total_kb>,
      "source_channel": "bugreport_procdump", "line_start": <line> },
    { "pid": <pid>, "process_name": "dumpstate", "user_id": null,
      "pss_total_kb": <pss_total_kb>, "swap_pss_kb": 0, "source_channel": "dumpsys_meminfo", "line_start": <line> }
  ],

  "categories": [
    { "category": "Native",      "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".apk mmap",   "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Dalvik",      "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".Heap",       "depth": 1, "parent_category": "Dalvik",   "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Dalvik Other","depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": "Unknown",     "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> },
    { "category": ".so mmap",    "depth": 0, "parent_category": null,        "pss_total_kb": <pss_total_kb>, "swap_pss_kb": <swap_pss_kb> }
  ],

  "procstats": [
    { "sub_section": "CURRENT", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "period_total_uptime": "<uptime as printed>",
      "period_complete": false, "period_flags": ["partial", "swapped-out-pss"], "aggregated_over_n": <n>,
      "checkin_period": { "start_clock": "<YYYY-MM-DD-HH-mm-ss>", "start_realtime_ms": <ms>,
                          "end_realtime_ms": <ms>, "complete": false, "window_ms": <ms> },
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "a single sample whose window falls inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "sub_section": "AGGREGATED_24H", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "aggregated_over_n": <n>,
      "period_complete": false, "period_flags": ["partial"],
      "backing_state_file": "<path to the on-disk state file the block was read from>",
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start precedes this boot by months; the window size in the header is a query parameter, not a fact about the data. capture_kind persisted_aggregate is the deciding input, not the wall clock alone.",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } },

    { "sub_section": "AGGREGATED_3H", "scope_label_verbatim": "<verbatim sub-section header>",
      "period_start_clock": "<YYYY-MM-DD HH:MM:SS>", "aggregated_over_n": <n>,
      "period_complete": false, "period_flags": ["partial"],
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start is identical to the longer-window block above, so the label in the header is not supported by the data",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } }
  ],

  "pressure_indicators": {
    "psi_memory": { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                    "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "psi_io":     { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                    "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "vmstat_oom_kill": <n>,
    "vmstat_pswpin": <n>, "vmstat_pswpout": <n>,
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "age_seconds_at_capture": null, "freshness": "undated",
      "freshness_basis": { "input": "none",
        "basis": "kernel cumulative counters carry no printed clock; they cannot outlive this boot but cannot be placed in the log window either",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "undated_source" }
  },

  "kills": [],
  "kill_evidence": {
    "lmkd_alive": true, "lmkd_using_psi": true, "lmkd_kill_log_matches": <n>,
    "vmstat_oom_kill": <n>, "am_kill_count": <n>,
    "am_low_memory_count": <n>, "am_low_memory_value": <n>,
    "am_meminfo_count": <n>, "am_pss_count": <n>,
    "provenance": { "capture_kind": "live_log", "captured_at": "<captured_at>",
      "captured_at_time_base": "logcat_line", "freshness": "in_window", "collector_perturbed": false }
  },

  "io": {
    "iotop": { "window_ms": <ms>, "line_start": <line>, "section_duration_ms": <ms>,
      "row_granularity": "thread", "sort_column": "<column>",
      "max_io_delay_pct": <pct>, "max_swap_delay_pct": <pct>,
      "max_sched_delay_pct": <pct>, "max_mem_delay_pct": <pct>,
      "resolution_floor_note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O. The same rows carry non-zero throughput, so a zero delay percentage must not be read as an absence of wait." },
    "storaged": { "line_start": <line>, "basis": "cumulative since process start",
                  "top_reader": "<uid or process, verbatim> <rchar> fg_rchar / <bytes> fg_rbytes" },
    "provenance": { "capture_kind": "command_output", "captured_at": null, "captured_at_time_base": "undetermined",
      "freshness": "undated",
      "freshness_basis": { "input": "none", "basis": "the reading carries no printed clock", "threshold_seconds": 3600 },
      "collector_perturbed": true,
      "collector_perturbed_basis": "section_measured_during_dump" }
  },

  "counts": {
    "am_kill_hits": {
      "pattern": "am_kill", "pattern_kind": "word_boundary", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": null,
      "coverage": { "window_start": "<window_start>", "window_end": "<window_end>",
                    "archive_window": "capture.log_window", "coverage_ratio": <coverage_ratio> },
      "regions": []
    }
  },

  "findings": [
    { "id": "mem-001", "kind": "no_pressure",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "memory_level": "<status, verbatim>" },
      "metrics": { "free_pct": <free_pct>, "psi_memory_full_avg10": <pct>, "vmstat_oom_kill": <n>, "lmkd_kill_count": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim total-RAM line carrying the status>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } },

    { "id": "mem-002", "kind": "swap_delta",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "page_scanning_ms_in_interval": <page_scanning_ms_in_interval> },
      "metrics": { "swap_growth_kb": <swap_growth_kb>, "page_scanning_total_ms": <ms>, "vmstat_oom_kill": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim swap/ZRAM summary line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "collection_swap_timeline", "archive_path": "<main_entry>",
        "section_title": "<section_title>", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "this is a delta between two snapshots; neither endpoint carries a clock, so the EDGE is undated even though its endpoints are ordered by line_start",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } },

    { "id": "mem-003", "kind": "stale_aggregate",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "capture_kind": "persisted_aggregate" },
      "metrics": { "age_seconds": <age>, "aggregated_over_n": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim procstats header block>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_procstats", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "persisted_aggregate", "freshness": "stale",
        "freshness_basis": { "input": "the printed Start time line",
          "basis": "the period start precedes this boot by months; capture_kind persisted_aggregate is the deciding input, not the wall clock alone",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "read_from_disk" } },

    { "id": "mem-004", "kind": "largest_allocator",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": "<boot_epoch>/<start_seq>", "event_time": null,
      "attributes": { "package_name": "<package_name>" },
      "metrics": { "pss_total_kb": <pss_total_kb>, "total_pss_kb": <total_pss_kb>, "native_category_kb": <pss_total_kb> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim per-process PSS row>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_meminfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "evidence_gaps": [
    { "field": "kill_evidence.am_meminfo_count", "reason": "REGION_EXCLUDED",
      "detail": "the event log is captured before the memory dump that writes this tag, so a zero here is structural for any archive whose event log precedes its memory sections and says nothing about whether memory telemetry works." },
    { "field": "kill_evidence.am_low_memory_count", "reason": "UNVERIFIABLE",
      "detail": "the byte value is read, but its meaning depends on a platform writer the analyzer cannot inspect. It must not be read as memory exhaustion without cross-checking memory_level and the lmkd kill count, which is why no interpreted field is published for it." },
    { "field": "collection.swap_timeline[].captured_at", "reason": "NOT_PRINNED",
      "detail": "a raw procfs read and a top footer print no clock, and dumpstate suppresses the mtime for procfs and sysfs paths. These readings are undated rather than stale; a snapshot that carries no clock is unlabelled, and collapsing undated into stale would make legitimate evidence look like historical residue." },
    { "field": "collection.swap_timeline[] ordering", "reason": "UNVERIFIABLE",
      "detail": "dumpstate runs sections on a thread pool and archive order is completion order, so no statement-execution order can be recovered from any archive. The timeline is therefore ordered by line_start and nothing claims to know the collector's intended order." },
    { "field": "processes[].zswap_kb", "reason": "PARSE_INDETERMINATE",
      "detail": "the per-process ZSwap column is a device-wide compression ratio apportioned across processes, not a measurement of that process. Every row therefore carries zswap_estimated: true and no consumer may treat the number as this process's own swapped bytes." },
    { "field": "swap.zram_orig_size_kb / zram_compressed_size_kb", "reason": "NOT_PRINNED",
      "detail": "no bugreport section exposes the original or compressed data size for a zram device, so no such field exists in this schema. Only the physical bytes used are published." }
  ],

  "notes": [
    "SWAP_DELTA_OVERLAPS_PAGE_SCANNING: a measurable amount of the swap growth falls between two identified timeline nodes whose interval contains page-scanning sections. The overlap is published as page_scanning_ms_in_interval; whether it explains the delta is a causal claim and belongs in README.md, not in this artifact.",
    "PROCSTATS_LABELS_ARE_QUERY_PARAMETERS: the longer and shorter aggregate headers resolve to the same printed start time, so the window named in the header is a query parameter rather than a fact about the data.",
    "ZSWAP_IS_APPORTIONED_NOT_MEASURED: every zswap_kb in this artifact is a device-wide ratio apportioned per process. Each row carries zswap_estimated: true."
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
- **Which of two `/proc/meminfo` reads fired.** Two dumpstate call sites read `/proc/meminfo`
  (`dumpstate.cpp:1689` and `dumpstate.cpp:1951`) and the archive contains one `MEMORY INFO`
  block, so the tool cannot tell which produced it. **No field depends on the answer** —
  `swap_timeline` is ordered by `line_start` and nothing claims statement order — so this is
  recorded here as an open question rather than as an `evidence_gaps[]` entry, because it does
  not make any published value indeterminate.
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
this archive the two point in opposite directions: a middling `TOTAL` with a dominant
`system_server` share looks like a load problem, while a high `irq` with `steal = 0` on every
core says the guest is being *interrupted*, not busy. A single score would say "CPU problem".
Two arrays say "compute-bound in `system_server`, host interrupt injection, and no lock
pathology worth reporting".

The other two separations are about **units**, not judgement:

- `utilization` is a **windowed ratio** whose window is arbitrary and must travel with it.
- `per_cpu_core` is **cumulative milliseconds since boot**, read at one `read_at_uptime_ms`.
  It is *not* a percentage and *not* the same window. Merging them produces a number that means
  nothing.
- `processes[]` shares `utilization`'s window exactly (one tracker delta), so it is
  attributable and gets a share field.
- `contention` is measured in **blocking events**, sampled at an unknown rate, over the **log
  window** — not the CPU window.

#### The `dumpsys cpuinfo` window — the finding that makes the CPU side provably clean

`AppProfiler.updateCpuStatsNow()` re-samples `/proc/stat` **only if ≥ 5 s have elapsed** since
the last sample (`AppProfiler.java:142`, `MONITOR_CPU_MIN_TIME = 5 * 1000`;
gate at `:2156-2160`), driven by a thread that has run since `system_server` started
(`:2358`, `new ProcessCpuThread("CpuTracker")`, loop at `:2285-2313`).
`CpuBinder.dumpCritical` merely prints **the last two samples it already has** — it does not
take a fresh one (`:2332-2335`).

The header carries the window explicitly (`ProcessCpuTracker.java:854-889`), in the form:

```
CPU usage from <n>ms to <n>ms ago (<start> to <end>):
```

From those printed numbers alone a tool can derive three facts, and all three are load-bearing:

- **window length** = `end − start`
- **the window ended before the dump printed it** — the section's own duration footer gives
  the print instant, so the gap is computable rather than asserted
- **whether that gap reaches back to the start of the dump** — compare it against
  `capture.captured_at` in `system.json`

**The CPU numbers therefore cannot be inflated by the collection, and this is checkable rather
than assumed.** `collector_perturbed: false` is only published when the measured window closed
*before* dumpstate began; otherwise the field is `true`. `capture_kind: background_sampler` is
what records why the gap exists — this channel prints pre-existing samples from a background
thread rather than taking a fresh one during the dump.

`top` (`dumpstate.cpp:1690`) is the opposite: it runs mid-dump, and its own `Swap:` line already
shows the collector's effect against the immediately preceding `/proc/meminfo` read.

#### `steal` is structurally unavailable in the global ratio

`dumpsys cpuinfo`'s `TOTAL` line is
`mRelUserTime + mRelSystemTime + mRelIoWaitTime + mRelIrqTime + mRelSoftIrqTime + mRelIdleTime`
(`ProcessCpuTracker.java:890-892` for the denominator, `:914` for the row) — it **excludes
steal**, because the aggregate `/proc/stat` read uses `SYSTEM_CPU_FORMAT` (`:126-134`) which
stops at `softirq`. Steal is available only per-CPU, via the `cpu_monitor` binder service
(`CpuInfoReader.java:69`, `:859-880`). **That asymmetry is why `cpu.json` needs both channels**,
and it is why `utilization[].pct_steal` is `null` for `dumpsys_cpuinfo` with a stated reason
rather than `0`.

#### Contention: three distinct channels, all real, none a count

A search for `Monitor contention with tid` returns **0**, while a case-insensitive search for
`Long monitor contention` returns **33**. These are **different producers with different
thresholds**, and a tool that conflates them reports a channel as broken when it is merely
silent. All three are published; none is a count of stalls.

**ART Java monitor.** Line shape (`art/runtime/monitor.cc:592-598` + `402-420`):

```
Long monitor contention with owner <owner_name> (<owner_tid>) at <owner_method>(<file>:<line>) waiters=<N> in <waiter_method> for <PrettyDuration>
```

`PrettyDuration` (`art/libartbase/base/time_utils.cc:53-75`) chooses ns/µs/ms/**s** and prints
at second granularity with fractions — hence `1.780s`, `181ms`, `12.3us`.

Threshold: `kLongWaitMs = 100 * kDebugThresholdFudgeFactor` (`monitor.cc:54`) and
`kDebugThresholdFudgeFactor = kIsDebugBuild ? 10 : 1` (`monitor.cc:53`) — so the floor is
**1000 ms on a debuggable build and 100 ms on a user build.** That is a real schema caveat, and
it is not knowable from the archive: *a stall well below the floor produces no line here, and
its absence is not evidence of health.*

Measured distribution of the long-contention lines (all wait strings): max **1.780 s**, min
105 ms; **6 events ≥ 1 s**, all inside two processes, all on the **same monitor**, all naming
the same owner method and dex, with `waiters=` climbing `0,1,2,3` on consecutive lines a few
milliseconds apart. A monotonic waiter queue on one monitor is a `MessageQueue.next()` shape.
**Zero of them are in `system_server`** — the `system_server` contention in this archive is the
*other* kind, the sampled channel below.

**`dvm_lock_sample` — the sampled channel.** `Monitor::LogContentionEvent` is emitted only with
probability equal to how far the wait exceeded the configured threshold (`monitor.cc:529-536`):

```cpp
if (wait_ms >= lock_profiling_threshold_) { sample_percent = 100; }
else                                        { sample_percent = 100 * wait_ms / lock_profiling_threshold_; }
if (sample_percent != 0 && (static_cast<uint32_t>(rand() % 100) < sample_percent)) { … LogContentionEvent(…) }
```

Field order (declared in the platform's event-log tag file, which **no bugreport contains** —
see the `evidence_gaps` entry for this channel):

```
1 process_name  2 is_sensitive  3 thread_name  4 wait_ms
5 waiter_file   6 waiter_line   7 waiter_method
8 owner_file    9 owner_line    10 owner_method  11 sample_percent
```

A representative line, verbatim and complete:

```
09-24 <HH:MM:SS.mmm>  <uid>   <pid>   <tid> I dvm_lock_sample: [<process>,<is_sensitive>,<thread>,<wait_ms>,<waiter_file>,<waiter_line>,<waiter_method>,<owner_file>,<owner_line>,<owner_method>,<sample_percent>]
```

⇒ `wait_ms`, `sample_percent`, and an `owner_file` of `-`, which is a "same file as the
waiter, save log space" shorthand (`monitor_android.cc:88-93`).

> **The count is not a count.** A wait logged with a high `sample_percent` implies roughly
> `lines / sample_percent` true events for waits of that length — but a *short* wait at the
> configured threshold logs only a small fraction of the time, so the *distribution* of unseen
> short waits is unbounded. **The schema records `sampled_event_count` and the observed
> `sample_percent` values, never a bare `contention_count`**, and `counting_caveat` is
> mandatory prose to that effect.

`lock_profiling_threshold_` comes from `Opt::LockProfThreshold`
(`art/runtime/runtime.cc:1589`; `runtime_options.def:123`) and is **zero on user builds** — in
which case `log_contention` is `false` and the whole branch is skipped (`monitor.cc:458`).

**ART runtime mutex — the rate-limited channel.** `Mutex::DumpStack`
(`art/runtime/base/mutex.cc:567-571`) is
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
| `load.uptime_ms_at_read` | integer \| null | yes | **Required whenever `load_1m` is present.** | header / `uptime` output |
| `online_cpus` | integer \| null | yes | Load is normalised by runnable CPUs, so a 1-minute average of 10.42 on 4 CPUs is 261 % runnable demand. | `ProcessCpuTracker.java:139-144` |
| `load_15m_implies_uptime_ms` | number \| null | yes | `load_15m × 15 × 1000` — the amount of history the 15-minute average implies it has seen. **Published so the contradiction is visible, not resolved.** | derived |
| `load_history_conflict` | boolean | yes | `true` when `load_15m_implies_uptime_ms` exceeds `uptime_ms_at_read`. **`uptime_ms_at_read` is `null` in the reference archive** — the header prints uptime only to minute granularity — so the comparison the boolean depends on is not available there; the value is then `null` and the conflict is recorded as an `evidence_gaps[]` entry with `reason: UNVERIFIABLE` rather than guessed. | derived |
| `provenance` | object | yes | | — |

> **Why `uptime_ms_at_read` is required.** Load averages are EWMAs decayed by the kernel's
> `LOAD_FREQ`. On a device with seconds of uptime the 1-minute average has only ever seen boot
> work, so it is arithmetically meaningless as a load signal. The reference archive is exactly
> that case, and it is *contradictory* in a second way: the header reports sub-minute uptime
> while `load_15m` implies roughly 700 s of history, which cannot exist on a boot that young.
> Nothing in the archive adjudicates why — see the `AMBIGUOUS_SOURCE` entry for
> `load.load_15m_implies_uptime_ms`. Either way the number cannot be a steady-state load, which
> is why this schema publishes the conflict (`load_history_conflict`) instead of a boolean
> verdict about it.

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

#### The two aggregate CPU rows disagree — correctly

Two channels, two aggregate rows, and they disagree. That disagreement is correct:

| Source | Normalisation |
| --- | --- |
| `dumpsys cpuinfo` `TOTAL` row | divided by a time span in which the device may be partly idle |
| `top` aggregate row | normalised across **all** online cores (`400%cpu` on a 4-core device) |

Both are correct over **different windows** (cpuinfo's is a short background sample taken
before the dump; `top`'s is its own mid-dump sample) and against **different denominators**.
Note also that the earlier reading of "9.2 user / 17k nice" was a **misreading**: the archive
says **`17% kernel`**, not nice — nice is the `12%` in the `top` line. **A schema that merges
these two rows into one "CPU %" is unsound**, so the schema keeps them as two entries of the
`utilization` array, each with its own window.

**On the high `irq`.** `stealTimeMillis = 0` on every core and `0%host` in `top` mean the guest
was **not** descheduled by the hypervisor. Combined with the product being an emulator
(`ro.ranchu`, and a fingerprint segment naming an emulator), the elevated `irq` in the
`cpuinfo` window is best explained as **host→guest device interrupt injection being accounted
in the guest's `irq` bucket, not by a guest driver defect**. **This is a labelled inference, not
a schema field** — the schema carries only the measured `irq` and `steal`. A finding may carry
`inference: true` and the supporting observations, and must then carry
`confidence: "inferred"`.

#### Worked example

```jsonc
{
  "schema_version": 1,
  "artifact": "cpu",
  "capture_ref": { "source": "system.json#/capture" },

  "collection": {
    "cpu_clean_channels": ["dumpsys_cpuinfo"],
    "cpu_perturbed_channels": ["top", "iotop", "dumpsys_meminfo", "bugreport_procdump"],
    "page_scanning_sections": [
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true },
      { "name": "<section_title>", "line_start": <line>, "line_end": <line>, "duration_ms": <ms>,
        "pages_scanned_per_pid_row": true }
    ],
    "page_scanning_total_ms": <page_scanning_total_ms>
  },

  "utilization": [
    { "channel": "dumpsys_cpuinfo", "line_start": <line>,
      "window": { "start_utc": "<captured_at>", "end_utc": "<captured_at>", "duration_ms": <ms> },
      "window_source": "section_body",
      "age_at_print_ms": { "start": <ms>, "end": <ms> },
      "total_pct": <pct>, "pct_user": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
      "pct_irq": <pct>, "pct_softirq": <pct>,
      "pct_nice": null, "pct_steal": null,
      "online_cpus": <n>,
      "null_reasons": {
        "pct_nice": "the aggregate cpu row this channel prints has no nice bucket; the ratio is structurally unavailable from these bytes",
        "pct_steal": "same; steal is only readable per-CPU via the cpu_monitor channel, never in this aggregate row"
      },
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "background_sampler",
        "freshness": "in_window",
        "freshness_basis": { "input": "the window header this section prints",
          "basis": "the sample window closed before dumpstate started and also falls inside capture.log_window, so it cannot include collection cost",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "channel": "top", "line_start": <line>,
      "window": { "start_utc": null, "end_utc": null, "duration_ms": null },
      "window_source": "unknown",
      "total_pct": null,
      "pct_user": <pct>, "pct_nice": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
      "pct_irq": <pct>, "pct_softirq": <pct>, "pct_steal": <pct>,
      "online_cpus": <n>,
      "threads_total": <n>, "running": <n>, "sleeping": <n>, "stopped": <n>, "zombie": <n>,
      "null_reasons": {
        "total_pct": "the aggregate row is normalised across all online cores, so it is not a 0-100 busy figure and cannot be compared with the other channel",
        "window": "this channel prints no window header, so its sampling interval is not derivable from the bytes"
      },
      "semantics_verified": false,
      "provenance": { "evidence_origin": "top", "section_title": "CPU INFO", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "the reading carries no printed clock; positioned only by its line_start within the archive",
          "threshold_seconds": 3600 },
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "per_cpu_core": [
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> },
    { "core_id": <n>, "is_online": true, "user_time_ms": <ms>, "nice_time_ms": <ms>,
      "system_time_ms": <ms>, "idle_time_ms": <ms>, "iowait_time_ms": <ms>,
      "irq_time_ms": <ms>, "softirq_time_ms": <ms>, "steal_time_ms": <ms>,
      "guest_time_ms": <ms>, "guest_nice_time_ms": <ms>, "busy_pct_since_boot": <pct>,
      "read_at_uptime_ms": <ms> }
  ],

  "processes": [
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "minor_faults": <n>, "major_faults": <n>, "is_thread_row": false,
      "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>", "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>,
      "minor_faults": <n>, "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<process_name>",
      "pct_total": <pct>, "minor_faults": <n>, "is_thread_row": false, "window_ref": "<utilization[].window_ref>" },
    { "pid": <pid>, "label": "<thread_name>", "pct_total": <pct>,
      "is_thread_row": true, "window_ref": "<utilization[].window_ref>" }
  ],

  "contention": {
    "art_java_monitor": {
      "event_count": <n>,
      "channel_present": true,
      "threshold_ms": <ms>,
      "threshold_derivation": "the reporting floor is a platform constant scaled by a build-variant factor. On this build the floor is high enough that a stall well below it emits no line at all, so the absence of short stalls is NOT evidence of health. The exact figure must be read from the build configuration, not assumed.",
      "wait_ms_max": <ms>, "wait_ms_min": <ms>,
      "events_over_threshold_1s": <n>,
      "events_in_system_server": <n>,
      "events_over_threshold_1s_in_system_server": <n>,
      "events": [
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_name": "<owner_name>", "owner_tid": <tid>,
          "owner_method": "<owner_method>(<signature>)",
          "owner_file": "<owner_file>", "owner_line": <n>,
          "waiters": <n>, "waiter_method": "<waiter_method>(<signature>)",
          "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim as ART chose to print it>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_method": "<owner_method>(<signature>)",
          "owner_line": <n>, "waiters": <n>, "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_method": "<owner_method>(<signature>)",
          "owner_line": <n>, "waiters": <n>, "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" },
        { "timestamp_utc": "<captured_at>", "process": "<process_name>",
          "pid": <pid>, "tid": <tid>, "owner_line": <n>, "waiters": <n>,
          "wait_ms": <ms>, "wait_verbatim": "<wait, verbatim>" }
      ],
      "events_truncated_in_example": <n>
    },
    "art_runtime_mutex": {
      "channel_present": true, "event_count": 0,
      "format": "<the channel's full line format, as documented above>",
      "short_format": "<the channel's abbreviated line format>",
      "zero_reason": "the channel is enabled and was searched, but no event satisfied its rate limit during the log window. Presence-with-zero is a result, not an absence."
    },
    "art_rate_limiting": {
      "java_monitor_is_sampled": true,
      "java_monitor_sampling_rule": "the emission probability is proportional to how far the wait exceeded the configured threshold, so a short wait is logged far less often than a long one",
      "java_monitor_sample_pct_observed": [<n>],
      "java_monitor_threshold_source": "a platform build option; not readable from a bugreport, so threshold_ms above must be published with threshold_derivation",
      "runtime_mutex_is_rate_limited": true,
      "runtime_mutex_rate_limit": "a fixed minimum interval between emitted events, scaled by a per-monitor attempt counter",
      "counting_caveat": "no field in contention is a contention count. The sampled channel's true population is unbounded below because sampling probability varies per wait duration, and the Java-monitor channel has a reporting floor above which shorter waits are unobservable. The published event_count is a count of lines, never of stalls."
    },
    "dvm_lock_sample": {
      "sampled_event_count": <n>, "in_system_server": <n>, "system_server_pct": <pct>,
      "field_order_source": "the positional layout this artifact assumes; it is declared in the platform's event-log tag file, which no bugreport contains, so each entry is also published in decomposed form",
      "example": { "timestamp_utc": "<captured_at>", "pid": <pid>, "tid": <tid>,
        "process_name": "<process_name>", "is_sensitive_thread": <n>, "thread_name": "<thread_name>",
        "wait_ms": <ms>, "waiter_file": "<File>", "waiter_line": <n>,
        "waiter_method": "<return type> <class>.<method>(<arg types>)",
        "owner_file": "-", "owner_line": <n>,
        "owner_method": "<return type> <class>.<method>(<arg types>)",
        "sample_percent": <n>,
        "owner_file_note": "an owner_file of '-' is a space optimisation meaning owner_file == waiter_file" }
    },
    "provenance": { "evidence_origin": "art_monitor_contention", "archive_path": "<main_entry>",
      "section_title": "SYSTEM LOG", "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
      "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
      "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "in_window",
      "freshness_basis": { "input": "logcat line stamp",
        "basis": "contention events are logged as they occur, inside capture.log_window",
        "threshold_seconds": 3600 },
      "collector_perturbed": false,
      "collector_perturbed_basis": "live_log_capture" }
  },

  "load": {
    "sources": [
      { "source": "bugreport_header", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>,
        "uptime_ms_at_read": null, "uptime_qualifier": "<uptime as printed in the header>",
        "line_start": <line> },
      { "source": "uptime_section", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>,
        "uptime_qualifier": "<uptime as printed>", "captured_at": "<captured_at>", "line_start": <line> },
      { "source": "dumpsys_cpuinfo", "load_1m": <n>, "load_5m": <n>, "load_15m": <n>, "line_start": <line> }
    ],
    "online_cpus": <n>,
    "uptime_ms_at_read": <ms>,
    "load_15m_implies_uptime_ms": <ms>,
    "load_history_conflict": true,
    "provenance": { "evidence_origin": "loadavg", "captured_at": "<captured_at>",
      "captured_at_time_base": "dumpsys_section_snapshot", "age_seconds_at_capture": <age>,
      "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "freshness_basis": { "input": "section body",
        "basis": "a kernel exponentially-weighted average, not a dump-scanned value",
        "threshold_seconds": 3600 },
      "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" }
  },

  "stall_signals": {
    "psi_cpu": { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                 "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "psi_io":  { "some": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> },
                 "full": { "avg10": <pct>, "avg60": <pct>, "avg300": <pct>, "total_us": <us> } },
    "iotop": { "window_ms": <ms>, "row_granularity": "thread", "sort_column": "<column>",
      "max_io_delay_pct": <pct>, "max_swap_delay_pct": <pct>,
      "max_sched_delay_pct": <pct>, "max_mem_delay_pct": <pct>,
      "resolution_floor_note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O; the same rows carry non-zero throughput",
      "provenance": { "capture_kind": "command_output", "captured_at": null, "freshness": "undated",
        "collector_perturbed": true,
        "collector_perturbed_basis": "section_measured_during_dump" } },
    "proc_churn": {
      "am_proc_start_count": <n>, "am_proc_died_count": <n>, "am_proc_bound_count": <n>,
      "am_uid_running_count": <n>, "am_uid_active_count": <n>,
      "window": { "start_utc": "<captured_at>", "end_utc": "<captured_at>",
                  "duration_seconds": <seconds>, "coverage_ratio": <coverage_ratio> },
      "rate_per_s": null,
      "rate_valid": false,
      "rate_suppressed_reason": "the event-log buffer covers only a fraction of the log window, and a per-boot churn rate computed over a window this short is dominated by the initial burst of runtime startup. The arithmetic result is withheld rather than published with a caveat, because rate_valid gates rate_per_s.",
      "provenance": { "capture_kind": "live_log", "captured_at": "<captured_at>",
        "captured_at_time_base": "logcat_line", "freshness": "in_window",
        "collector_perturbed": false,
        "collector_perturbed_basis": "live_log_capture" } }
  },

  "graphics": {
    "device_uptime_ms_at_capture": <ms>,
    "per_process": [
      { "pid": <pid>, "process_name": "<process_name>",
        "stats_since_ns": <ns>, "total_frames": <n>, "janky_frames": <n>, "janky_pct": <pct>,
        "p50_ms": <ms>, "p90_ms": <ms>, "p95_ms": <ms>, "p99_ms": <ms> }
    ],
    "window_note": "the per-process statistics window is independent per process and is not the same as utilization[].window; compare them only after reading stats_since_ns against device_uptime_ms_at_capture",
    "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
      "section_duration_ms": <ms>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
      "collector_perturbed": false, "collector_perturbed_basis": "section_measured_before_dump" }
  },

  "power_history": {
    "batterystats_reset_time": "<YYYY-MM-DD-HH-mm-ss>",
    "batterystats_duration_s": <seconds>,
    "batterystats_history_used_pct": <pct>,
    "powerstats_earliest_data_time": "<captured_at>",
    "freshness": "stale",
    "quarantine_reason": "battery and power history both begin before this boot. They describe a previous boot, are stored here only so a reader can see they exist, and MUST NOT be joined to utilization[] or to any uptime-derived field.",
    "provenance": { "capture_kind": "persisted_aggregate", "captured_at": "<captured_at>",
      "captured_at_time_base": "dumpsys_section_snapshot", "freshness": "stale" }
  },

  "counts": {
    "dvm_lock_sample_hits": {
      "pattern": " dvm_lock_sample:", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "these lines are NOT contention counts. They are a duration-weighted probability sample and are reported separately under contention.dvm_lock_sample as sampled_event_count",
      "coverage": { "window_start": "<captured_at>", "window_end": "<captured_at>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    },
    "long_monitor_contention_hits": {
      "pattern": "Long monitor contention", "pattern_kind": "substring", "case_sensitive": false,
      "raw_count": <n>, "matched_evidence": <n>, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "coverage": { "window_start": "<captured_at>", "window_end": "<captured_at>",
                    "archive_window": "capture.log_window", "coverage_ratio": 1.0 },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    },
    "monitor_contention_with_tid_hits": {
      "pattern": "Monitor contention with tid", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": 0, "matched_evidence": 0, "suppressed_noise": 0, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the channel is present and enabled; zero events satisfied its rate limit. Presence-with-zero is a result.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": []
    },
    "held_by_thread_hits": {
      "pattern": "held by thread", "pattern_kind": "substring", "case_sensitive": true,
      "raw_count": <n>, "matched_evidence": 0, "suppressed_noise": <n>, "not_emitted_by_platform": 0,
      "suppressed_noise_note": "the single match falls inside an archived trace section that predates this boot by months. Reporting it as a live lock-contention finding is the exact false positive this schema exists to prevent.",
      "coverage": { "window_start": null, "window_end": null, "archive_window": "capture.log_window", "coverage_ratio": null },
      "regions": [ { "archive_path": "<main_entry>", "section_title": "<section_title>", "section_ordinal": 0,
                     "line_start": <line>, "line_end": <line>, "line_count": <n> } ]
    }
  },

  "findings": [
    { "id": "cpu-001", "kind": "clean_window",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "channel": "dumpsys_cpuinfo", "capture_kind": "background_sampler" },
      "metrics": { "window_duration_ms": <ms>, "age_before_dump_ms": <ms>,
                   "pct_total": <pct>, "pct_user": <pct>, "pct_kernel": <pct>, "pct_iowait": <pct>,
                   "pct_irq": <pct>, "pct_softirq": <pct> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim CPU-usage window header>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpuinfo", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "background_sampler", "freshness": "in_window",
        "freshness_basis": { "input": "the window header plus the section footer",
          "basis": "the window closed before dumpstate started and inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false,
        "collector_perturbed_basis": "section_measured_before_dump" } },

    { "id": "cpu-002", "kind": "host_interrupt_injection",
      "severity": "info", "unconstrained_severity": "info", "confidence": "inferred",
      "inference": true,
      "process_key": null, "event_time": null,
      "attributes": { "inference_basis": ["ro.ranchu", "<fingerprint emulator segment>",
                                          "steal_time_ms = 0 on every core", "top reporting 0%host"] },
      "metrics": { "pct_irq": <pct>, "steal_time_ms": <ms>, "cores_measured": <n> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim per-core usage row>",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "dumpsys_cpu_monitor", "section_title": "<section_title>",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "dumpsys_section_snapshot",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "in_window",
        "freshness_basis": { "input": "the section footer's ending-at stamp",
          "basis": "inside capture.log_window",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "see cpu-001" } },

    { "id": "cpu-003", "kind": "load_history_conflict",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "load_history_conflict": true },
      "metrics": { "load_1m": <n>, "load_5m": <n>, "load_15m": <n>, "online_cpus": <n>,
                   "uptime_ms_at_read": <ms>, "load_15m_implies_uptime_ms": <ms> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim header uptime and load-average line>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "captured_at": "<captured_at>", "captured_at_time_base": "dumpstate_header",
        "age_seconds_at_capture": <age>, "capture_kind": "point_in_time_snapshot", "freshness": "now",
        "freshness_basis": { "input": "the uptime the header itself prints",
          "basis": "printed in the archive header itself",
          "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" } },

    { "id": "cpu-004", "kind": "contention_localized",
      "severity": "medium", "unconstrained_severity": "medium", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "localized_to": ["<process_name>", "<process_name>"],
                      "owner_method": "<owner_method>(<signature>)" },
      "metrics": { "art_java_monitor_event_count": <n>, "wait_ms_max": <ms>, "wait_ms_min": <ms>,
                   "events_over_threshold_1s": <n>, "events_in_system_server": <n>,
                   "dvm_lock_sample_sampled_event_count": <n>, "dvm_lock_sample_max_wait_ms": <ms> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim contention line>",
        "excerpt_truncated": true, "excerpt_bytes": <n>, "excerpt_complete": false },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "art_monitor_contention", "section_title": "SYSTEM LOG",
        "section_ordinal": 0, "line_start": <line>, "line_end": <line>,
        "captured_at": "<captured_at>", "captured_at_time_base": "logcat_line",
        "age_seconds_at_capture": <age>, "capture_kind": "live_log", "freshness": "in_window",
        "freshness_basis": { "input": "logcat line stamp",
          "basis": "logged as they occur, inside capture.log_window", "threshold_seconds": 3600 },
        "collector_perturbed": false, "collector_perturbed_basis": "live_log_capture" } },

    { "id": "cpu-005", "kind": "no_io_stall",
      "severity": "info", "unconstrained_severity": "info", "confidence": "confirmed",
      "process_key": null, "event_time": null,
      "attributes": { "note": "a 0.00% reading is a fixed-decimal resolution floor, not zero I/O; the same table carries non-zero throughput" },
      "metrics": { "max_sched_delay_pct": <pct>, "max_io_delay_pct": <pct>,
                   "psi_io_full_avg10": <pct>, "psi_cpu_full_avg10": <pct> },
      "evidence": { "archive_path": "<main_entry>", "kind": "line_range",
        "locator": { "line_start": <line>, "line_end": <line> },
        "excerpt": "<verbatim iotop row>",
        "excerpt_truncated": false, "excerpt_bytes": <n>, "excerpt_complete": true },
      "corroborating_evidence": [],
      "provenance": { "evidence_origin": "iotop", "section_title": "IOTOP", "section_ordinal": 0,
        "line_start": <line>, "line_end": <line>,
        "captured_at": null, "captured_at_time_base": "undetermined",
        "age_seconds_at_capture": null, "capture_kind": "command_output", "freshness": "undated",
        "freshness_basis": { "input": "none",
          "basis": "the reading carries no printed clock; its window comes from the tool's own sampling loop, not from a stamp",
          "threshold_seconds": 3600 },
        "collector_perturbed": true, "collector_perturbed_basis": "section_measured_during_dump" } }
  ],

  "evidence_gaps": [
    { "field": "utilization[].pct_steal", "reason": "NOT_PRINNED",
      "detail": "the aggregate CPU row this channel prints has no steal bucket, so steal is structurally unavailable from these bytes. It is never defaulted to 0; where the aggregate channel omits it, the value is null with a stated reason and steal is read per-CPU from the cpu_monitor channel instead." },
    { "field": "utilization[].pct_nice", "reason": "NOT_PRINNED",
      "detail": "same structural absence as steal for the aggregate row. Note the top channel does print a nice column, so the two entries of this array legitimately disagree on which buckets exist." },
    { "field": "contention.art_java_monitor.threshold_ms", "reason": "UNVERIFIABLE",
      "detail": "the reporting floor is a platform constant scaled by a build-variant factor. No bugreport records either value, so a tool cannot derive the floor from the archive; it must be supplied by configuration and published alongside threshold_derivation. Its practical consequence is that stalls below the floor are unobservable and their absence is not evidence of health." },
    { "field": "contention.dvm_lock_sample.example field order", "reason": "UNVERIFIABLE",
      "detail": "the payload is positional and its layout is declared in the platform event-log tag file, which no bugreport contains. The example is published in decomposed form so a reader need not trust the assumed slot order to use the numbers." },
    { "field": "load.load_15m_implies_uptime_ms", "reason": "AMBIGUOUS_SOURCE",
      "detail": "load_15m implies far more history than the printed uptime admits, and nothing in the archive adjudicates why — whether the kernel does not reset the load average across the first userspace processes or seeds the average from a previous boot. load_history_conflict publishes the conflict and load_1m must never be reported without uptime_ms_at_read." },
    { "field": "stall_signals.proc_churn.rate_per_s", "reason": "REGION_EXCLUDED",
      "detail": "the event-log buffer retains only a fraction of the log window, and a rate computed over so short an interval is dominated by the startup burst rather than by steady-state churn. rate_valid is false and the arithmetic result is withheld rather than published with a caveat." },
    { "field": "power_history", "reason": "UNVERIFIABLE",
      "detail": "both power sections describe a previous boot. They are quarantined in their own top-level object so that no join to utilization[] or to any uptime-derived field is structurally possible." }
  ],

  "negative_evidence": [
    { "scope": "ART runtime-mutex contention channel",
      "statement": "channel enabled and searched; 0 'Monitor contention with tid' events satisfied its rate limit during the log window",
      "source": { "archive_path": "<main_entry>", "section_title": "SYSTEM LOG", "section_ordinal": 0 },
      "authority": "archive_absence" }
  ],

  "notes": [
    "CPUINFO_WINDOW_PREDATES_DUMP: the dumpsys cpuinfo window closed before dumpstate started, because this channel prints pre-existing background samples rather than taking a fresh one. collector_perturbed is false for this channel and true for top, iotop, dumpsys_meminfo and bugreport_procdump.",
    "NO_STEAL_IN_GLOBAL_RATIO: the aggregate CPU row structurally cannot contain steal. pct_steal is null for that channel with null_reasons stating why; it is never 0 by default.",
    "CHANNELS_DISAGREE_BY_DESIGN: two CPU channels disagree because they measure different windows against different denominators. They are two entries of utilization[] and are never merged; neither is corrected toward the other.",
    "CONTENTION_IS_NEVER_A_COUNT: no field in contention is a count of stalls. The sampled channel's true population is unbounded below because sampling probability varies per wait duration, and the Java-monitor channel has a reporting floor above which shorter waits are unobservable.",
    "HELD_BY_THREAD_IS_STALE: the only match for this pattern in the archive falls inside an archived trace section that predates this boot by months, so it is counted as stale_region rather than as live contention.",
    "PROC_CHURN_RATE_GATED: the arithmetic per-boot churn rate over so short and so poorly covered a window is withheld rather than published with a caveat. rate_valid is false and rate_per_s is null.",
    "POWER_HISTORY_QUARANTINED: battery and power history describe a previous boot and are never joined to utilization[] or to any uptime-derived field."
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

