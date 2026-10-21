# Artifact schemas

> **Promoted contract for `UC-03`.**
> These seven artifact structures are binding under [`REQ-INSIGHT-03`](../../swe.1-requirements/02-software-requirements.md#req-insight-03), [`REQ-INSIGHT-04`](../../swe.1-requirements/02-software-requirements.md#req-insight-04), and [`REQ-INSIGHT-05`](../../swe.1-requirements/02-software-requirements.md#req-insight-05).
> Other `DRAFT-` requirements and architecture remain deferred.

This is the normative schema specification for the seven analysis artifacts: `README.md`, `system.json`, `apps.json`, `anr.json`, `crashes.json`, `memory.json`, and `cpu.json`.
`REQ-INSIGHT-04` promotes the evidence, freshness, identity, coverage, and domain conclusions described here.


A [fact-check](../../../verification/results/aosp-source-verification.md) compares the platform claims with the Lima checkout and hash-matched fixture.
Source citations refer to `android-15.0.0_r17`; they do not prove an OEM device shipped identical binaries.
The review corrects gates, emitters, CPU/memory semantics and locators; unresolved schema/identity decisions remain there.


## Example notation

Worked examples preserve measured counts, sizes, intervals, and schema structure.
Local archive names, device/build identifiers, and the capture date are replaced with descriptive placeholders.
`<archive_stem>` is the filename stem shared by the ZIP and its main text entry; `<capture_date>` is the same redacted calendar date throughout the examples.
Ages were measured before redaction.
`<generated_at>` is the host generation timestamp; it is independent of the device capture date.
Replace notation tokens with real values before RFC 3339 parsing.
Concrete dates used solely to illustrate RFC 3339 syntax are synthetic.

## Reading order

Read [`01-envelope.md`](01-envelope.md) → [`02-evidence-conventions.md`](02-evidence-conventions.md) → [generation rules](12-generation-rules.md) → the artifact entry.
Generation rules resolve the promoted proposal’s conflicting shorthand, missing severity mappings and reference-count corrections.
Read [`03-build-variants.md`](03-build-variants.md) if the device may be a non-user build — several fields are gated by it.

| File | Covers | Lines |
| --- | --- | --- |
| [`01-envelope.md`](01-envelope.md) | The fourteen-key envelope, the section-status enum, and why no count is a bare number | ~380 |
| [`02-evidence-conventions.md`](02-evidence-conventions.md) | Provenance, freshness, severity, evidence, identity, `null` discipline, the two parsing traps, units | ~450 |
| [`03-build-variants.md`](03-build-variants.md) | Build type, report mode, permissions and runtime gates | ~190 |
| [`04-readme-artifact.md`](04-readme-artifact.md) | `README.md`, the human triage document | ~170 |
| [`05-system-artifact.md`](05-system-artifact.md) | `system.json`, the device fact sheet | ~480 |
| [`07-anr-artifact.md`](07-anr-artifact.md) | `anr.json`, keyed on the incident | ~470 |
| [`06-apps-artifact.md`](06-apps-artifact.md) + [`-examples.md`](examples/01-apps.md) | `apps.json`, inventory and process table | ~680 |
| [`08-crashes-artifact.md`](08-crashes-artifact.md) + [`-examples.md`](examples/02-crashes.md) | `crashes.json`, channels kept apart | ~550 |
| [`09-memory-artifact.md`](09-memory-artifact.md) + [`-examples.md`](examples/03-memory.md) | `memory.json`, and whether the collector perturbed the reading | ~640 |
| [`10-cpu-artifact.md`](10-cpu-artifact.md) + [`-examples.md`](examples/04-cpu.md) | `cpu.json`, utilisation and contention never merged | ~740 |
| [`12-generation-rules.md`](12-generation-rules.md) | Closed finding/severity rules, signature/clock decisions and reference corrections | ~85 |
| [`13-source-provenance.md`](13-source-provenance.md) | Module/source lookup, domain emitters and revision limits | ~100 |
| [`11-limitations.md`](11-limitations.md) | Shared decisions, reference facts and interpretation limits | ~115 |

Four large artifacts use `examples/`; schema section numbers preserve example/question references.

## Relationship to requirements

| Document | Answers | Does not answer |
| --- | --- | --- |
| [Deferred requirements](../02-requirements.md) | **What** each artifact must let a consumer determine. | How the artifact is shaped to make that possible. |
| This directory | **How** the artifact is shaped: keys, types, enums, invariants. | Why a consumer should want the information in the first place. |

The ANR contract distinguishes supported current incidents from retained historical trace evidence.
[`02-evidence-conventions.md`](02-evidence-conventions.md) specifies the `freshness` enum, the `capture_kind` field, and the severity cap that make that distinction impossible to lose.

Fields lacking AOSP/archive provenance belong in the artifact's *Open questions*.
