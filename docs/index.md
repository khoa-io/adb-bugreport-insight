# Documentation

<!-- This file routes between documents. It is not a summary of the project — the
project summary is [`../README.md`](../README.md): what the tool is, the seven
tools, the workspace layout, and how to build.

What follows is the map of the documentation set: which document is the source
of truth for which question, how the documents join, and where to start. -->

## Start here

| If you want to know | Read |
| --- | --- |
| What the tool must do | [`requirements.md`](./requirements.md) |
| What the tool is made of | [`architecture.md`](./architecture.md) |
| Which element satisfies which requirement | [`diagrams/index.md`](./diagrams/index.md) |
| Who uses the tool, and how | [`use-cases.md`](./use-cases.md) |
| Why a design decision was made | [`design/index.md`](./design/index.md) |
| What is deferred | [`draft/index.md`](./draft/index.md) |

## The set

**Committed — this is what the tool does.**

| Document | Source of truth for |
| --- | --- |
| [`requirements.md`](./requirements.md) | What the tool must do. 23 requirements, all implemented. |
| [`architecture.md`](./architecture.md) | What it is made of — crates, elements, IDs, status, structural constraints. |
| [`diagrams/index.md`](./diagrams/index.md) | Which element satisfies which requirement, with risk and verification method. |
| [`traceability-matrix.md`](./traceability-matrix.md) | The join: requirements ↔ elements ↔ use cases. |
| [`use-cases.md`](./use-cases.md) | Use cases and their actors. |

**Draft — deferred work, recorded so the reasoning survives.**

| Document | Holds |
| --- | --- |
| [`draft/requirements.md`](./draft/requirements.md) | The 27 `DRAFT-`-prefixed requirements: the artifact set, the CLI analysis command and toggles, the analyzer tools. |
| [`draft/artifact-schemas/`](./draft/artifact-schemas/index.md) | Field-level schemas for the seven analysis artifacts. |
| [`draft/index.md`](./draft/index.md) | What the `DRAFT-` prefix means, and the rule for working against it. |

**Historical — decisions taken before implementation.**

| Document | Holds |
| --- | --- |
| [`design/index.md`](./design/index.md) | What was decided, what was rejected, what is still open. |

`design/` is not deferred work and not a plan. It is a record of reasoning, and
it spans both halves of the set above.

**50 requirements: 23 committed, 27 draft.** No committed requirement depends on
a draft concept, so nothing in the committed scope is traceable into the draft
half.

## How the documents join

Three joins carry the set, and they are not the same join:

| Join | Carried by |
| --- | --- |
| requirement → element, with risk and verification | [`diagrams/index.md`](./diagrams/index.md) |
| requirement ↔ element ↔ use case, both directions | [`traceability-matrix.md`](./traceability-matrix.md) |
| decision → the requirement it justified | [`design/index.md`](./design/index.md) |

The split is deliberate. `docs/` is the source of truth for *scope*; `design/`
is the source of truth for *reasoning*. Where a design record and a
specification document disagree, the specification wins.

**Recent maintenance:** `crates/mcp/tests/tools.rs` cleaned for `clippy::nursery` (`literal_string_with_formatting_args` at line 130) and `clippy::pedantic` assertions; documentation comments updated with backticks for `snake_case` vocabulary. No functional change.
