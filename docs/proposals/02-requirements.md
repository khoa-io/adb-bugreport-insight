# Draft requirements

> Remaining `DRAFT-` requirements are proposals; superseded IDs remain here for provenance.
> [`REQ-INSIGHT-*`](../swe.1-requirements/02-software-requirements.md#static-insight-artifacts) is the binding single-report contract.
> Unlike [committed IDs](../swe.1-requirements/02-software-requirements.md), `DRAFT-` IDs may change or be withdrawn without breaking changes.
> **Do not implement against a `DRAFT-` ID, and do not treat one as traceability evidence.**
> See [`../traceability.md`](../traceability.md) for how draft status is recorded in the join.

The remaining draft requirements cover analyzer MCP tools, CLI toggles, and multi-format archive access.
The committed set covers archive reading, MCP serving, and single-report static insight generation.

## The committed / draft boundary

| | Count | Where |
| --- | --- | --- |
| Committed | 29 | [Committed requirements](../swe.1-requirements/02-software-requirements.md) |
| Draft | 16 | this file |
| Retired draft IDs reserved for traceability | 11 | this file and traceability |
| **Total identifiers, including superseded draft IDs** | **56** | [Traceability](../traceability.md) |

The boundary is shipped behavior, not size: ZIP reading, MCP queries, and `UC-03` static artifact generation are committed.

## Inventory

| ID | States | Proposed element |
| --- | --- | --- |
| `DRAFT-REQ-GEN-03` | Superseded by `REQ-INSIGHT-04` for the six single-report analysis domains | `ARCH-INSIGHT-ANALYZERS` |
| `DRAFT-REQ-GEN-04` | Single-report output subset superseded by `REQ-INSIGHT-03`; batch NDJSON, manifest, and session segmentation remain draft | `DRAFT-ARCH-SUPPORT` |
| `DRAFT-REQ-MCP-07` | Superseded by `REQ-MCP-07` | `ARCH-MCP-RES` |
| `DRAFT-REQ-MCP-TOOLS-ANALYSIS` | Analyzer tool results publish no causal claim the artifacts would not | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-ARC-08` | `PatternMode::Regex` | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-ARC-09` | Multi-format archives — TAR, nested ZIP | — |
| `DRAFT-REQ-MCP-{ANR,APP,CRS,MEM,CPU}-01` | The five analyzer tools | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-CLI-01` | Superseded by `REQ-INSIGHT-01` | `ARCH-CLI-INSIGHT` |
| `DRAFT-REQ-CLI-02`, `-06` … `-11` | Deferred per-analysis CLI toggles | `DRAFT-ARCH-CLI-CMD` |
| `DRAFT-REQ-ART-01` … `-08` | Superseded by `REQ-INSIGHT-03` … `REQ-INSIGHT-05` | `ARCH-INSIGHT-WRITER` |

Field-level detail for the promoted artifact set is in [`artifact-schemas/`](artifact-schemas/index.md).

## Promoted artifact contract; superseded draft IDs retained <a id="draft-req-art-02"></a> ([`DRAFT-REQ-ART-02`](../traceability.md#draft-req-art-02))

The single-report artifact behavior below is committed under [`REQ-INSIGHT-03`](../swe.1-requirements/02-software-requirements.md#req-insight-03) and [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04).
The retired `DRAFT-REQ-ART-*` identifiers remain reserved aliases; their successors define current acceptance scope.
[`artifact-schemas/`](artifact-schemas/index.md) is the normative field-level contract.

Each artifact answers a question with evidence and limits; this retained proposal text explains consumer conclusions now committed by `REQ-INSIGHT-04`.
[`artifact-schemas/`](artifact-schemas/index.md) owns the normative keys, enums, types, and invariants.

Proposed generated output structure:
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

The rules that constrain each JSON artifact are specified once, as field schemas, in [`artifact-schemas/01-envelope.md`](artifact-schemas/01-envelope.md) and [`artifact-schemas/02-evidence-conventions.md`](artifact-schemas/02-evidence-conventions.md).
Requirements define conclusions; schemas define supporting shapes.

Four shared constraints:

- **Evidence is a locator, never a copy.**
  Every finding carries an archive path, a kind, a line or byte range, and an excerpt capped at **512 bytes** with truncation flagged.
- **Freshness is a four-state enum assigned mechanically** — `now`, `in_window`, `stale`, `undated`.
  Unknown timestamps never default to recent.
  A stale or undated finding is labelled, never dropped and never silently demoted.
- **No count is a bare number.**
  Every count is a match block separating confirmed evidence from suppressed substring noise, with the channel's window coverage.
- **Process identity is `process_key`, never `pid`.**
  PIDs are recycled within the log window, so a `pid` is an observation attribute and never a key.

### `README.md` <a id="draft-req-art-01"></a> (source proposal superseded by [`REQ-INSIGHT-03`](../swe.1-requirements/02-software-requirements.md#req-insight-03) and [`REQ-INSIGHT-05`](../swe.1-requirements/02-software-requirements.md#req-insight-05))

`README.md` answers, in order: what is this device, is it healthy now, what is the single most important thing, how much of this can I trust, and where do I look next.
It is a triage document, not a dump.
Every number in it is a pointer to a finding in a JSON artifact; it never asserts anything the JSON does not.

Required sections, in this order: header and archive identity; capture context; verdict; top findings; freshness ledger; noise register; evidence index; what this archive cannot tell you; and reproduction inputs.

- **The verdict is derived only from findings that are `now` or `in_window`.**
  When no such finding exists, the verdict says so and names the stale and undated totals.
  It is never "healthy" by default.
- **The cold-boot banner is mandatory** when uptime at capture is under 300 s, before any finding list.
  On a cold boot the absence of a finding is weak evidence of absence on the device, and the banner must say that.
- **The freshness ledger is never omitted, and never omits the `undated` row.**
  Every row names its time base; a reader never sees a time without its basis.
  A previous-boot evidence set is called out in plain language.
- **A stale finding may not appear without its age**, and its row is subordinate to any current row of the same nominal severity.
  When any finding is more than 24 h old, a warning stating how much of the report describes the device as it was, rather than as it is, follows the header.
- **The noise register is never omitted.**
  It reports every match block: what was searched, the raw hit count, what was confirmed, what was suppressed, and the leading suppression reason.
  Outside this register, counts are presented as records, never as raw hits.
  A channel that matched nothing while corroborating evidence for the condition exists elsewhere gets a mandatory footnote.
- **The top-findings table is capped at ten rows** and carries the freshness state and the age inline.
  Every row links to its JSON finding and to its evidence locator.
- **The two overlapping `SYSTEM LOG` sections and their de-duplication are stated** wherever an event count is presented.
- **The document is bounded by construction**: excerpts are shown under the 512-byte evidence cap with an explicit elision marker, and the target length is at most 400 lines.

### `system.json` <a id="draft-req-art-03"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`system.json` establishes device identity, capture/build context and time-window trust, without ranking problems.
`DRAFT-REQ-ART-02` places it in the output set; `DRAFT-REQ-ART-03` fixes it as the single owner of capture context.

Payload keys, in addition to the fourteen-key envelope: `capture` as the canonical object, `build`, `product`, `device`, `kernel`, `users`, `displays[]`, `hal`, `storage`, `sections[]`, `archive_entries`, and `evidence_gaps[]`.

- **`capture` is computed here and nowhere else.**
  The capture instant, its time base, uptime at capture, the log window and the boot instant are established once.
  The other five artifacts point here through `capture_ref` and re-derive none of it.
- **The log window is computed over the de-duplicated log sections** and must state whether the archive's log sections are contiguous, because they are not in the general case.
- **Section status is load-bearing.**
  A root failure can end the dump while still producing a well-formed archive, so a consumer must be able to tell "no ANRs exist" from "we were not allowed to look".
  A silently truncated dump must be detectable from the artifact.
- **Unavailable observations are tri-state.**
  `IsUserBuild()` compares `ro.build.type` with `user`, defaulting to `user`; userdebug and eng take the same branch at these call sites.
  Report mode and permissions also matter: full reports request getprop on every build type, while telephony-only user reports omit sensitive blocks.
  `null` means unobservable, never false; assign `skipped_user_build` only when the mode/build rule or a skip marker supports that reason.
- **Fallback paths are recorded.**
  The helper's user default does not prove that every field in a debug report is degraded or unavailable.
  Record the actual source and relevant diagnostics.
- **The set of dumpsys sections is resolved at runtime and is open.**
  No artifact may carry a hardcoded section allowlist.
- **`format_version` is read verbatim and never branched on.**
  Its absence means format 0 and is reported as such rather than defaulted.
- **Every fact carries an evidence locator**, and staleness of copied archive entries is computed once, here, so no other artifact re-derives it.

### `apps.json` <a id="draft-req-art-04"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`apps.json` separates installed/user inventory from process attribution over time.
A moving-system snapshot can show one failing app under many PIDs.
Inventory/process tables use stored links, not inferred names.

Payload keys: `capture_ref`, `inventory` (with `packages[]`, `processes[]`, `shared_uids[]` and a precomputed index), `channels[]`, `lifecycle_events[]`, `activity_events[]`, `attribution`, and `evidence_gaps[]`.

- **Process identity is `process_key`.**
  A `pid` is carried as an observation attribute only.
  In the reference archive one crashing process occupies hundreds of PIDs inside a 50 s window, so keying on `pid` manufactures hundreds of processes that are one app failing.
- **The primary key of a package entry is `package_name`**, and of a user installation the pair of package name and user id.
  The per-user app id is stored as a fact and used only to accelerate a join: it is reassigned across reinstall and shared across the platform, so it is never an identity.
- **The link between a process and a package is stored, not recomputed.**
  A process name is not a package name; the platform's own package list for the record is the authoritative mapping, and it is a many-to-many relation.
- **Every evidence channel states its own observed window and whether it covers the archive's log window.**
  The real process-event channel covers roughly 20 s of a 65 s log window, so its death count cannot bound anything over the full window; where coverage is partial, the match block carries the required coverage fields.
- **An empty package list is a permission question before it is a parsing question.**
  The package dump is suppressed unless the caller holds two permissions, and the artifact must be able to say so rather than reporting an empty inventory as a device with no apps.
- **A `tier` label is a label, never a filter.**
  Platform packages with real crash relevance must not be excluded by a third-party filter, and the raw evidence the label is derived from is retained so the label is recomputable.
- **Observations that cannot be attributed are listed with a reason**, never guessed and never silently dropped.

### `anr.json` <a id="draft-req-art-05"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`anr.json` separates ANR **evidence** from supported **incidents**, with times, process identities and thread-dump ages; it cannot establish present responsiveness.

Payload keys: `capture_ref`, `channel_scan[]`, `anr_incidents[]`, `trace_files[]`, `last_anr_trace`, `negative_evidence[]`, and `evidence_gaps[]`.

- **The unit is the incident, not the file and not the process.**
  Trace files and dumped processes are child arrays of an incident, each with its own provenance.
  The reference archive holds 65 retained trace files, one of which contains 31 process dumps, spanning 4 h 19 m on a boot 126 days before capture: that is one incident plus a previous boot's residue, not 65 or 31 ANRs.
- **Disk evidence alone never creates an incident.**
  An incident is emitted only when an in-window dated declaration supports it.
  Retained files are evidence to investigate, and are labelled with their age.
  On the reference archive the correct output is an empty incident list beside 65 stale trace files, with the platform's own assertion that no ANR has occurred since boot recorded as authoritative negative evidence.
- **The declaration channel is the on-disk inventory and the archived traces, not the logcat tag.**
  The `ANR in ` tag matches zero times on the reference archive while 65 real traces exist in the archive; it is a secondary channel and its zero count is reported with a mandatory note rather than as evidence of absence.
- **The newest-trace section header is the freshness linchpin and must be parsed.**
  It carries the file path, the millisecond creation time encoded in the filename, and the file mtime in one line.
  The per-process stamps inside the trace make staleness provable rather than inferred, and a monotonic cross-boot reference taken from them is a required provenance input.
- **Retention is event-driven, not clock-driven.**
  Pruning runs only when a new file is written, so a quiet device can exceed the nominal retention ring.
  The file count is therefore never the incident count, and the artifact must not present it as one.
- **A numeric general timeout is not sourceable** on the current platform.
  The verbatim reason and its origin are carried instead.
- **Each evidence channel reports its own completeness**, because a rate-limited or ring-wrapped channel cannot bound what happened.

### `crashes.json` <a id="draft-req-art-06"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`crashes.json` answers: what crashed, how often, in what pattern, and how old is the evidence.
Its central modelling problem is that one archive records crashes in several mutually inconsistent channels with different completeness guarantees, so that a single file can hold a live crash loop and a months-old native-crash set at once.
The artifact must keep them apart and must not let a lossy channel's count stand in for the truth.

Payload keys: `capture_ref`, `channel_scan[]`, `java_crash_loops[]`, `java_crash_incidents[]`, `native_crash_groups[]`, `native_crash_incidents[]`, `negative_evidence[]`, and `evidence_gaps[]`.

- **Exactly one channel is authoritative for the headline count** and says so.
  Every other channel carries its own completeness and its own coverage.
- **Java and native are separate buckets by design.**
  A live Java crash loop and a previous-boot tombstone set must never be merged, ranked together, or summarised by one number.
- **Denormalised duplicates are counted, not silently absorbed.**
  On the reference archive 274 raw hits resolve to 271 distinct crashes; the 3 extras are logcat copies inlined into the dropbox dump.
  The split is reported in the match block with a note naming the cause, and the headline number is the de-duplicated one.
- **Loops are keyed on a normalised signature** — exception class or signal, plus a top frame with line numbers stripped — never on pid and never on file.
  The artifact must report the number of distinct occurrences, the raw line hits, and the gap statistics that justify the loop classification, plus the classification's thresholds by name.
- **Process identity is `process_key`.**
  A restart loop yields a new PID per attempt for one process identity, so an incident record cannot be keyed on pid.
- **`capture_kind` decides staleness, not elapsed time alone.**
  A crash-buffer line is a live log and a tombstone file is a disk artifact; the previous-boot tombstone set is therefore `stale` while the live loop is not, and `freshness_basis` names the kind.
- **Manager-kill and rescue-party messages are indicative, not events.**
  They are corroboration, and their counts must never be presented as crash counts.
- **Substring noise is reported, not hidden.**
  The raw tombstone-string hit count in the main entry resolves to zero real tombstones, because the real tombstones are archived entries rather than main text.
  Both numbers, and the suppression split, are required.

### `memory.json` <a id="draft-req-art-07"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`memory.json` separates pressure/reactions at capture, memory owners and memory categories.
Collection timing and swap growth are published together; temporal overlap alone does not establish that the collector caused a change.

Payload keys: `capture_ref`, `channels[]`, `collection`, `system`, `swap`, `processes[]`, `categories[]`, `kernel_memory`, `procstats[]`, `pressure_indicators`, `kills[]`, `kill_evidence`, `io`, and `evidence_gaps[]`.

- **Collector work is recorded as measurements.**
  Carry page-scanning section durations and the observed swap timeline.
  An overlap supports possible perturbation, not exclusive causation.
- **Perturbation may be indeterminate.**
  Publish the flag and its evidence basis; do not choose collector versus device from output position alone.
  A background window ending before collection can exclude that collector's effect on the already completed sample.
- **Snapshot deltas are edges between identified nodes.**
  Preserve line order for locators and use independently anchored execution intervals for temporal comparisons.
  Parallel task output is appended when specific futures are awaited, not necessarily in completion order.
- **Persisted aggregates are never merged with each other or with live readings.**
  Each aggregate entry carries its own window and its own completeness flag, and an aggregate whose window long predates the capture is `stale` with the uncapped severity retained.
- **Every derived number carries its derivation.**
  Reference total-PSS is derived from two dump lines; carry its formula with the value.
- **Zero counts are bounded observations.**
  AppProfiler can emit am_meminfo and am_pss before collection.
  Publish searched windows and count coverage; collection order cannot establish why no events were retained.
- **An empty kill list is a result, not an absence**, and is backed by the structured kill evidence that supports it.
- **Units are explicit.**
  `_kb` and `_mb` are decimal; `_kib` and `_mib` are binary.
  Values the platform prints in a 1024-based unit are recorded in the binary unit and are not silently converted.

### `cpu.json` <a id="draft-req-art-08"></a> (source proposal superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04))

`cpu.json` answers how much CPU was used, by whom, over what window, and separately whether anything was blocked.
Do not combine these failure modes: large `system_server` share, high interrupt time and zero per-core steal describe accounted time without identifying interrupt sources or excluding host scheduling delays.

Payload keys: `capture_ref`, `channels[]`, `collection`, `utilization[]`, `per_cpu_core[]`, `processes[]`, `contention`, `load`, `stall_signals`, `graphics`, `power_history`, and `evidence_gaps[]`.

- **Every ratio travels with its window.**
  The utilisation window is chosen by a background sampler and is not the log window, so it must be stated alongside the ratio.
- **A pre-collection sample is not stale merely because it preceded collection.**
  Its own interval determines freshness.
  A genuinely stale background sample still receives the shared freshness cap; capture_kind identifies the measurement kind rather than overriding its clock.
- **Cumulative per-core counters and windowed ratios are separate arrays with different units.**
  The per-core figures are milliseconds since boot and are neither a percentage nor the utilisation window.
- **A sample count is not an event count.**
  Contention samples are collected at an unknown rate over the log window; the artifact must carry the sampling basis, and the number of samples must never be presented as the number of contentions.
- **Partial channel coverage is reported.**
  The real process-event channel covers a small fraction of the archive's log window, which is why a few dozen recorded process deaths coexist with hundreds of actual ones.
  Every count derived from that channel carries the required coverage fields.
- **Resolution floors are not zeros.**
  Columns the platform renders with fixed precision are recorded as below resolution, not as zero.
- **Two total-utilisation figures that disagree are both published**, each with its own window and provenance, and are never reconciled into a single value.
- **Persisted power history is quarantined, not deleted.**
  It is carried with its own window and its own freshness rather than being mixed into the live picture.

### Analyzer tools (DRAFT)

One tool per analyzer crate.
Analyzer scope under [`DRAFT-REQ-GEN-03`](#draft-req-gen-03) remains draft; names propose interfaces without committing scanners.
Analyzer tool result shapes remain deferred and are unrelated to the committed files generated by `UC-03`.

- **`list_anr`** — ANR incidents from the `anr` crate, keyed on incident rather than on a trace file.
  <a id="draft-req-mcp-anr-01"></a> ([`DRAFT-REQ-MCP-ANR-01`](../traceability.md#draft-req-mcp-anr-01))
- **`list_apps`** — Installed packages and running processes from the `apps` crate, separated and keyed on `process_key` rather than `pid`.
  <a id="draft-req-mcp-app-01"></a> ([`DRAFT-REQ-MCP-APP-01`](../traceability.md#draft-req-mcp-app-01))
- **`list_crashes`** — Crash evidence from the `stacktrace` crate, with Java and native evidence kept separate and the de-duplication split reported.
  <a id="draft-req-mcp-crs-01"></a> ([`DRAFT-REQ-MCP-CRS-01`](../traceability.md#draft-req-mcp-crs-01))
- **`list_memory`** — Memory pressure findings from the `memory` crate, carrying the collector's own cost so a swap delta is not read as the device's.
  <a id="draft-req-mcp-mem-01"></a> ([`DRAFT-REQ-MCP-MEM-01`](../traceability.md#draft-req-mcp-mem-01))
- **`list_cpu`** — CPU findings from the `cpu` crate, separating windowed utilization from cumulative counters and from contention.
  <a id="draft-req-mcp-cpu-01"></a> ([`DRAFT-REQ-MCP-CPU-01`](../traceability.md#draft-req-mcp-cpu-01))
- **`DRAFT-REQ-MCP-TOOLS-ANALYSIS`** — Analyzer tool results must never publish a causal claim linking a delta to its collector (e.g., attributing a swap delta to a specific collector section) unless the corresponding proposed artifact (`system.json`, `memory.json`, etc.) would also publish that claim.
  This prohibition applies only to analyzer-backed tools (`DRAFT-REQ-MCP-ANR-01` through `DRAFT-REQ-MCP-CPU-01`); archive tools (`REQ-MCP-ARC-*`) make no causal claims and are unaffected.
  <a id="draft-req-mcp-tools-analysis"></a> ([`DRAFT-REQ-MCP-TOOLS-ANALYSIS`](../traceability.md#draft-req-mcp-tools-analysis))
## General proposal IDs and successors

- **Evidence extraction.**
  The single-report scan scope is superseded by [`REQ-INSIGHT-04`](../swe.1-requirements/02-software-requirements.md#req-insight-04); specialized per-domain analyzer MCP tools remain draft; generic in-memory insight access is committed under [`REQ-MCP-13`](../swe.1-requirements/02-software-requirements.md#req-mcp-13).
  <a id="draft-req-gen-03"></a> ([`DRAFT-REQ-GEN-03`](../traceability.md#draft-req-gen-03))
- **Artifact generation.**
  The seven single-report artifacts are specified by [`REQ-INSIGHT-03`](../swe.1-requirements/02-software-requirements.md#req-insight-03).
  Batch NDJSON, a manifest, and session segmentation remain draft pending schemas and acceptance criteria.
  <a id="draft-req-gen-04"></a> ([`DRAFT-REQ-GEN-04`](../traceability.md#draft-req-gen-04))

## Command-line interface (DRAFT)

`bugreport-insight --insight BUGREPORT` is superseded by [`REQ-INSIGHT-01`](../swe.1-requirements/02-software-requirements.md#req-insight-01).
<a id="draft-req-cli-01"></a> ([`DRAFT-REQ-CLI-01`](../traceability.md#draft-req-cli-01), superseded)

Seven toggles select which analyses run.
All default to true:

| ID | Flag | Includes |
| --- | --- | --- |
| <a id="draft-req-cli-02"></a>[`DRAFT-REQ-CLI-02`](../traceability.md#draft-req-cli-02) | `--sys` | system analysis |
| <a id="draft-req-cli-06"></a>[`DRAFT-REQ-CLI-06`](../traceability.md#draft-req-cli-06) | `--apps` | apps analysis |
| <a id="draft-req-cli-07"></a>[`DRAFT-REQ-CLI-07`](../traceability.md#draft-req-cli-07) | `--anr` | ANR analysis |
| <a id="draft-req-cli-08"></a>[`DRAFT-REQ-CLI-08`](../traceability.md#draft-req-cli-08) | `--crash` | crash analysis |
| <a id="draft-req-cli-09"></a>[`DRAFT-REQ-CLI-09`](../traceability.md#draft-req-cli-09) | `--memory` | memory analysis |
| <a id="draft-req-cli-10"></a>[`DRAFT-REQ-CLI-10`](../traceability.md#draft-req-cli-10) | `--cpu` | CPU analysis |
| <a id="draft-req-cli-11"></a>[`DRAFT-REQ-CLI-11`](../traceability.md#draft-req-cli-11) | `--no-*` | disables one analysis type, e.g. `--no-anr` skips ANR analysis |

`--mcp`, `--help`, and `--verbose` are committed and are specified in [Committed requirements](../swe.1-requirements/02-software-requirements.md#command-line-interface).

## MCP surface (DRAFT)

- **Artifacts are exposed as resources — promoted.**
  The generated artifacts, including `README.md`, are readable as MCP resources, so an agent can cite the triage document rather than re-deriving one.
  What a resource returns is the artifact as written, not a summary of it.
  <a id="draft-req-mcp-07"></a> The source proposal `DRAFT-REQ-MCP-07` is superseded by [`REQ-MCP-07`](../swe.1-requirements/02-software-requirements.md#req-mcp-07).
- **`PatternMode::Regex`.**
  Regex support for `find_entries` and `grep_archive` is deferred.
  Committing its behavior, rather than a dated local dependency-cache observation, is the scope decision.
  <a id="draft-req-mcp-arc-08"></a> ([`DRAFT-REQ-MCP-ARC-08`](../traceability.md#draft-req-mcp-arc-08))
- **Multi-format archive support.**
  TAR and nested ZIP access remain beyond the committed ZIP-only scope (`REQ-MCP-ARC-02`).
  Nested-archive depth and format-specific safety constraints require a future decision.
  <a id="draft-req-mcp-arc-09"></a> ([`DRAFT-REQ-MCP-ARC-09`](../traceability.md#draft-req-mcp-arc-09))

## Allocations

[Core allocation](./allocations/01-core-library.md), [MCP allocation](./allocations/02-mcp-tools.md), [CLI allocation](./allocations/03-cli.md), and [artifact allocation](./allocations/04-artifacts.md) illustrate these proposals.
[Traceability](../traceability.md) owns the proposed element allocations.

## Risk and verification intent

These classifications are retained design intent, not completed checks or evidence of satisfaction.
No draft requirement is part of committed software verification.

| Requirement | Risk | Proposed method |
| --- | --- | --- |
| [`DRAFT-REQ-GEN-04`](#draft-req-gen-04) | medium | test |
| [`DRAFT-REQ-MCP-07`](#draft-req-mcp-07) | superseded by REQ-MCP-07 | recorded in committed requirements |
| [`DRAFT-REQ-MCP-ARC-08`](#draft-req-mcp-arc-08) | medium | test |
| [`DRAFT-REQ-MCP-ARC-09`](#draft-req-mcp-arc-09) | medium | test |
| [`DRAFT-REQ-MCP-ANR-01`](#draft-req-mcp-anr-01) | high | test |
| [`DRAFT-REQ-MCP-APP-01`](#draft-req-mcp-app-01) | medium | test |
| [`DRAFT-REQ-MCP-CRS-01`](#draft-req-mcp-crs-01) | high | test |
| [`DRAFT-REQ-MCP-MEM-01`](#draft-req-mcp-mem-01) | high | test |
| [`DRAFT-REQ-MCP-CPU-01`](#draft-req-mcp-cpu-01) | medium | test |
| [`DRAFT-REQ-MCP-TOOLS-ANALYSIS`](#draft-req-mcp-tools-analysis) | high | test |
| [`DRAFT-REQ-CLI-02`](#draft-req-cli-02) | medium | test |
| [`DRAFT-REQ-CLI-06`](#draft-req-cli-06) | medium | test |
| [`DRAFT-REQ-CLI-07`](#draft-req-cli-07) | medium | test |
| [`DRAFT-REQ-CLI-08`](#draft-req-cli-08) | medium | test |
| [`DRAFT-REQ-CLI-09`](#draft-req-cli-09) | medium | test |
| [`DRAFT-REQ-CLI-10`](#draft-req-cli-10) | medium | test |
| [`DRAFT-REQ-CLI-11`](#draft-req-cli-11) | medium | test |
