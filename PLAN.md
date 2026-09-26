# `bugreport-insight` implementation plan

## Goal

Build a Rust command-line tool that analyzes Android bug reports produced by
`adb bugreport` and emits both machine-readable findings and a concise Markdown
report for engineers.

The first supported workflow will be:

```console
$ bugreport-insight BUGREPORT.zip
```

`BUGREPORT.zip` and `.BUGREPORT/` are generic placeholders, not literal
required names. For an input named `BUGREPORT.zip`, the command will read the
archive in place, write structured insight artifacts into
`.BUGREPORT/insight/`, and write the human-readable report to
`.BUGREPORT/insight/README.md`.

```text
BUGREPORT.zip
.BUGREPORT/
└── insight/
    ├── README.md
    ├── dumpstate_0/
    │   └── metadata.json
    ├── manifest.json
    ├── sessions.json
    ├── findings.json
    └── findings.ndjson
```

`--help` and `help` will both expose the same command help.

## Scope for the first release

### Command-line interface

Use `clap` derive support to provide:

```text
bugreport-insight <BUGREPORT_ZIP> [--output <DIR>] [--force]
bugreport-insight help [<COMMAND>]
bugreport-insight dumpstate <BUGREPORT_ZIP> [--output <DIR>] [--force]
bugreport-insight mcp
```

* The default action accepts one local ZIP archive and generates its overview.
* `mcp` is a built-in subcommand that starts the local Model Context Protocol
  (MCP) server over standard input/output.
* Extension names are accepted as subcommands. The core resolves the
  `dumpstate` subcommand to the separately installed
  `bugreport-insight-dumpstate` executable; future extensions follow the
  `bugreport-insight-<extension>` naming convention.
* The default output directory is a hidden sibling of the archive: strip the
  `.zip` extension and prefix the remaining basename with a dot. In this plan,
  `BUGREPORT.zip` maps generically to `.BUGREPORT/`; for example,
  `/tmp/pixel-2026-09-24.zip` maps to `/tmp/.pixel-2026-09-24/`.
* `--output` overrides that directory.
* Refuse to overwrite a non-empty output directory unless `--force` is given.
* Return actionable errors for an absent, unreadable, corrupt, unsupported, or
  unsafe archive; use non-zero exit codes.

### MCP server

`bugreport-insight mcp` starts a local, stdio-transport MCP server. It does
not start an HTTP listener, access the network, or analyze a bugreport until an
MCP client invokes a tool. Reserve standard output exclusively for MCP
JSON-RPC messages; send diagnostics and operational logs to standard error.

Implement the server with a maintained Rust MCP SDK (for example, `rmcp`) and
expose this initial tool surface:

| MCP tool | Inputs | Behavior and result |
| --- | --- | --- |
| `analyze_bugreport` | `input_path`, optional `output_path`, optional `force` | Runs the same default analysis as `bugreport-insight BUGREPORT.zip`; returns the output directory, parsed manifest, severity counts, session/reboot summary, and artifact paths. |
| `analyze_dumpstate` | `input_path`, optional `output_path`, optional `force` | Runs the same extension workflow as `bugreport-insight dumpstate BUGREPORT.zip`; returns the extension artifact paths and parsed validated metadata. |
| `read_insight` | `output_path` | Reads and returns the validated `insight/manifest.json`, `findings.json`, and session index from an already generated output directory without re-running analysis. |

Define JSON Schemas for every tool input and result; reject unknown fields,
relative path traversal through configured output paths, invalid argument
types, and absent artifacts with structured MCP tool errors. Normalize every
path before use and apply the same output-directory, archive, extension, and
JSON-validation protections as the CLI. The server must return extension
lookup or vendor-format failures as tool errors rather than leaving partial
success-shaped results.

The server does not expose a generic shell, arbitrary command execution, or
unbounded raw-bugreport read tool. Its purpose is to let MCP clients request
the existing bounded analyses and consume their structured outputs.

### Read-only archive access

Open the input with `zip::ZipArchive<BufReader<File>>` and enumerate its
central directory without extracting any archive entry. Treat ZIP entry names
as untrusted metadata, never as output paths. The only files the core writes
are the generated `.BUGREPORT/insight/` artifacts.

