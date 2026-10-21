# Generation rules and resolved schema decisions

This normative supplement defines generation decisions for `UC-03`.
It refines conflicting examples and missing rule tables; all other envelope, domain and evidence obligations remain applicable.
Implementing units and limits are in the [detailed design](../../swe.3-detailed-design/05-insight-generation.md); executed evidence is in the [verification record](../../../verification/results/insight-generation.md).

## Shared structure and clocks

`capture_ref` is an object with `source: "system.json#/capture"`, `boot_epoch`, `uptime_seconds_at_start`, and `log_window_ref`.
The older string shorthand denotes its `source` member; five artifacts never re-derive capture.
Unknown `is_cold_boot` and `log_window.is_contiguous` are null with evidence gaps, following the shared null discipline.
Host generator timestamps are distinct from device/evidence clocks.
Without capture comparison context, provenance is undated and comparison timestamps/ages are null, even where a domain record retains an absolute occurrence timestamp.

Current log-window bounds use retained SYSTEM/EVENT/RADIO/STATS buffer timestamps less than one hour before capture.
Rows with older embedded clocks and LAST LOGCAT are retained separately; they do not extend that current cohort.
The year comes from capture with December/January rollover handling; an explicit same-day trace offset can resolve a banner lacking timezone.
Post-banner collection is reported, with age clamped to zero rather than negative.
Freshness and cross-boot provenance always accompany severity; stale/undated high or critical becomes medium while `unconstrained_severity` retains the rule result.

Raw searches cover the main report plus detected outer text entries.
Coverage lists scanned IDs and excluded binary/nested ZIP IDs; historical main-only example counts do not define archive-wide raw totals.
Nested ZIP metadata is limited to the outer entry's observed presence/size/type; nested inventories requiring recursion carry a gap.
PROCRANK process rows use `source_channel: "procrank"` alongside `dumpsys_meminfo` and `bugreport_procdump`.
Its ZSwap values remain estimates and never replace printed SwapPSS.

## Closed finding kinds and severity rules

These are the emitted kinds and their unconstrained rules.
Every high/critical rule publishes metrics; the shared freshness cap applies afterwards.
A rule describes retained evidence rather than a universal device-health assertion.

| Artifact | Kind | Rule and metrics | Severity |
| --- | --- | --- | --- |
| all | `signal_absence` | Zero structurally validated records for a published raw search; `raw_hits`, `validated_records`. | info |
| system | `archive_identity` | Identified report; stored bytes and ZIP entries. | info |
| system | `cold_boot_context` | Canonical capture indicates a cold boot; uptime/log-window context. | low |
| apps | `package_inventory` | Parsed snapshot; package/process/shared-UID counts. | info |
| apps | `process_churn` | Retained lifecycle activity; count, measured coverage/window and cold-boot context. | low |
| anr | `anr_incident`, `historical_anr_report` | Structurally supported live/event report; `incident_records`. | high before freshness cap |
| anr | `retained_anr_trace` | Parsed historical trace; process/file count and trace uptime. | info |
| crashes | `crash_loop` | All five configured criteria pass; repeat/rate/window/gap/escalation metrics. | critical before freshness cap |
| crashes | `crash_group` | Supported group failing loop criteria; repeat/rate/window/gap metrics. | high before freshness cap |
| crashes | `reported_crash_event` | Event-log report when primary Java records are absent; retained event rows. | high before freshness cap |
| memory | `memory_snapshot` | Printed memory level critical/low; RAM/free ratio and level. | high; otherwise info |
| memory | `oom_counter` | Printed since-boot oom_kill counter >0; counter value, with undated temporal scope. | critical before freshness cap |
| memory | `lmkd_kill` | Parsed kill with oom_score_adj ≤0; RSS and adjustment. | high; otherwise medium |
| memory | `historical_memory_aggregate` | A stale procstats block; aggregation count. | info |
| cpu | `monitor_contention` | Distinct ART Java-monitor wait ≥1000 ms; wait/waiter metrics. | medium; otherwise low |
| cpu | `cpu_utilization` | Printed sample total ≥80%; sample total/window. | medium; otherwise info |

No absence rule asserts a healthy device.
Negative evidence distinguishes literal platform assertions from scoped archive-search absence.
Process fallback keys are window-local; PID alone never establishes a cross-channel identity.

## Crash classification and signatures

`classification` remains the enum `crash_loop` / `recurring` / `isolated` / `stale_recurring`.
Applied values live in sibling `classification_thresholds`, replacing the impossible nested `classification.thresholds` spelling.
Defaults are repeat count 5, rate 1/s, window ≤300 s, maximum gap from printed `min_crash_interval` then 120000 ms, and at least one escalation observation.
All five criteria must pass for `crash_loop`; stale groups cannot become current loops.
Identical signatures partition into current, stale and undated cohorts before repeat/rate/escalation counting.
The reusable `InsightOptions` API accepts alternate positive finite bounds with repeat count ≥2; no new CLI flags are implied.

`group_id` is the first 16 hexadecimal characters of SHA-256 over the normalized signature plus freshness cohort, reusing the workspace hash implementation.
Java duplicate detection requires equal process, subject PID, exact timestamp, exception and normalized frame, replacing the weaker `same_pid_same_second` proposal.
Duplicate prefix/logging identities remain separate from the subject PID.
Native signatures sharing termination frames split at the first differing normalized frame; abort-message wording never substitutes for stack identity.
Java-buffer records, event reports and native tombstones retain separate count authorities.
Cross-channel ANR joins without a shared platform incident token are inferred, with the heuristic basis recorded.

## Reference corrections and exclusions

The reference report has 33 raw long-monitor lines, 30 distinct parsed events and 17 tagged `system_server` events, all below one second.
The ≥1-second system_server subset is distinct from the full monitor channel.
`dvm_lock_sample` still has 168 sampled events; its unknown sampling gate prevents extrapolation to an exact total contention count.

Unrecognized OEM fields remain null with evidence gaps instead of guessed values.
Read/expansion/retention budgets and invalid invariant failures abort publication; successful output never silently represents a truncated parse as complete.
The runtime validator checks documented required structures and cross-field invariants, while fixture assertions check actual analysis semantics.
