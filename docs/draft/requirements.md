# Draft requirements

> **Everything in this file is draft.** Every ID below carries the `DRAFT-` prefix, which is not
> decoration: an unprefixed ID in [`../requirements.md`](../requirements.md) is a requirement this
> project has committed to, and a `DRAFT-` ID may change or be withdrawn without that being a
> breaking change. **Do not implement against a `DRAFT-` ID, and do not treat one as traceability
> evidence.** See [`../traceability-matrix.md`](../traceability-matrix.md) for how draft status is
> recorded in the join.

These 26 requirements describe the analyzer half of the tool: the artifact set, the CLI command and
toggles that drive it, and the analyzer tools and crates behind them. None of it exists. The committed
half — reading the archive, and serving it over MCP — is in [`../requirements.md`](../requirements.md).

## The committed / draft boundary

| | Count | Where |
| --- | --- | --- |
| Committed | 23 | [`../requirements.md`](../requirements.md) |
| Draft | 26 | this file |
| **Total** | **49** | [`../traceability-matrix.md`](../traceability-matrix.md) |

The boundary is not "big" versus "small". It is: does the requirement constrain anything the tool
does today? Reading a ZIP and answering questions about it does; writing seven JSON artifacts does
not.

## Inventory

| ID | States | Implemented by |
| --- | --- | --- |
| `DRAFT-REQ-GEN-03` | Evidence extraction — scan logcat/dumpstate for signals | `DRAFT-ARCH-ANR`, `-APPS`, `-STACKTRACE`, `-MEMORY`, `-CPU` |
| `DRAFT-REQ-GEN-04` | Artifact generation — JSON/NDJSON findings, a README, a manifest, session segmentation | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` |
| `DRAFT-REQ-MCP-07` | Artifacts readable as MCP resources | `DRAFT-ARCH-MCP-RES` |
| `DRAFT-REQ-MCP-TOOLS-ANALYSIS` | Analyzer tool results publish no causal claim the artifacts would not | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-ARC-08` | `PatternMode::Regex` | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-MCP-ARC-09` | Multi-format archives — TAR, nested ZIP | — |
| `DRAFT-REQ-MCP-{ANR,APP,CRS,MEM,CPU}-01` | The five analyzer tools | `DRAFT-ARCH-MCP-TOOLS-ANALYSIS` |
| `DRAFT-REQ-CLI-01`, `-02`, `-06` … `-11` | The `BUGREPORT` analysis command and its seven toggles | `DRAFT-ARCH-CLI-CMD` |
| `DRAFT-REQ-ART-01` … `-08` | The seven artifacts | `DRAFT-ARCH-CORE-ART`, `DRAFT-ARCH-SUPPORT` |

Field-level detail for the artifact set is in [`artifact-schemas/`](./artifact-schemas/index.md),
which is written and versioned with `DRAFT-REQ-ART-01` … `-08`.

## Analysis Artifacts (DRAFT) <a id="draft-req-art-02"></a> ([`DRAFT-REQ-ART-02`](../diagrams/index.md#draft-req-art-02))

This section and every requirement in it are **draft**, because every one of them describes what an
analyzer produces. The `DRAFT-` prefix is not decoration: an
unprefixed ID is a requirement this project has committed to, and a `DRAFT-` ID may change or be
withdrawn without that being a breaking change. Do not implement against a `DRAFT-` ID, and do not
treat it as traceability evidence. [`artifact-schemas/`](./artifact-schemas/index.md) is the companion specification and
moves with this section. [`DRAFT-REQ-GEN-04`](#draft-req-gen-04) in [General](#general-draft) states the
same output-set commitment and carries the same prefix.

Each generated artifact answers one question, and states the evidence and the limits of that
answer. This section states what every artifact must let a consumer conclude, and the constraints
that make each conclusion defensible. Field-level detail — every key, every enum, every type —
belongs to the companion specification, [`artifact-schemas/`](./artifact-schemas/index.md), which
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

The rules that constrain `DRAFT-REQ-ART-03` through `DRAFT-REQ-ART-08` are specified once, as field
schemas, in [`artifact-schemas/conventions-envelope.md`](./artifact-schemas/conventions-envelope.md)
and [`artifact-schemas/conventions-evidence.md`](./artifact-schemas/conventions-evidence.md). They are
not restated here: this file states what each artifact must let a consumer conclude, and the schema
files state how its shape makes that possible.

Four of those rules constrain the requirement text below and are worth repeating in one line each,
because violating them changes what a conclusion means:

- **Evidence is a locator, never a copy.** Every finding carries an archive path, a kind, a line or
  byte range, and an excerpt capped at **512 bytes** with truncation flagged.
- **Freshness is a four-state enum assigned mechanically** — `now`, `in_window`, `stale`, `undated`.
  There is no "unknown defaults to recent" member; the absence of that member is the requirement. A
  stale or undated finding is labelled, never dropped and never silently demoted.
- **No count is a bare number.** Every count is a match block separating confirmed evidence from
  suppressed substring noise, with the channel's window coverage.
- **Process identity is `process_key`, never `pid`.** PIDs are recycled within the log window, so a
  `pid` is an observation attribute and never a key.#

### `README.md` <a id="draft-req-art-01"></a> ([`DRAFT-REQ-ART-01`](../diagrams/index.md#draft-req-art-01))

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

### `system.json` <a id="draft-req-art-03"></a> ([`DRAFT-REQ-ART-03`](../diagrams/index.md#draft-req-art-03))

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

### `apps.json` <a id="draft-req-art-04"></a> ([`DRAFT-REQ-ART-04`](../diagrams/index.md#draft-req-art-04))

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

### `anr.json` <a id="draft-req-art-05"></a> ([`DRAFT-REQ-ART-05`](../diagrams/index.md#draft-req-art-05))

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

### `crashes.json` <a id="draft-req-art-06"></a> ([`DRAFT-REQ-ART-06`](../diagrams/index.md#draft-req-art-06))

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

### `memory.json` <a id="draft-req-art-07"></a> ([`DRAFT-REQ-ART-07`](../diagrams/index.md#draft-req-art-07))

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

### `cpu.json` <a id="draft-req-art-08"></a> ([`DRAFT-REQ-ART-08`](../diagrams/index.md#draft-req-art-08))

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

### Analyzer tools (DRAFT)

One tool per analyzer crate. Each is draft because the analyzer behind it is, under
[`DRAFT-REQ-GEN-03`](#draft-req-gen-03); naming them now fixes the interface without committing the
scanner. Their result shapes follow [`DRAFT-REQ-ART-*`](#draft-req-art-02), so they are draft twice
over.

- **`list_anr`** — ANR incidents from the `anr` crate, keyed on incident rather than on a trace file.
  <a id="draft-req-mcp-anr-01"></a> ([`DRAFT-REQ-MCP-ANR-01`](../diagrams/index.md#draft-req-mcp-anr-01))
- **`list_apps`** — Installed packages and running processes from the `apps` crate, separated and keyed
  on `process_key` rather than `pid`. <a id="draft-req-mcp-app-01"></a> ([`DRAFT-REQ-MCP-APP-01`](../diagrams/index.md#draft-req-mcp-app-01))
- **`list_crashes`** — Crash evidence from the `stacktrace` crate, with Java and native evidence kept
  separate and the de-duplication split reported. <a id="draft-req-mcp-crs-01"></a> ([`DRAFT-REQ-MCP-CRS-01`](../diagrams/index.md#draft-req-mcp-crs-01))
- **`list_memory`** — Memory pressure findings from the `memory` crate, carrying the collector's own
  cost so a swap delta is not read as the device's. <a id="draft-req-mcp-mem-01"></a> ([`DRAFT-REQ-MCP-MEM-01`](../diagrams/index.md#draft-req-mcp-mem-01))
- **`list_cpu`** — CPU findings from the `cpu` crate, separating windowed utilization from cumulative
  counters and from contention. <a id="draft-req-mcp-cpu-01"></a> ([`DRAFT-REQ-MCP-CPU-01`](../diagrams/index.md#draft-req-mcp-cpu-01))
- **`DRAFT-REQ-MCP-TOOLS-ANALYSIS`** — Analyzer tool results must never publish a causal claim linking a delta to its collector (e.g., attributing a swap delta to a specific collector section) unless the corresponding committed artifact (`system.json`, `memory.json`, etc.) would also publish that claim. This prohibition applies only to analyzer-backed tools (`DRAFT-REQ-MCP-ANR-01` through `DRAFT-REQ-MCP-CPU-01`); archive tools (`REQ-MCP-ARC-*`) make no causal claims and are unaffected. <a id="draft-req-mcp-tools-analysis"></a> ([`DRAFT-REQ-MCP-TOOLS-ANALYSIS`](../diagrams/index.md#draft-req-mcp-tools-analysis))
## General (DRAFT)

- **Evidence extraction.** Scan logcat/dumpstate for ANR, crashes, memory pressure, CPU contention,
  and other signals. <a id="draft-req-gen-03"></a> ([`DRAFT-REQ-GEN-03`](../diagrams/index.md#draft-req-gen-03))
- **Artifact generation.** Produce JSON/NDJSON findings, a Markdown `README.md`, a manifest, and
  session segmentation, into a sibling `.BUGREPORT/insight/` directory. <a id="draft-req-gen-04"></a> ([`DRAFT-REQ-GEN-04`](../diagrams/index.md#draft-req-gen-04))

## Command-line interface (DRAFT)

`bugreport-insight BUGREPORT` generates the static analysis artifacts above. Default output directory
is `.<BUGREPORT>/insight/`. <a id="draft-req-cli-01"></a> ([`DRAFT-REQ-CLI-01`](../diagrams/index.md#draft-req-cli-01))

Seven toggles select which analyses run. All default to true:

| ID | Flag | Includes |
| --- | --- | --- |
| <a id="draft-req-cli-02"></a>[`DRAFT-REQ-CLI-02`](../diagrams/index.md#draft-req-cli-02) | `--sys` | system analysis |
| <a id="draft-req-cli-06"></a>[`DRAFT-REQ-CLI-06`](../diagrams/index.md#draft-req-cli-06) | `--apps` | apps analysis |
| <a id="draft-req-cli-07"></a>[`DRAFT-REQ-CLI-07`](../diagrams/index.md#draft-req-cli-07) | `--anr` | ANR analysis |
| <a id="draft-req-cli-08"></a>[`DRAFT-REQ-CLI-08`](../diagrams/index.md#draft-req-cli-08) | `--crash` | crash analysis |
| <a id="draft-req-cli-09"></a>[`DRAFT-REQ-CLI-09`](../diagrams/index.md#draft-req-cli-09) | `--memory` | memory analysis |
| <a id="draft-req-cli-10"></a>[`DRAFT-REQ-CLI-10`](../diagrams/index.md#draft-req-cli-10) | `--cpu` | CPU analysis |
| <a id="draft-req-cli-11"></a>[`DRAFT-REQ-CLI-11`](../diagrams/index.md#draft-req-cli-11) | `--no-*` | disables one analysis type, e.g. `--no-anr` skips ANR analysis |

`--mcp`, `--help`, and `--verbose` are committed and are specified in
[`../requirements.md`](../requirements.md#command-line-interface).

## MCP surface (DRAFT)

- **Artifacts are exposed as resources.** The generated artifacts, including `README.md`, are
  readable as MCP resources, so an agent can cite the triage document rather than re-deriving one.
  What a resource returns is the artifact as written, not a summary of it. <a id="draft-req-mcp-07"></a> ([`DRAFT-REQ-MCP-07`](../diagrams/index.md#draft-req-mcp-07))
- **`PatternMode::Regex`.** `find_entries` and `grep_archive` currently accept a literal or glob
  pattern only. Regex support is deferred here. It was originally deferred because the `regex` crate
  was absent from the offline cache; that turned out to be false — `regex 1.13.1` **is** cached and
  `grep-regex` is a declared dependency. Availability was never the blocker; committing the
  requirement is. <a id="draft-req-mcp-arc-08"></a> ([`DRAFT-REQ-MCP-ARC-08`](../diagrams/index.md#draft-req-mcp-arc-08))
- **Multi-format archive support.** The server may eventually open TAR archives and nested ZIP
  entries in addition to the committed ZIP-only scope (`REQ-MCP-ARC-02`). `crates/core` deliberately
  implements no TAR. Prerequisites are the `zip` crate's absent `time` feature and a decision on
  nested-archive depth. <a id="draft-req-mcp-arc-09"></a> ([`DRAFT-REQ-MCP-ARC-09`](../diagrams/index.md#draft-req-mcp-arc-09))

## Diagrams

The requirement-to-element diagrams covering this file are [`req-draft.md`](../diagrams/req-draft-artifacts.md),
and [`req-cli.md`](../diagrams/req-cli.md) for the committed CLI flags. The full ID index is
[`../diagrams/index.md`](../diagrams/index.md).