Enforce configurable-safe defaults for maximum ZIP entry count, compressed and
uncompressed entry sizes, total uncompressed bytes read, and primary-report
bytes processed. Stream selected entries through `Read`; do not buffer a
complete bugreport or materialize any archive payload to disk.

Detect the primary bugreport text from ZIP entry metadata and a bounded
streamed prefix instead of relying on one Android release-specific filename.
Candidate selection will prefer a top-level `bugreport-*.txt` entry and
otherwise choose the largest text-like entry containing recognizable
dumpstate/logcat section headers. If no suitable file is found, emit an
explicit diagnostic while preserving the original ZIP unchanged.

### Vendor-specific dumpstate extension

`dumpstate_board.bin` is vendor-specific and therefore intentionally outside
the core parser. `bugreport-insight dumpstate BUGREPORT.zip` will:

1. Reuse the core's read-only ZIP access and output-directory policy.
2. Find every ZIP entry named `dumpstate_board.bin`; fail with an explicit
   message when none is present.
3. Resolve `bugreport-insight-dumpstate` from `PATH` using an exact executable
   name. Never derive an executable name from archive contents, invoke a
   shell, download an extension, or silently skip a requested extension.
4. Invoke the extension once per matching entry, stream that ZIP entry to the
   extension's standard input, and pass explicit metadata/output arguments:

   ```text
   bugreport-insight-dumpstate \
     --input - \
     --input-name <zip-entry-path>/dumpstate_board.bin \
     --output <.BUGREPORT/insight/dumpstate_<index>>
   ```

5. Surface a missing executable, non-zero extension exit, malformed extension
   output, or unsupported vendor archive as a command failure with the
   extension's diagnostic preserved for the user.

The extension owns the vendor archive format. For its initial contract,
`bugreport-insight-dumpstate` accepts `--input -` as a TAR byte stream and
uses `tar::Archive` to enumerate and read TAR entries without untarring them
to disk. It treats entry paths and links as untrusted metadata, never follows
them, and enforces TAR entry-count, per-entry-size, and cumulative-read-byte
limits. It then writes a schema-versioned `metadata.json` describing the
input, detected vendor format when known, TAR entry inventory, and analysis
results. It must not write outside `--output`.

The core treats extension output as an artifact, not trusted input: it checks
that the output directory exists and contains valid JSON at
`metadata.json`, records the artifact path and extension exit status in
`insight/manifest.json`, and renders a link/status in `insight/README.md`.
The core will not reinterpret vendor-specific payloads or merge unvalidated
extension findings into its canonical `findings.json`.

### Normalized findings contract

Model all scanners behind a common `Scanner` trait. Every emitted finding will
be serializable with `serde` and include:

```json
{
  "schema_version": 1,
  "id": "anr",
  "severity": "high",
  "title": "Application Not Responding",
  "summary": "…",
  "evidence": [
    {
      "source": "bugreport-…txt",
      "line_start": 123,
      "line_end": 128,
      "excerpt": "…"
    }
  ],
  "metadata": {
    "package": "com.example.app",
    "pid": 12345,
    "timestamp": "…"
  }
}
```

The exact fields within `metadata` will be scanner-specific and optional.
Evidence excerpts will be bounded and will retain source line references, so
automation can locate the original text without parsing Markdown.

`insight/manifest.json` will describe the schema version, tool version,
timestamp, input archive name, selected primary text file, and generated
artifact names. It will also summarize the logcat session count and reboot
count. `insight/sessions.json` will be the ordered machine-readable session
index. `findings.json` will contain the complete schema-versioned array;
`findings.ndjson` will contain one finding per line for streaming tools.

### Reboot-aware logcat sessions

Logcat captured in a bug report can span multiple device boots. Identify each
contiguous boot session, number it chronologically starting at zero, and report
the number of sessions and inferred reboots in both `manifest.json` and the
human-readable summary. A report with `N` sessions has `N - 1` observed reboot
boundaries; an incomplete log capture may omit earlier boots, so this is a
lower bound rather than a claim of the device's lifetime reboot count.

