# Static insight generation

`UC-03` implements `REQ-INSIGHT-01` through `REQ-INSIGHT-05` using `ARCH-CLI-INSIGHT`, `ARCH-ARCHIVE`, `ARCH-INSIGHT-ANALYZERS`, and `ARCH-INSIGHT-WRITER`.
The [artifact contracts](../proposals/artifact-schemas/index.md) and [generation rules](../proposals/artifact-schemas/12-generation-rules.md) define output fields and decisions.

## Units and interfaces

| Unit | Contract |
| --- | --- |
| `crates/cli/src/main.rs` | Clap selects standalone `--insight BUGREPORT` publication or combined `--mcp --insight PATH` tools; rejects a second positional archive; maps usage/error/success to 2/1/0 and emits diagnostics only on stderr. |
| `insight::generate_insight(&Archive)` | Returns seven `InsightFile` values in README/system/apps/ANR/crashes/memory/CPU order; performs no filesystem writes. |
| `insight::generate_insight_with_options` | Same contract with validated `InsightOptions::crash_thresholds`; positive finite thresholds, repeat count ≥2. |
| `insight::generate_to_with_limits(path, limits)` | Same publication contract with selected coherent archive policy; the exact FORCED profile also raises retained-text byte/line bounds; custom profiles retain default insight retention. |
| `insight::generate_to(path)` | Opens the archive, reserves staging, generates/validates/renders, then atomically publishes the complete set beside the input. |
| `insight::report::Report` | Owns bounded main text, section index, raw-property spelling, resolved clocks, logcat records and archive-wide raw-search ledgers. |
| `insight::system` | Canonical capture/boot context, build/device/user/display/HAL facts, archive metadata inventory and fact evidence. |
| `insight::apps` | Package and per-user state, process records, lifecycle/usage events and guarded process attribution. |
| `insight::incidents` | Current ANR reports, retained traces, Java/native crash records, deduplication and configurable grouping/classification. |
| `insight::resources` | Memory snapshots, swap deltas, procstats, pressure/kills/IO, CPU sample/cumulative windows, contention, graphics and persisted power history. |
| `insight::Artifact::finish` | Assigns stable IDs, applies stale/undated severity caps, derives exact summary counts and records missing-value gaps. |
| `insight::validate::artifacts` | Checks required envelope/domain fields, shared source/generator context, SHA-256, summaries, count partitions, clocks, severity rules and excerpt bounds before publication. |
| `insight::render::readme` | Nine ordered sections, ten top findings, capped evidence rows, explicit JSON pointer for omitted detail and fewer than 400 lines. |
| `insight::output::Stage` | Owns staging and cleanup; fixed filenames, exclusive file creation, file/directory synchronization and exclusive final rename. |

These private units live under [the core module](../../crates/core/src/insight.rs) and its adjacent `insight/` directory.
The MCP crate preserves seven ordinary archive tools, adds three optional insight tools under REQ-MCP-13, and provides [read-only generated-artifact resources](03-mcp-server.md#insight-resource-provider).

## Data and evidence flow

Analysis computes mandatory stored-file SHA-256 even if an earlier core consumer cached identity without a digest.
All artifacts share source and host-run context; only `system.json` owns capture, and the others reference it.
Host time never supplies missing device year/timezone.
An explicit same-day report timestamp may resolve a banner's timezone; unresolved comparisons remain undated.
Current log buffers select the near-capture clock cohort; old embedded rows and LAST LOGCAT remain evidence without stretching that current window.
Boot time is derived from measured uptime anchors with uncertainty retained.

Every promoted finding has a source locator, confidence, freshness, metrics and provenance.
PID alone never establishes process identity; fallback keys stay window-local.
Historical files never become current incidents through filename presence alone.
Raw substring matches are partitioned into validated records and suppressed noise, with scanned/excluded entry IDs and per-section regions.
Absent/failed/OEM-unrecognized fields remain null with gaps; zero findings is not a healthy-device verdict.
A present `SERVICE meminfo` section can contain no recognized RAM summary.
Its `memory_snapshot` finding then uses the section header as provenance while keeping unavailable metrics null and unresolved source clocks undated.
Recognized metrics retain their existing line-specific provenance; no capture clock or RAM value is synthesized to satisfy validation.

## Bounds and errors

Existing core archive expansion/entry/file limits and the 1 MiB retained-line cap still apply.
Insight rejects incomplete retained lines, more than 128 MiB retained text or 2,000,000 lines per parsed text entry by default (512 MiB or 8,000,000 lines with the raised policy), more than 20,000 main section headers, or more than 50,000 raw-count regions.
It reads outer text entries in place, skips detected binary/nested ZIP contents explicitly, and never extracts archive members.
Evidence excerpts preserve UTF-8 and stay within 512 bytes; truncation/NUL incompleteness is explicit.
Collector output failure markers retain section status; an application's log message containing “timed out” is not a collector failure.

`InsightError` distinguishes archive, serialization, invalid analysis/contract and filesystem errors.
A limit, unreadable source, invalid threshold, schema invariant, render bound or write error aborts publication.
JSON is constructed as structured `serde_json::Value`, checked at runtime, and serialized as one object per file.
The validator checks the documented structures/invariants; it is not an independent JSON Schema engine.

## Publication state machine

The destination is `<parent>/.<file-stem>/insight/`.
An existing regular sibling root may contain other files; an existing final destination or a symlink sibling root is refused.
`Stage::new` creates an exclusively owned `.insight-stage-<pid>-<serial>` directory inside that root.
All seven files are written with `create_new`, synchronized, and published through one directory rename.
macOS uses `renamex_np(RENAME_EXCL)`; Linux uses `renameat2(RENAME_NOREPLACE)`; Windows uses directory rename semantics that refuse an existing target.
Other targets return Unsupported rather than offer a racy overwrite fallback.
Even a racing empty destination survives unchanged.

RAII removes owned staging on analysis/write/rename failure and removes a newly created sibling root only if empty.
Successful rename is the completion point; no later fallible operation can turn published success into a reported failure.
This ensures complete visibility and no replacement; it does not promise recovery from machine power loss.
The source ZIP is never opened for writing.

## Verification

[Unit measures](../swe.4-unit-verification/01-verification-measures.md#insight-units), [integration measures](../swe.5-component-and-integration-verification/02-integration-and-verification.md#insight-process), and [requirement measures](../swe.6-software-verification/01-requirement-verification.md#req-insight-01) identify assertions.
The [generation evidence](../../verification/results/insight-generation.md) supports the implemented artifact workflow.
