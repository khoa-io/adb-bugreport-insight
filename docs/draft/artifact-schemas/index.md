# Artifact schemas

> **Draft.** Every file in this directory specifies output the tool does not produce. The analyzers
> behind it (`DRAFT-REQ-GEN-03`) are not built. Nothing here is a committed requirement, and no field
> is a promise. See [`../requirements.md`](../requirements.md) for what the `DRAFT-` prefix means.

This is the schema specification for the seven analysis artifacts: `README.md`, `system.json`,
`apps.json`, `anr.json`, `crashes.json`, `memory.json`, `cpu.json`. It is the contract a future
implementation of `DRAFT-ARCH-CORE-ART` would write against, and it is binding on any future artifact
author.

It was previously one file of 4,973 lines. It is split by its own sections; the content is unchanged
apart from retitling and relinking.

## Reading order

An artifact author needs [`conventions-envelope.md`](./conventions-envelope.md) and
[`conventions-evidence.md`](./conventions-evidence.md), then their own artifact's entry, and nothing
else. Read [`build-variant.md`](./build-variant.md) if the device may be a non-user build — several
fields are gated by it.

| File | Covers | Lines |
| --- | --- | --- |
| [`conventions-envelope.md`](./conventions-envelope.md) | The thirteen-key envelope, the section-status enum, and why no count is a bare number | ~380 |
| [`conventions-evidence.md`](./conventions-evidence.md) | Provenance, freshness, severity, evidence, identity, `null` discipline, the two parsing traps, units | ~450 |
| [`build-variant.md`](./build-variant.md) | `IsUserBuild()` as the single primitive, and what it gates | ~190 |
| [`artifact-readme.md`](./artifact-readme.md) | `README.md`, the human triage document | ~170 |
| [`artifact-system.md`](./artifact-system.md) | `system.json`, the device fact sheet | ~480 |
| [`artifact-anr.md`](./artifact-anr.md) | `anr.json`, keyed on the incident | ~470 |
| [`artifact-apps.md`](./artifact-apps.md) + [`-examples.md`](./artifact-apps-examples.md) | `apps.json`, inventory and process table | ~680 |
| [`artifact-crashes.md`](./artifact-crashes.md) + [`-examples.md`](./artifact-crashes-examples.md) | `crashes.json`, channels kept apart | ~550 |
| [`artifact-memory.md`](./artifact-memory.md) + [`-examples.md`](./artifact-memory-examples.md) | `memory.json`, and whether the collector perturbed the reading | ~640 |
| [`artifact-cpu.md`](./artifact-cpu.md) + [`-examples.md`](./artifact-cpu-examples.md) | `cpu.json`, utilisation and contention never merged | ~740 |
| [`divergences-and-limitations.md`](./divergences-and-limitations.md) | The D1–D5 rulings, corrected facts, and what this document cannot tell you | ~115 |

The four largest artifacts are split at `#

### Worked example`, because the worked examples are long
and are read separately from the schema they illustrate. Each `-examples.md` file continues its
sibling's numbering.

## Relationship to the committed specification

| Document | Answers | Does not answer |
| --- | --- | --- |
| [`../../requirements.md`](../../requirements.md) | **What** each artifact must let a consumer determine. | How the artifact is shaped to make that possible. |
| This directory | **How** the artifact is shaped: keys, types, enums, invariants. | Why a consumer should want the information in the first place. |

[`../../requirements.md`](../requirements.md#draft-req-art-03) states, for example, that `anr.json`
exists so an engineer can tell a live ANR from a historical one.
[`conventions-evidence.md`](./conventions-evidence.md) specifies the `freshness` enum, the
`capture_kind` field, and the severity cap that make that distinction impossible to lose.

A field that appears here but cannot be traced to an AOSP source or an archive section does not
belong here. It belongs in that artifact's *Open questions* subsection.