Segment sessions from the ordered logcat records using evidence such as a log
timestamp moving backwards, kernel/system-server boot or shutdown markers,
`BOOT_COMPLETED` activity, and log-buffer reset markers. Preserve the
supporting lines and a confidence value for every boundary. Do not create
separate sessions merely because a logcat buffer (`main`, `system`, `events`,
or `crash`) begins; merge records with the same boot window across buffers.

For every detected session, create a directory under `insight/`:

```text
.BUGREPORT/insight/
├── logcat_0/
│   └── metadata.json
├── logcat_1/
│   └── metadata.json
└── ...
```

Each `logcat_<index>/metadata.json` will be a schema-versioned, machine-readable
summary with the session number, start and end timestamps, duration when
timestamps permit, boundary evidence, and the apps observed in that session.
`sessions.json` will summarize these directories for clients that need the
ordered reboot/session timeline without reading every metadata file. The
session range shown in `README.md` will be derived from this same data.

```json
{
  "schema_version": 1,
  "session_index": 0,
  "time_range": {
    "start": "2026-09-23T08:14:02.123",
    "end": "2026-09-23T11:36:47.442",
    "duration_ms": 12165319
  },
  "boundary": {
    "start_confidence": "high",
    "end_reason": "timestamp_regression",
    "evidence": [
      { "source": "bugreport-…txt", "line_start": 120, "line_end": 120 }
    ]
  },
  "apps": [
    {
      "package": "com.example.app",
      "pids": [1234, 1288],
      "data_dirs": ["/data/user/0/com.example.app"],
      "apk_dirs": ["/data/app/…/base.apk"],
      "loaded_libraries": ["/data/app/…/lib/arm64/libexample.so"],
      "crashes": [
        {
          "timestamp": "2026-09-23T09:00:00.000",
          "pid": 1288,
          "kind": "java_fatal_exception",
          "evidence": [
            { "source": "bugreport-…txt", "line_start": 450, "line_end": 470 }
          ]
        }
      ]
    }
  ]
}
```

All PID fields are arrays where an app can have more than one process or PID
over the session. Collect app associations from process-start/death logs,
crash records, process listings, and relevant dumpstate sections; retain
unresolved PIDs as session-level records with evidence instead of guessing an
application. Treat data directories, APK paths, and loaded libraries as
optional observed values, deduplicate them in encounter order, and associate
them with an app only when supported by explicit bugreport evidence.

### Initial scanners

Base the initial scans on the Android documentation's recommended diagnostic
signals, while treating them as evidence—not definitive root-cause statements:

| Scanner | Detection | Reported evidence |
| --- | --- | --- |
| ANR | `am_anr` event-log entries and `ANR in` logcat entries | package, PID when available, reason, load/CPU block, nearby VM-trace reference |
| Process churn / low memory | `am_low_memory`, `am_proc_start`, and `am_proc_died` | counts, rolling high-churn windows, involved packages |
| Watchdog / runtime restart | `WATCHDOG KILLING SYSTEM PROCESS`, system-server death/restart markers | matching log lines and timestamps |
| Java/native crash | fatal-exception and tombstone/crash signatures | process/package, signal/exception where parseable, stack excerpt |
| High CPU / I/O pressure | CPU snapshots and `kswapd`, `kworker`, `mmcqd`, or `dex2oat` indicators | utilization/process rows and qualifying log excerpt |
| Monitor contention | ART long-monitor-contention and `dvm_lock_sample` records | owning method/thread and duration |
| Broadcast backlog | active ordered broadcast queue size / slow dispatch markers | queue identifiers and dispatch timing |
| Focused activity timeline | `am_focused_activity` | ordered activity events for correlation, informational severity |
| Logcat reboot sessions | timestamp regressions and boot/shutdown/reset markers | chronological sessions, time ranges, boundary evidence, and per-session app metadata |

Scanners will tolerate missing sections and Android-version format variations.
They will produce zero findings rather than failing when a particular section
is absent.

### Human-readable `insight/README.md`

The Android documentation explains the relevant raw bugreport signals (ANRs,
process churn, memory snapshots, broadcasts, monitor contention, and
compilation), but does not prescribe an insight-report format. Generate this
tool-owned Markdown report:

