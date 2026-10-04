# Requirements

## Table of Contents

- [General](#general)
- [Core Library](#core-library)
- [MCP Server](#mcp-server)
- [Command-Line Interface](#command-line-interface)
- [Analysis Artifacts (DRAFT) `DRAFT-REQ-ART-02`](#draft-req-art-02)
  - [Shared artifact conventions](#shared-artifact-conventions)
  - [`README.md` `DRAFT-REQ-ART-01`](#draft-req-art-01)
  - [`system.json` `DRAFT-REQ-ART-03`](#draft-req-art-03)
  - [`apps.json` `DRAFT-REQ-ART-04`](#draft-req-art-04)
  - [`anr.json` `DRAFT-REQ-ART-05`](#draft-req-art-05)
  - [`crashes.json` `DRAFT-REQ-ART-06`](#draft-req-art-06)
  - [`memory.json` `DRAFT-REQ-ART-07`](#draft-req-art-07)
  - [`cpu.json` `DRAFT-REQ-ART-08`](#draft-req-art-08)
- [Requirement Diagram](#requirement-diagram)

---

## General

This project provides a command-line tool for analyzing Android bugreport ZIP archives. The primary requirements are:

- **Read-only ZIP access**: Must open bugreport ZIP archives without extracting files to disk. <a id="req-gen-01"></a> ([`REQ-GEN-01`](./requirement-diagram.md#req-gen-01))
- **Safety limits**: Enforce size and entry count limits to prevent resource exhaustion. <a id="req-gen-02"></a> ([`REQ-GEN-02`](./requirement-diagram.md#req-gen-02))
- **Evidence extraction**: Scan logcat/dumpstate for ANR, crashes, memory pressure, CPU contention, and other signals. <a id="draft-req-gen-03"></a> ([`DRAFT-REQ-GEN-03`](./requirement-diagram.md#draft-req-gen-03))
- **Artifact generation**: Produce JSON/NDJSON findings, a Markdown README, a manifest, and session segmentation. <a id="draft-req-gen-04"></a> ([`DRAFT-REQ-GEN-04`](./requirement-diagram.md#draft-req-gen-04))

These requirements drive the architecture and implementation decisions throughout the codebase.

## Core Library

The analysis engine is a library, not only a command-line tool: the `bugreport-insight` binary is one
consumer of it, and other Rust projects are expected to be others. Naming and packaging are left to
the implementation. Key requirements include:

- **Library distribution**: Distribute the core analysis engine as a library so other Rust crates can depend on it. <a id="req-lib-01"></a> ([`REQ-LIB-01`](./requirement-diagram.md#req-lib-01))

## MCP Server

`REQ-CLI-03` states that the `--mcp` flag exists. This section states what the server behind that
flag must do. The flag is the interface contract; this is the implementation contract. A reader who
has read only `REQ-CLI-03` knows the server can be started, and nothing about what it exposes, what
it refuses to do, or what an agent is allowed to conclude from a tool result.

Not every requirement here is draft. The server is in scope now, so most of these carry no
`DRAFT-` prefix — including MCP-06, MCP-08, and MCP-09, which constrain how the server presents
artifact-defined concepts. The artifact set they present is itself still
[`DRAFT-REQ-ART-*`](#draft-req-art-02), so those describe an interface whose content is not yet
committed; the interface is. `DRAFT-REQ-MCP-07` is the exception: exposing the artifacts as MCP
resources is not being implemented now.

### Tools

The seven tools in this section are specified under [Tools](#tools) below, each with its own ID.
Key requirements include:

- **Independent crate**: The server is its own crate, a peer of `cli`, and not a mode compiled into
  the CLI binary. It depends on `core` and the analyzer crates; `cli` and `mcp` never depend on each
  other. <a id="req-mcp-01"></a> ([`REQ-MCP-01`](./requirement-diagram.md#req-mcp-01))
- **Single-archive scope**: One server instance is bound to exactly one `BUGREPORT` archive for its
  lifetime. There is no multi-archive query, no archive list, and no way to open a second archive
  mid-session. <a id="req-mcp-02"></a> ([`REQ-MCP-02`](./requirement-diagram.md#req-mcp-02))
- **Read-only over the archive**: The server reads the archive in place and never extracts it, under
  the same size and entry-count limits as `REQ-GEN-01` and `REQ-GEN-02`. Exposing an archive to an
  agent does not relax the read-only guarantee. <a id="req-mcp-03"></a> ([`REQ-MCP-03`](./requirement-diagram.md#req-mcp-03))
- **Read-only tool surface**: No tool mutates the archive, the analysis output directory, or any other
  state. The CLI's job is to write artifacts; the server's job is to answer questions about one
  already-opened archive. <a id="req-mcp-04"></a> ([`REQ-MCP-04`](./requirement-diagram.md#req-mcp-04))
- **Tool naming and typing**: Every tool is named, described, and declared with a typed input schema
  and a documented result shape, so an agent can call it without reading the implementation. A tool
  that no backing capability justifies does not exist. <a id="req-mcp-05"></a> ([`REQ-MCP-05`](./requirement-diagram.md#req-mcp-05))
- **Bounded responses**: A tool result is bounded. An excerpt is capped at 512 bytes with its
  truncation flagged, a result set is capped and reports what it omitted, and a tool can never return
  an archive section verbatim. The failure mode being designed against is an agent whose context is
  consumed by one response. <a id="req-mcp-06"></a> ([`REQ-MCP-06`](./requirement-diagram.md#req-mcp-06))
- **Artifacts are exposed as resources**: The generated artifacts, including `README.md`, are readable
  as MCP resources, so an agent can cite the triage document rather than re-deriving one. What a
  resource returns is the artifact as written, not a summary of it. <a id="draft-req-mcp-07"></a> ([`DRAFT-REQ-MCP-07`](./requirement-diagram.md#draft-req-mcp-07))
- **Empty is not error**: A search that matched nothing returns a successful result stating that
  nothing matched, distinguishing that from a tool that failed, a section that was absent from the
  archive, and a section the tool was not permitted to read. The `section_status` enum survives into
  tool results. <a id="req-mcp-08"></a> ([`REQ-MCP-08`](./requirement-diagram.md#req-mcp-08))
- **Evidence discipline carries over**: A tool result states evidence as a locator — archive path,
  kind, line or byte range — exactly as the JSON artifacts do, and never as a copy of the source. A
  tool never returns a claim the corresponding artifact would not publish, including a causal claim
  linking a delta to its collector. <a id="req-mcp-09"></a> ([`REQ-MCP-09`](./requirement-diagram.md#req-mcp-09))
- **stdio is the required transport**: The server speaks MCP over stdio. Additional transports are
  optional and out of scope for this requirement set. <a id="req-mcp-10"></a> ([`REQ-MCP-10`](./requirement-diagram.md#req-mcp-10))

### Tools

The tool surface is the server's actual product, so each tool carries its own ID. The `ARC` family is
backed by `core` archive access and is in scope now; the analyzer families are backed by crates that
are themselves draft, so their IDs carry the `DRAFT-` prefix. The boundary is visible in the ID
rather than left to the reader to infer.

#### Archive tools <a id="req-mcp-arc-01"></a>

These seven map one-to-one onto the archive capabilities `core` already owns — listing, reading,
finding, grepping, and per-entry metadata — per [`architecture.md`](./architecture.md). None of them
runs an analyzer.

- **`archive_info`** — Identity and shape of the open archive: basename, size, sha256 over the stored
  bytes, entry count, `version.txt` body, and the main entry's name and size. This is the tool an
  agent calls first, and the one that answers "am I looking at the archive I think I am". The values
  are the same ones [`system.json`](#draft-req-art-03) publishes, but computed without an analyzer.
  <a id="req-mcp-arc-01"></a> ([`REQ-MCP-ARC-01`](./requirement-diagram.md#req-mcp-arc-01))
- **`list_entries`** — Lists the ZIP and TAR entries with their paths and sizes. Takes an optional
  path prefix so an agent can enumerate one subtree — `ANR/`, for instance — rather than the whole
  archive. Bounded by `REQ-MCP-06`, and reports what it omitted. <a id="req-mcp-arc-02"></a> ([`REQ-MCP-ARC-02`](./requirement-diagram.md#req-mcp-arc-02))
- **`find_entries`** — Filters entries by glob or regular expression on the path and returns matching
  paths only, with no content. The cheap way for an agent to discover what is in the archive before
  reading any of it. <a id="req-mcp-arc-03"></a> ([`REQ-MCP-ARC-03`](./requirement-diagram.md#req-mcp-arc-03))
- **`read_entry`** — Returns a bounded slice of one entry, addressed by line range or byte range.
  Never returns the whole entry; the cap and the truncation flag are mandatory, and a range beyond
  end-of-entry returns what exists plus the fact that it did. <a id="req-mcp-arc-04"></a> ([`REQ-MCP-ARC-04`](./requirement-diagram.md#req-mcp-arc-04))
- **`grep_archive`** — Searches entry contents for a literal or regular expression and returns
  matches as locators — path, line, and a bounded excerpt — never as raw section text. Because this
  is the tool most likely to exhaust an agent's context, it takes an explicit match cap and an
  explicit entry scope, and `REQ-MCP-06` applies on top of both. `REQ-MCP-08` governs the
  zero-match case. <a id="req-mcp-arc-05"></a> ([`REQ-MCP-ARC-05`](./requirement-diagram.md#req-mcp-arc-05))
  - **`entry_info`** — Returns everything known about **one** entry, addressed by path: compressed and
    uncompressed byte size, compression method, modification time, and its detected content type. This
    is where size becomes authoritative; `list_entries` carries sizes for orientation, and this is the
    tool an agent calls when the number decides something. It also reports whether the entry is a
    nested archive, whether it is readable, and whether its bytes were readable as text. <a id="req-mcp-arc-06"></a> ([`REQ-MCP-ARC-06`](./requirement-diagram.md#req-mcp-arc-06))
  - **`find_by_type`** — Filters entries by detected content type across the archive and returns
    matching paths with their sizes, so an agent can ask "what images are in here" or "which entries
    are binary" without reading anything. Detection is reported with its basis and never as a bare
    confident label: an entry is identified as text or binary from its bytes, and a more specific type
    is claimed only when the archive's own metadata or the byte content supports it. An
    unidentifiable entry is reported as such rather than guessed at. <a id="req-mcp-arc-07"></a> ([`REQ-MCP-ARC-07`](./requirement-diagram.md#req-mcp-arc-07))

#### Analyzer tools (DRAFT)

One tool per analyzer crate. Each is draft because the analyzer behind it is, under
[`DRAFT-REQ-GEN-03`](#draft-req-gen-03); naming them now fixes the interface without committing the
scanner. Their result shapes follow [`DRAFT-REQ-ART-*`](#draft-req-art-02), so they are draft twice
over.

- **`list_anr`** — ANR incidents from the `anr` crate, keyed on incident rather than on a trace file.
  <a id="draft-req-mcp-anr-01"></a> ([`DRAFT-REQ-MCP-ANR-01`](./requirement-diagram.md#draft-req-mcp-anr-01))
- **`list_apps`** — Installed packages and running processes from the `apps` crate, separated and keyed
  on `process_key` rather than `pid`. <a id="draft-req-mcp-app-01"></a> ([`DRAFT-REQ-MCP-APP-01`](./requirement-diagram.md#draft-req-mcp-app-01))
- **`list_crashes`** — Crash evidence from the `stacktrace` crate, with Java and native evidence kept
  separate and the de-duplication split reported. <a id="draft-req-mcp-crs-01"></a> ([`DRAFT-REQ-MCP-CRS-01`](./requirement-diagram.md#draft-req-mcp-crs-01))
- **`list_memory`** — Memory pressure findings from the `memory` crate, carrying the collector's own
  cost so a swap delta is not read as the device's. <a id="draft-req-mcp-mem-01"></a> ([`DRAFT-REQ-MCP-MEM-01`](./requirement-diagram.md#draft-req-mcp-mem-01))
- **`list_cpu`** — CPU findings from the `cpu` crate, separating windowed utilization from cumulative
  counters and from contention. <a id="draft-req-mcp-cpu-01"></a> ([`DRAFT-REQ-MCP-CPU-01`](./requirement-diagram.md#draft-req-mcp-cpu-01))

## Command-Line Interface

The command-line interface (CLI) should provide intuitive options for users to interact with the tool. `--mcp`, `--help`, and `--verbose` are in scope now; every requirement here that drives an analyzer is draft, because the analyzers are. Key requirements include:
* `bugreport-insight BUGREPORT`: Generate static analysis artifacts from the specified bugreport ZIP archive. There are some options: <a id="draft-req-cli-01"></a> ([`DRAFT-REQ-CLI-01`](./requirement-diagram.md#draft-req-cli-01))
  * `--mcp`: Start an MCP server for live inspection of the bugreport. saved. Default is create a new directory named `.<BUGREPORT>`. <a id="req-cli-03"></a> ([`REQ-CLI-03`](./requirement-diagram.md#req-cli-03))
  * `--sys`: Include system analysis in the generated artifacts. Default is true. <a id="draft-req-cli-02"></a> ([`DRAFT-REQ-CLI-02`](./requirement-diagram.md#draft-req-cli-02))
  * `--apps`: Include apps analysis in the generated artifacts. Default is true. <a id="draft-req-cli-06"></a> ([`DRAFT-REQ-CLI-06`](./requirement-diagram.md#draft-req-cli-06))
  * `--anr`: Include ANR (Application Not Responding) analysis in the generated artifacts. Default is true. <a id="draft-req-cli-07"></a> ([`DRAFT-REQ-CLI-07`](./requirement-diagram.md#draft-req-cli-07))
  * `--crash`: Include crash analysis in the generated artifacts. Default is true. <a id="draft-req-cli-08"></a> ([`DRAFT-REQ-CLI-08`](./requirement-diagram.md#draft-req-cli-08))
  * `--memory`: Include memory analysis in the generated artifacts. Default is true. <a id="draft-req-cli-09"></a> ([`DRAFT-REQ-CLI-09`](./requirement-diagram.md#draft-req-cli-09))
  * `--cpu`: Include CPU analysis in the generated artifacts. Default is true. <a id="draft-req-cli-10"></a> ([`DRAFT-REQ-CLI-10`](./requirement-diagram.md#draft-req-cli-10))
  * `--no-*`: Disable specific analysis types. For example, `--no-anr` will skip ANR analysis. <a id="draft-req-cli-11"></a> ([`DRAFT-REQ-CLI-11`](./requirement-diagram.md#draft-req-cli-11))
* `bugreport-insight --help`: Display usage information and available options. <a id="req-cli-04"></a> ([`REQ-CLI-04`](./requirement-diagram.md#req-cli-04))
* `bugreport-insight --verbose`: Enable verbose logging for debugging purposes. <a id="req-cli-05"></a> ([`REQ-CLI-05`](./requirement-diagram.md#req-cli-05))

## Analysis Artifacts (DRAFT) <a id="draft-req-art-02"></a> ([`DRAFT-REQ-ART-02`](./requirement-diagram.md#draft-req-art-02))

This section and every requirement in it are **draft**, because every one of them describes what an
analyzer produces. The `DRAFT-` prefix is not decoration: an
unprefixed ID is a requirement this project has committed to, and a `DRAFT-` ID may change or be
withdrawn without that being a breaking change. Do not implement against a `DRAFT-` ID, and do not
treat it as traceability evidence. `docs/artifact-schemas.md` is the companion specification and
moves with this section. [`DRAFT-REQ-GEN-04`](#draft-req-gen-04) in [General](#general) states the
same output-set commitment and carries the same prefix.

Each generated artifact answers one question, and states the evidence and the limits of that
answer. This section states what every artifact must let a consumer conclude, and the constraints
that make each conclusion defensible. Field-level detail — every key, every enum, every type —
belongs to the companion specification, [`docs/artifact-schemas.md`](./artifact-schemas.md), which
is written and versioned with this section. Nothing here restates a field table.

This example shows the expected structure of the generated artifacts from analyzing a bugreport ZIP archive:
```text
BUGREPORT.zip
.BUGREPORT/
└── insight/
    ├── README.md # Contains a summary of the analysis findings and instructions for further investigation.
    ├── system.json
    ├── apps.json
    ├── anr.json
    ├── crashes.json
    ├── memory.json
    └── cpu.json
```

### Shared artifact conventions

The rules in this subsection constrain `DRAFT-REQ-ART-03` through `DRAFT-REQ-ART-08` and are stated once. Each
artifact below adds only what is specific to it.

**Envelope.** Every JSON artifact is one object with exactly these thirteen top-level keys and no
others. Every key is required; there is no optional top-level key.

| Key | Requirement |
| --- | --- |
| `schema_version` | Integer version of this convention set. Not the bugreport format version. |
| `artifact` | One of `system`, `apps`, `anr`, `crashes`, `memory`, `cpu`. |
| `generator` | Tool name and version, host run start/finish, host timezone offset. Host clock, never device. |
| `source` | Archive identity: name, size, SHA-256, `format_version` read verbatim from `version.txt`, main entry name/size/line count, ZIP entry count, device identity block. |
| `capture` | The capture instant and context. `system.json` owns the canonical object; every other artifact carries `capture_ref = "system.json#/capture"` here instead of a copy. |
| `scope` | Which sections were scanned, matched, listed, skipped, and not found, plus read caps and truncation. |
| `collection` | What the collector itself did, built from dumpstate's per-section duration footers, so that "is this number self-inflicted?" is answerable. |
| `counts` | Match blocks. Never bare integers. See below. |
| `severity_summary` | Tally by severity. Must sum to `findings.length`. |
| `freshness_summary` | Tally by freshness state. Must sum to `findings.length`. |
| `negative_evidence[]` | Required array of assertions of absence. An empty array is itself a defect. |
| `findings[]` | The payload. May be empty; empty is legal only when `scope.sections_not_found` or `notes[]` explains it. |
| `notes[]` | Free-text caveats, each prefixed with a machine-parseable code. |

**Cross-artifact values are referenced, never recomputed.** The capture instant, uptime at capture,
the log window, and the boot instant are computed exactly once in the whole output set and live in
`system.json#/capture`. The same derived value must never appear twice with two slightly different
answers.

**Provenance is required on every finding and on every collection object.** Each carries
`captured_at` (RFC 3339 with an explicit offset, or `null`), `captured_at_time_base` naming which of
the archive's several time bases produced it, `evidence_time_source`, `capture_kind`, the archive
locator, `age_seconds_at_capture` (integer, or `null` when unknowable), `freshness`, and
`freshness_basis`. The capture instant of the archive is never substituted for the capture instant of
the evidence.

**Freshness is a four-state enum, assigned mechanically.** `now`, `in_window`, `stale`, `undated`.

- `now` and `in_window` are legal only when the evidence's own clock demonstrably falls inside the
  capture log window. There is no "unknown defaults to recent" member; the absence of that member is
  the requirement.
- The threshold is **3 600 s**, fixed in the convention and not configurable per run, chosen because
  the previous boot can be as little as roughly 8.5 h away and a wider threshold would label a
  previous-boot evidence set as current.
- The **cross-boot check is a required input** to every classification, not an optional refinement.
  When it fires, the finding is `stale` regardless of wall-clock age, and `freshness_basis` names it.
- `freshness_basis` states three things: which clock or check was used, why it decides the verdict,
  and the threshold it was compared against.
- `undated` is a legal, non-error value for sources that print no clock at all. Inventing a time is
  worse than admitting there is none.
- **A stale or undated finding is labelled, never dropped and never silently demoted.**

**Severity** is a closed five-level enum: `critical`, `high`, `medium`, `low`, `info`. Severity is
capped at `medium` when `freshness` is `stale` or `undated`, and the uncapped value is always
retained in a sibling `unconstrained_severity` so the cap is visible rather than invisible. A raw
pattern-hit count is never a severity input.

**No count is a bare number.** Every count is a match block carrying `pattern`, `pattern_kind`,
`case_sensitive`, `raw_count`, `matched_evidence`, `suppressed_noise`, `not_emitted_by_platform`, a
required `suppressed_noise_note` whenever anything is suppressed, and a `coverage` block. Two
invariants are checkable:

- `matched_evidence + suppressed_noise + not_emitted_by_platform == raw_count`.
- `coverage` is present whenever the channel observes a window narrower than the archive's log
  window, stating the channel window, the archive window, and the ratio. A count without its
  coverage is the same error as a count without its noise split, one level up.

Count-key suffixes are closed: `_hits` for raw string matches, `_records` for promoted findings,
`_entries` for archive entries.

**`negative_evidence[]` is required** and every entry states its `authority`: `platform_assertion`
when a platform string asserts absence — authoritative — or `archive_absence` when we searched and
found nothing — indicative only. The two are never conflated, and the array is never empty.

**Evidence is a locator, never a copy of the source.** Because the tool reads a ZIP without
extracting it, every finding points at an archive path, a kind, a line or byte range, and a bounded
excerpt capped at **512 bytes** with its truncation and completeness flagged. `findings` is the
counting unit; evidence is not.

**Process identity is `process_key`, never `pid`.** The primary form pairs a boot epoch with the
platform's own monotonic process-start counter; where that counter is unavailable, a
name/uid/ordinal form is used and recorded as such. PIDs are recycled within the log window, so a
`pid` is an observation attribute and never a key.

**Unknown is `null`, and keys are never omitted.** Build-type-dependent fields are tri-state: `null`
means *not observable* and never means `false`. A value the device rendered as a placeholder
sentinel is normalised to `null` at the artifact boundary.

**Section status is an enum, not a boolean.** `ok`, `empty`, `skipped_user_build`,
`skipped_root_denied`, `skipped_missing_source`, `skipped_budget`, `not_found`, `unreadable`. The
distinction between "none exist" and "we were not permitted to look" must survive into the artifact.

**The two `SYSTEM LOG` sections overlap.** In the reference archive they share 184 lines. Sections
are de-duplicated on parse, and every event-derived count and the log window itself are computed
over the de-duplicated union.

### `README.md` <a id="draft-req-art-01"></a> ([`DRAFT-REQ-ART-01`](./requirement-diagram.md#draft-req-art-01))

`README.md` answers, in order: what is this device, is it healthy now, what is the single most
important thing, how much of this can I trust, and where do I look next. It is a triage document, not
a dump. Every number in it is a pointer to a finding in a JSON artifact; it never asserts anything
the JSON does not.

Required sections, in this order: header and archive identity; capture context; verdict; top
findings; freshness ledger; noise register; evidence index; what this archive cannot tell you; and
reproduction inputs.

- **The verdict is derived only from findings that are `now` or `in_window`.** When no such finding
  exists, the verdict says so and names the stale and undated totals. It is never "healthy" by
  default.
- **The cold-boot banner is mandatory** when uptime at capture is under 300 s, before any finding
  list. On a cold boot the absence of a finding is weak evidence of absence on the device, and the
  banner must say that.
- **The freshness ledger is never omitted, and never omits the `undated` row.** Every row names its
  time base; a reader never sees a time without its basis. A previous-boot evidence set is called
  out in plain language.
- **A stale finding may not appear without its age**, and its row is subordinate to any current row
  of the same nominal severity. When any finding is more than 24 h old, a warning stating how much
  of the report describes the device as it was, rather than as it is, follows the header.
- **The noise register is never omitted.** It reports every match block: what was searched, the raw
  hit count, what was confirmed, what was suppressed, and the leading suppression reason. Outside
  this register, counts are presented as records, never as raw hits. A channel that matched nothing
  while corroborating evidence for the condition exists elsewhere gets a mandatory footnote.
- **The top-findings table is capped at ten rows** and carries the freshness state and the age
  inline. Every row links to its JSON finding and to its evidence locator.
- **The two overlapping `SYSTEM LOG` sections and their de-duplication are stated** wherever an
  event count is presented.
- **The document is bounded by construction**: excerpts are shown under the 512-byte evidence cap
  with an explicit elision marker, and the target length is at most 400 lines.

### `system.json` <a id="draft-req-art-03"></a> ([`DRAFT-REQ-ART-03`](./requirement-diagram.md#draft-req-art-03))

`system.json` is the device fact sheet: what was captured, when, on what, and by which build. It
answers "is this the device I think it is, and is this window of time trustworthy", not "what went
wrong". It therefore carries no ranked list of problems, and its role is to be the foundation every
other artifact is interpreted against. `DRAFT-REQ-ART-02` places it in the output set; `DRAFT-REQ-ART-03` fixes
it as the single owner of capture context.

Payload keys, in addition to the thirteen-key envelope: `capture` as the canonical object,
`build`, `product`, `device`, `kernel`, `users`, `displays[]`, `hal`, `storage`, `sections[]`,
`archive_entries`, and `evidence_gaps[]`.

- **`capture` is computed here and nowhere else.** The capture instant, its time base, uptime at
  capture, the log window and the boot instant are established once. The other five artifacts point
  here through `capture_ref` and re-derive none of it.
- **The log window is computed over the de-duplicated log sections** and must state whether the
  archive's log sections are contiguous, because they are not in the general case.
- **Section status is load-bearing.** A root failure can end the dump while still producing a
  well-formed archive, so a consumer must be able to tell "no ANRs exist" from "we were not allowed
  to look". A silently truncated dump must be detectable from the artifact.
- **Build-type-dependent fields are tri-state.** The gate is a single primitive, a literal compare of
  the build type against `"user"` whose default is `"user"`. There is no `userdebug`-versus-`eng`
  distinction anywhere in dumpstate — both take the same inclusive branch — so no field may depend on
  one. `null` means not observable, and pairs with a section status of `skipped_user_build`; it
  never degrades to `false`.
- **A failed property read silently degrades a debug bugreport to user level.** Where a fact was
  inferred from a fallback path, the path used is recorded.
- **The set of dumpsys sections is resolved at runtime and is open.** No artifact may carry a
  hardcoded section allowlist.
- **`format_version` is read verbatim and never branched on.** Its absence means format 0 and is
  reported as such rather than defaulted.
- **Every fact carries an evidence locator**, and staleness of copied archive entries is computed
  once, here, so no other artifact re-derives it.

### `apps.json` <a id="draft-req-art-04"></a> ([`DRAFT-REQ-ART-04`](./requirement-diagram.md#draft-req-art-04))

`apps.json` answers two questions and refuses to blur them: what is installed and who is it for, and
which process ran on whose behalf at which moment. Its central problem is that one bugreport captures
a moving system in a single snapshot, so the same failing app appears under many PIDs within the log
window. It is both an inventory and a process table, joined by a stored, first-class link rather than
by name inference.

Payload keys: `capture_ref`, `inventory` (with `packages[]`, `processes[]`, `shared_uids[]` and a
precomputed index), `channels[]`, `lifecycle_events[]`, `activity_events[]`, `attribution`, and
`evidence_gaps[]`.

- **Process identity is `process_key`.** A `pid` is carried as an observation attribute only. In the
  reference archive one crashing process occupies hundreds of PIDs inside a 50 s window, so keying on
  `pid` manufactures hundreds of processes that are one app failing.
- **The primary key of a package entry is `package_name`**, and of a user installation the pair of
  package name and user id. The per-user app id is stored as a fact and used only to accelerate a
  join: it is reassigned across reinstall and shared across the platform, so it is never an identity.
- **The link between a process and a package is stored, not recomputed.** A process name is not a
  package name; the platform's own package list for the record is the authoritative mapping, and it
  is a many-to-many relation.
- **Every evidence channel states its own observed window and whether it covers the archive's log
  window.** The real process-event channel covers roughly 20 s of a 65 s log window, so its death
  count cannot bound anything over the full window; where coverage is partial, the match block
  carries the required coverage fields.
- **An empty package list is a permission question before it is a parsing question.** The package
  dump is suppressed unless the caller holds two permissions, and the artifact must be able to say so
  rather than reporting an empty inventory as a device with no apps.
- **A `tier` label is a label, never a filter.** Platform packages with real crash relevance must not
  be excluded by a third-party filter, and the raw evidence the label is derived from is retained so
  the label is recomputable.
- **Observations that cannot be attributed are listed with a reason**, never guessed and never
  silently dropped.

### `anr.json` <a id="draft-req-art-05"></a> ([`DRAFT-REQ-ART-05`](./requirement-diagram.md#draft-req-art-05))

`anr.json` answers: did an application stop responding, when, and what thread dumps exist, of what
age, from which process. It deliberately does not answer "is the device responsive now", because a
bugreport cannot. It reports the ANR **evidence** in the archive and separately reports the ANR
**incidents** that the evidence supports, and those two are frequently not the same set.

Payload keys: `capture_ref`, `channel_scan[]`, `anr_incidents[]`, `trace_files[]`,
`last_anr_trace`, `negative_evidence[]`, and `evidence_gaps[]`.

- **The unit is the incident, not the file and not the process.** Trace files and dumped processes
  are child arrays of an incident, each with its own provenance. The reference archive holds 65
  retained trace files, one of which contains 31 process dumps, spanning 4 h 19 m on a boot 126 days
  before capture: that is one incident plus a previous boot's residue, not 65 or 31 ANRs.
- **Disk evidence alone never creates an incident.** An incident is emitted only when an in-window
  dated declaration supports it. Retained files are evidence to investigate, and are labelled with
  their age. On the reference archive the correct output is an empty incident list beside 65 stale
  trace files, with the platform's own assertion that no ANR has occurred since boot recorded as
  authoritative negative evidence.
- **The declaration channel is the on-disk inventory and the archived traces, not the logcat tag.**
  The `ANR in ` tag matches zero times on the reference archive while 65 real traces exist in the
  archive; it is a secondary channel and its zero count is reported with a mandatory note rather
  than as evidence of absence.
- **The newest-trace section header is the freshness linchpin and must be parsed.** It carries the
  file path, the millisecond creation time encoded in the filename, and the file mtime in one line.
  The per-process stamps inside the trace make staleness provable rather than inferred, and a monotonic
  cross-boot reference taken from them is a required provenance input.
- **Retention is event-driven, not clock-driven.** Pruning runs only when a new file is written, so a
  quiet device can exceed the nominal retention ring. The file count is therefore never the incident
  count, and the artifact must not present it as one.
- **A numeric general timeout is not sourceable** on the current platform. The verbatim reason and
  its origin are carried instead.
- **Each evidence channel reports its own completeness**, because a rate-limited or ring-wrapped
  channel cannot bound what happened.

### `crashes.json` <a id="draft-req-art-06"></a> ([`DRAFT-REQ-ART-06`](./requirement-diagram.md#draft-req-art-06))

`crashes.json` answers: what crashed, how often, in what pattern, and how old is the evidence. Its
central modelling problem is that one archive records crashes in several mutually inconsistent
channels with different completeness guarantees, so that a single file can hold a live crash loop and
a months-old native-crash set at once. The artifact must keep them apart and must not let a lossy
channel's count stand in for the truth.

Payload keys: `capture_ref`, `channel_scan[]`, `java_crash_loops[]`, `java_crash_incidents[]`,
`native_crash_groups[]`, `native_crash_incidents[]`, `negative_evidence[]`, and `evidence_gaps[]`.

- **Exactly one channel is authoritative for the headline count** and says so. Every other channel
  carries its own completeness and its own coverage.
- **Java and native are separate buckets by design.** A live Java crash loop and a previous-boot
  tombstone set must never be merged, ranked together, or summarised by one number.
- **Denormalised duplicates are counted, not silently absorbed.** On the reference archive 274 raw
  hits resolve to 271 distinct crashes; the 3 extras are logcat copies inlined into the dropbox dump.
  The split is reported in the match block with a note naming the cause, and the headline number is
  the de-duplicated one.
- **Loops are keyed on a normalised signature** — exception class or signal, plus a top frame with
  line numbers stripped — never on pid and never on file. The artifact must report the number of
  distinct occurrences, the raw line hits, and the gap statistics that justify the loop
  classification, plus the classification's thresholds by name.
- **Process identity is `process_key`.** A restart loop yields a new PID per attempt for one process
  identity, so an incident record cannot be keyed on pid.
- **`capture_kind` decides staleness, not elapsed time alone.** A crash-buffer line is a live log and
  a tombstone file is a disk artifact; the previous-boot tombstone set is therefore `stale` while the
  live loop is not, and `freshness_basis` names the kind.
- **Manager-kill and rescue-party messages are indicative, not events.** They are corroboration, and
  their counts must never be presented as crash counts.
- **Substring noise is reported, not hidden.** The raw tombstone-string hit count in the main entry
  resolves to zero real tombstones, because the real tombstones are archived entries rather than main
  text. Both numbers, and the suppression split, are required.

### `memory.json` <a id="draft-req-art-07"></a> ([`DRAFT-REQ-ART-07`](./requirement-diagram.md#draft-req-art-07))

`memory.json` answers three separable questions and refuses to let one answer stand for another:
was the device under memory pressure at capture and was it reacting; who was holding the memory; and
what kind of memory it was. It also carries an explicit verdict on whether the collection itself
perturbed the measurement, because in the reference archive it did.

Payload keys: `capture_ref`, `channels[]`, `collection`, `system`, `swap`, `processes[]`,
`categories[]`, `kernel_memory`, `procstats[]`, `pressure_indicators`, `kills[]`, `kill_evidence`,
`io`, and `evidence_gaps[]`.

- **The collector's own cost is a first-class, measured quantity.** Two sections in the reference
  archive walk every process's page tables and together account for roughly 207 MiB of swap growth
  across the dump. `collection` carries those sections with their measured durations, built from the
  duration footers dumpstate already prints.
- **The perturbation assessment is computed, not asserted**, over a closed set of verdicts: collector,
  device, indeterminate, or no growth observed. Any finding that cites swap growth must carry the
  perturbation flag and its basis, or be dropped.
- **Snapshot deltas are edges between identified nodes.** Swap lives in an ordered snapshot list
  keyed by section execution order, and no bare scalar exists that a consumer could compare against
  another artifact and read as a rate. The reference archive holds four swap snapshots, not two, and
  ordering is by position in the dump rather than by value.
- **Persisted aggregates are never merged with each other or with live readings.** Each aggregate
  entry carries its own window and its own completeness flag, and an aggregate whose window long
  predates the capture is `stale` with the uncapped severity retained.
- **Every derived number carries its derivation.** The total-PSS figure in the reference archive does
  not appear in the archive at all; it is computed from two different lines of one dump, and the
  formula travels with the value so a reader never has to trust it.
- **A zero count carries an absence reason.** Event-log memory counters are structurally zero because
  the event log is captured before the meminfo dump writes them; a zero there is not evidence of a
  working telemetry path.
- **An empty kill list is a result, not an absence**, and is backed by the structured kill evidence
  that supports it.
- **Units are explicit.** `_kb` and `_mb` are decimal; `_kib` and `_mib` are binary. Values the
  platform prints in a 1024-based unit are recorded in the binary unit and are not silently
  converted.

### `cpu.json` <a id="draft-req-art-08"></a> ([`DRAFT-REQ-ART-08`](./requirement-diagram.md#draft-req-art-08))

`cpu.json` answers how much CPU was used, by whom, over what window, and separately whether anything
was blocked. Those are different failure modes with different evidence, and folding them into one
"CPU problem" number is prohibited: in the reference archive a total utilisation figure with a large
`system_server` share coexists with high interrupt time and zero steal on every core, which together
mean the guest is being interrupted rather than being busy.

Payload keys: `capture_ref`, `channels[]`, `collection`, `utilization[]`, `per_cpu_core[]`,
`processes[]`, `contention`, `load`, `stall_signals`, `graphics`, `power_history`, and
`evidence_gaps[]`.

- **Every ratio travels with its window.** The utilisation window is chosen by a background sampler
  and is not the log window, so it must be stated alongside the ratio.
- **A measurement taken by a background sampler is never severity-capped for being old.** In the
  reference archive the sampler window ends before the dump and the reading is clean; `capture_kind`
  records that this is a background sampler rather than a snapshot or a persisted aggregate, and the
  kind — not elapsed time alone — is what a staleness decision turns on.
- **Cumulative per-core counters and windowed ratios are separate arrays with different units.** The
  per-core figures are milliseconds since boot and are neither a percentage nor the utilisation
  window.
- **A sample count is not an event count.** Contention samples are collected at an unknown rate over
  the log window; the artifact must carry the sampling basis, and the number of samples must never be
  presented as the number of contentions.
- **Partial channel coverage is reported.** The real process-event channel covers a small fraction of
  the archive's log window, which is why a few dozen recorded process deaths coexist with hundreds
  of actual ones. Every count derived from that channel carries the required coverage fields.
- **Resolution floors are not zeros.** Columns the platform renders with fixed precision are
  recorded as below resolution, not as zero.
- **Two total-utilisation figures that disagree are both published**, each with its own window and
  provenance, and are never reconciled into a single value.
- **Persisted power history is quarantined, not deleted.** It is carried with its own window and its
  own freshness rather than being mixed into the live picture.

## Requirement Diagram

Every requirement above carries a stable ID, printed as a clickable badge that jumps to its row in
the [Requirement Index](./requirement-diagram.md#requirement-index) of [Requirement Diagram](./requirement-diagram.md),
the traceability view mapping each requirement to the system element satisfying it along with its
risk and verification method. Each index row links back to the requirement it visualizes.