```markdown
# Bugreport insight

## Summary

- Source: `bugreport-…txt`
- Findings: 3 high, 2 medium, 4 informational
- Logcat sessions: 2; observed reboot boundaries: 1
- Analysis caveat: Findings are evidence from a captured snapshot, not proof
  of causality.

## Findings

### High — Application Not Responding

<one-sentence interpretation>

**Evidence**

- `bugreport-…txt:123-128` — `<bounded excerpt>`

**Suggested investigation**

- Inspect the matching main-thread VM trace and correlate it with CPU and
  process-lifecycle events.

## Activity timeline

<chronological focused-activity events, when available>

## Logcat sessions

| Session | Time range | Observed reboot boundary |
| --- | --- | --- |
| `logcat_0` | 2026-09-23 08:14:02–11:36:47 | Ends before timestamp regression |
| `logcat_1` | 2026-09-23 11:37:20–14:02:11 | Starts after timestamp regression |

Each session links to its `logcat_<index>/metadata.json`, which lists observed
apps, all associated PIDs, data/APK paths, loaded libraries, and crashes.

## Generated artifacts

- `insight/README.md`
- `insight/manifest.json`
- `insight/sessions.json`
- `insight/findings.json`
- `insight/findings.ndjson`
- `insight/logcat_<index>/metadata.json`
```

The renderer will escape Markdown supplied by the bugreport, cap excerpts and
timeline lengths, and never claim a finding is a root cause. Suggestions will
link each class of signal back to the appropriate raw data (for example,
matching ANR PID and timestamp with VM traces).

## Project structure

Use a Cargo workspace that separates the scanning/rendering library from the
CLI entry point, so the library has no `clap` dependency and can be reused
(for example by an MCP server) without pulling in argument parsing:

```text
Cargo.toml           # [workspace] manifest, no [package]; [workspace.dependencies] shared
                      # across every crate below
crates/
├── core/            # bugreport-insight-core: library crate, no clap dependency
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs        # pub mod declarations and top-level re-exports
│       ├── analyze.rs    # open_report() (shared by the CLI and every subcommand),
│       │                 # unclaimed_findings() (scanners with no owning subcommand),
│       │                 # and render_insight() (output lifecycle)
│       ├── archive.rs    # read-only ZIP validation, discovery, and streaming
│       ├── source.rs     # primary report discovery and line-indexed reading
│       ├── model.rs      # finding, app-info, evidence, severity, and manifest schemas
│       ├── sessions.rs   # reboot-aware logcat segmentation and app metadata
│       ├── text.rs       # shared string helpers (bounding, escaping)
│       ├── error.rs      # typed domain errors
│       ├── scanners/
│       │   ├── mod.rs      # pattern-matching helpers (find_all, lookahead,
│       │   │                # unique_sample) shared by scanners in this crate
│       │   │                # and in the subcommand crates below
│       │   ├── crash.rs       # scan_watchdog (no owning subcommand)
│       │   ├── performance.rs # scan_broadcast_latency (no owning subcommand)
│       │   └── activity.rs    # scan_activity_timeline (no owning subcommand)
│       └── render.rs     # JSON/NDJSON and insight/README.md rendering
├── cli/             # bugreport-insight: thin binary crate; default full report,
│   │                # plus git/cargo-style subcommand dispatch (see below)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs   # CLI entry point, exit-code handling, subcommand dispatch
│       ├── cli.rs    # clap command definitions
│       ├── plugin.rs # `bugreport-insight-<verb>` binary discovery and exec
│       └── report.rs # assembles the default full report from every subcommand
│                      # crate's scan_*() library function plus core's
│                      # unclaimed_findings()
├── subcommand-support/  # bugreport-insight-subcommand-support: shared clap
│   │                    # args + JSON-emit helper for subcommand binaries
│   └── src/lib.rs
├── subcommand-anr/         # bugreport-insight-anr: lib.rs owns scan_anrs();
│   │                       # main.rs is the `bugreport-insight anr` binary
│   ├── Cargo.toml
│   └── src/{lib.rs,main.rs}
├── subcommand-apps/        # bugreport-insight-apps: lib.rs owns scan_apps()
│   │                       # (installed/running app inventory: PID, package,
│   │                       # data dir, install path)
│   ├── Cargo.toml
│   └── src/{lib.rs,main.rs}
├── subcommand-stacktrace/  # bugreport-insight-stacktrace: lib.rs owns
│   │                       # scan_java_crashes() and scan_native_crashes()
│   ├── Cargo.toml
│   └── src/{lib.rs,main.rs}
├── subcommand-memory/      # bugreport-insight-memory: lib.rs owns
│   │                       # scan_memory_pressure()
│   ├── Cargo.toml
│   └── src/{lib.rs,main.rs}
└── subcommand-cpu/         # bugreport-insight-cpu: lib.rs owns scan_cpu_pressure()
    │                       # and scan_monitor_contention()
    ├── Cargo.toml
    └── src/{lib.rs,main.rs}
tests/
├── fixtures/      # small synthetic, non-sensitive reports and ZIPs
├── cli.rs
├── archive.rs
└── analyze.rs
README.md
```

### Scanner ownership: core vs. subcommand crates

Each built-in subcommand crate (`subcommand-anr`, `-apps`, `-stacktrace`,
`-memory`, `-cpu`) is a lib+bin crate: `lib.rs` holds that verb's scanner
code (moved out of `crates/core/src/scanners/`) as a `pub fn scan_*()`, and
`main.rs` is a thin binary that calls it. This makes each scanner
independently reusable as a library — by the `bugreport-insight` CLI's
default full-report assembly (`crates/cli/src/report.rs`), or by a
third-party subcommand that wants that one specific scanner — without
pulling in the whole scanner suite. `bugreport-insight-core` keeps: the
shared infra every subcommand needs (`open_report`, the `Finding`/`Evidence`/
`AppInfo` model types, rendering, sessions), the small pattern-matching
helpers scanners share (`find_all`, `lookahead`, `unique_sample`,
`MAX_EVIDENCE_PER_FINDING`, all `pub` in `scanners::mod`), and the few
scanners with no owning built-in subcommand (`watchdog`, `broadcast_latency`,
`activity_timeline` — exposed together via `core::unclaimed_findings()`).
Subcommand crates depend on `bugreport-insight-core`; `bugreport-insight-core`
never depends back on a subcommand crate, so there is no dependency cycle —
the `bugreport-insight` CLI (`crates/cli`) is the one place that depends on
all of them to compose the default full report.

### Subcommand dispatch (git/cargo-style plugins)

`bugreport-insight <verb> [args...]` execs a sibling `bugreport-insight-<verb>`
binary instead of being handled inline by the main CLI (see `crates/cli/src/plugin.rs`).
Built-in verbs (`anr`, `apps`, `stacktrace`, `memory`, `cpu`) are separate binary
crates in this workspace, each a thin wrapper around its own `scan_*()`
library function (see above). Any other verb (for example a third-party
`dumpstate` subcommand) is looked up the same way: it only runs if a matching
`bugreport-insight-<verb>` executable is discoverable next to the current
executable or on `PATH`, otherwise the CLI falls back to its default
full-report behavior (treating the verb as a `BUGREPORT_ZIP` path).
`bugreport-insight help <verb>` execs `bugreport-insight-<verb> --help` so
each subcommand's own help text (rather than the main program's) is shown.
Because `bugreport-insight-core` has no CLI dependency, a third-party
subcommand (like `dumpstate`) can link it directly as a library for shared
infra, and optionally depend on one or more specific
`bugreport-insight-<verb>` crates as libraries too — it does not need to
shell out to the `bugreport-insight` binary or reimplement scanning.

Future `extensions.rs`/`mcp.rs` modules (extension discovery/execution and the
stdio MCP server) belong in `crates/core` or a dedicated `crates/mcp` crate,
depending on whether they need the CLI's dependencies.

## Rust package selection

Use focused, maintained crates with narrow responsibilities. Pin compatible
versions in `Cargo.lock`; enable only the compression features required by real
Android bugreport fixtures.

| Package | Purpose | Selection rationale |
| --- | --- | --- |
| `clap` | CLI arguments, help, and subcommands | Derive-based definitions keep the default action, `dumpstate`, `mcp`, and generated help consistent. |
| `thiserror` | Typed domain errors | Produces actionable CLI and MCP diagnostics without broad error handling. |
| `zip` | Read-only ZIP enumeration and entry streaming | Reads archive entries through `Read` without extracting them. |
| `tar` | Read-only `dumpstate_board.bin` TAR enumeration | Accepts the streamed ZIP entry and avoids materializing TAR files. |
| `serde` + `serde_json` | Artifact models and JSON encoding/decoding | Provides typed, stable JSON contracts shared by CLI, extensions, and MCP. |
| `schemars` | JSON Schema generation | Generates published schemas from the versioned `serde` models once the artifact contract stabilizes. |
| `rmcp` | Stdio MCP server | Implements the bounded MCP tool surface and JSON-RPC lifecycle. |
| `time` | Normalized timestamps and durations | Avoids ad hoc timestamp handling across logcat sessions and artifact metadata. |
| `tempfile` | Tests only | Creates isolated test archives and output directories; production processing remains read-only and streaming. |

Prefer standard-library buffered/streaming I/O and explicit parsers over
loading complete multi-gigabyte bugreports or vendor TAR payloads into memory.

## File format decision

Use **JSON** as the canonical machine-readable artifact format and **NDJSON**
as the streaming representation for large finding sets. This matches the
`serde` model layer, supports JSON Schema validation, works directly with MCP,
and is readily consumed by tools such as `jq`.

| Artifact | Format | Contract |
| --- | --- | --- |
| `insight/manifest.json` | JSON | Versioned artifact index, tool/input metadata, and session/reboot totals. |
| `insight/sessions.json` | JSON | Ordered reboot-aware session index and links to session directories. |
| `insight/logcat_<index>/metadata.json` | JSON | One session's time range, apps, PID arrays, paths, libraries, crashes, and evidence. |
| `insight/dumpstate_<index>/metadata.json` | JSON | One vendor TAR analysis result and inventory. |
| `insight/findings.json` | JSON | Complete deterministic array of normalized findings. |
| `insight/findings.ndjson` | NDJSON | One normalized finding per line for incremental readers and large reports. |
| `insight/README.md` | Markdown | Human-readable summary with links to the canonical structured artifacts. |

Every JSON document will include `schema_version` and use stable, explicit
field names. Publish corresponding JSON Schemas when the model stabilizes.
YAML is not the primary format because implicit typing, aliases, and
parser-specific behavior weaken a generated long-lived contract; TOML is less
suited to nested, repeated evidence records.

## Development fixture

Use the provided Android Automotive bugreport as the canonical manual
development fixture:

```text
bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip
bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17/
```

The ZIP is the input to exercise. The matching extracted directory is a
read-only reference for comparing ZIP entry discovery and primary-report
selection; it contains
`bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.txt`.
Do not use that reference directory as command output. The default analysis
command instead creates the separate hidden sibling directory:

```console
$ cargo run -- \
  bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip

# Expected generated root:
# .bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17/
```

Use this fixture for manual end-to-end practice while developing read-only ZIP
access, primary text selection, logcat session segmentation, scanners, and the
rendered `insight/README.md`. Its current extracted contents do not include
`dumpstate_board.bin`, so it is not a dumpstate-extension fixture. Cover that
extension with small synthetic TAR fixtures and a vendor-supplied sample when
one is available.

Bugreports can include sensitive device, account, application, and filesystem
information. Keep this fixture local to the development workspace; do not
upload it, include its contents in documentation, or create derived artifacts
from it in source control. Add the generated hidden output directory to
`.gitignore` when initializing the repository.

## Implementation sequence

1. Initialize the binary crate, command grammar, consistent error types, and
   README with the privacy warning that bugreports commonly contain sensitive
   device and application data.
2. Implement output-path calculation, bounded read-only ZIP enumeration and
   entry streaming, overwrite policy, primary-report discovery, and tests for
   malformed archives, unsupported compression, and archive resource limits.
3. Implement extension-subcommand discovery and the `dumpstate` orchestration
   contract: exact `PATH` lookup, argument-safe process execution, per-input
   output directories, and extension-output validation. Implement the
   `bugreport-insight-dumpstate` extension's bounded, read-only TAR streaming
   and `metadata.json` contract in its own Rust binary crate.
4. Implement the stdio MCP server with schema-validated, bounded
   `analyze_bugreport`, `analyze_dumpstate`, and `read_insight` tools. Route
   each through the same core analysis functions as the CLI; test the
   initialize, tool-list, success, and structured-error protocol paths.
5. Define the versioned finding, manifest, and session-index models; write
   JSON and NDJSON artifacts and generate their JSON Schemas before adding
   individual analyses.
6. Implement reusable line indexing, section recognition, timestamp handling,
   bounded excerpts, reboot-aware logcat segmentation, per-session app
   metadata extraction, and the scanner trait.
7. Add the initial scanners in the table above, using version-tolerant parsing
   and structured evidence for every result.
8. Render deterministic `insight/README.md` from the same finding model; include
   severity ordering, evidence locations, reboot/session ranges, caveats, and
   raw-data investigation guidance, plus vendor-extension artifact status.
9. Add end-to-end fixture tests that assert archive layout, output schemas,
   Markdown sections, and representative findings; run formatting, Clippy, and
   the full test suite.

## Test and acceptance criteria

* `bugreport-insight --help` and `bugreport-insight help` succeed and show the
  default analysis syntax and the `dumpstate` extension command.
* Analyzing a valid fixture archive produces the specified output layout,
  valid JSON/NDJSON, and a readable `insight/README.md`.
* All findings reference a source file and valid, non-zero source line range.
* ZIP entry paths are never materialized; an archive containing traversal or
  absolute-path entry names cannot create files outside the output directory.
* ZIP entry-count, size, and total streamed-byte limits prevent resource
  exhaustion and return an explicit error.
* Existing non-empty outputs are protected unless `--force` is supplied.
* Missing optional Android sections do not fail analysis.
* `bugreport-insight mcp` completes MCP initialization and lists only the
  documented bounded tools; all protocol messages are valid JSON-RPC on
  standard output and diagnostics remain on standard error.
* The MCP analysis tools produce the same artifact layout and validated
  summaries as their CLI equivalents, and report invalid paths, missing
  artifacts, archive failures, and extension failures as structured errors.
* `bugreport-insight dumpstate BUGREPORT.zip` finds all
  `dumpstate_board.bin` inputs and invokes `bugreport-insight-dumpstate` with
  explicit paths; a missing executable, a non-zero exit, or invalid
  `metadata.json` fails visibly rather than producing a success-shaped result.
* Extension TAR fixtures prove paths and links are never followed or
  materialized, and TAR entry-count, individual-size, and cumulative-byte
  limits are enforced while streaming.
* A multi-boot fixture produces one chronological `logcat_<index>/metadata.json`
  directory per session, reports `sessions - 1` observed reboot boundaries,
  includes each session's time range and boundary evidence, and retains all
  observed app PIDs in PID arrays.
* Session metadata includes only evidence-supported data directories, APK
  paths, loaded libraries, and crashes; unknown PID ownership is represented
  explicitly rather than guessed.
* Fixtures cover ANR, low-memory/process churn, crash, CPU pressure, monitor
  contention, broadcast backlog, multiple reboot-aware logcat sessions, and
  no-finding reports.
* A documented manual smoke test runs the default analysis on
  `bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip`, creates
  its hidden sibling output directory without modifying the existing extracted
  reference directory, and produces the expected insight artifacts.

## Deliberately deferred

* Reading an uncompressed bugreport text file directly.
* Interactive terminal UI, HTML reports, databases, or network uploads.
* Automatic root-cause determination, remediation, or issue filing.
* Full parsing of every Android/OEM-specific dumpstate section.
* Built-in parsing of vendor-specific `dumpstate_board.bin`; this remains the
  responsibility of the separately versioned `bugreport-insight-dumpstate`
  extension.
* Cross-report trend analysis and incremental reprocessing.

## Source reference

The scanner selection and evidence guidance derive from Android's
[Read bug reports](https://source.android.com/docs/core/tests/debug/read-bug-reports)
documentation, especially its sections on logcat/event logs, ANRs and
deadlocks, activities, memory, broadcasts, monitor contention, and background
compilation.
