---
description: 'Lightweight Automotive SPICE SWE.1–SWE.6 adaptation for requirements, design, implementation, verification, and traceability in this Rust workspace.'
applyTo: '**'
---

# ASPICE SWE instructions

Apply these project rules when changing requirements, design, Rust code, interfaces, or tests.
For documentation-only work, apply the relevant consistency and link checks.

## Scope and tailoring

Use **Automotive SPICE 4.0, SWE.1–SWE.6** as the reference baseline.
The process identifiers below refer to [VDA QMC's PRM/PAM, §4.4](https://vda-qmc.de/wp-content/uploads/2023/12/Automotive-SPICE-PAM-v40.pdf). These are project-specific engineering instructions for a small software tool, not a reproduction of the model or a claim of assessment, certification, or capability level.

- Adapt only SWE.
  Do not introduce SYS, HWE, MAN, SUP, or other process-group deliverables.
- Work in small increments.
  The six SWE concerns can be handled in one change; separate phases, documents, teams, and approval meetings are not required.
- Reuse the existing documentation, source, tests, and review records.
  Add a separate design note only when it explains a significant decision, contract, algorithm, or risk.
- Scale verification to impact: wording changes need review; behavior changes need relevant executable checks; archive safety and protocol changes need boundary and failure checks.
- Reuse existing evidence when it remains applicable, and explain why.
  Record any omitted activity with a concrete reason; do not silently skip it because the project is small.
- Use the current user request as authorization for its stated scope.
  These instructions add no permission gate.
  Surface unresolved behavior decisions without inventing acceptance or approval.

## Existing sources of truth

Read [the documentation index](../../docs/index.md) before implementation.

| Concern | Maintain here |
| --- | --- |
| Committed software behavior and constraints | [Requirements](../../docs/swe.1-requirements/02-software-requirements.md), using stable `REQ-*` IDs |
| Deferred scope | [Draft requirements](../../docs/proposals/02-requirements.md), retaining `DRAFT-` IDs |
| Components, dependencies, interfaces, and runtime behavior | [Architecture](../../docs/swe.2-architecture/02-components-and-dependencies.md), using stable `ARCH-*` IDs |
| Requirement allocation | [Traceability](../../docs/traceability.md) |
| Requirement risk | [Committed requirements](../../docs/swe.1-requirements/02-software-requirements.md#requirement-risk) and [proposal requirements](../../docs/proposals/02-requirements.md) |
| Verification measures and results | [Shared verification](../../docs/verification/index.md) and linked SWE.4–SWE.6 pages |
| Requirement, architecture, and use-case joins | [Traceability matrix](../../docs/traceability.md) |
| Design rationale and substantial detailed design | [Detailed design](../../docs/swe.3-detailed-design/index.md), [decisions](../../docs/decisions/index.md), and Rust API documentation |
| Implementation and executable verification | `crates/core`, `crates/mcp`, `crates/cli`, and shared `crates/test-fixtures` |
| Execution and review evidence | Change/PR description, CI run, or a linked verification record in `docs/` |

Keep documentation and implementation consistent in the same change.
Historical design records explain decisions; they do not override current specifications.
Follow the [ID conventions](../../docs/conventions.md#ids-and-links) when adding IDs; preserve existing IDs and mark withdrawn items explicitly.
Do not implement draft scope unless the user authorizes its promotion and the committed specification and traceability are updated together.

## SWE.1 — Requirements

- Identify the affected `REQ-*` IDs and the originating user request, issue, or use case.
  For this standalone tool, use stakeholder intent and the operating environment as inputs; do not invent a vehicle-level specification.
- Specify observable inputs, outputs, failure behavior, and acceptance criteria before changing behavior.
  Include relevant resource limits, compatibility constraints, and units of measurement.
- Check ambiguity, contradictions, dependencies, technical feasibility, and impact on callers, filesystem access, stdout/stderr, and archive handling.
  State unresolved assumptions explicitly.
- Put new behavior in the committed requirements and record its source link or request summary in the change record.
  Record risk with requirements and verification measures in the relevant SWE.4–SWE.6 pages.
- Communicate the resulting scope and any changed contract in the change description.
  A bug fix restoring an existing requirement can reuse that ID without creating a duplicate requirement.

## SWE.2 — Architecture

- Allocate affected requirements to existing `ARCH-*` elements.
  Preserve crate boundaries unless the requested change requires a documented architectural revision.
- Update component responsibilities, dependencies, and public interfaces when they change.
  Describe relevant startup, request handling, shutdown, ownership, and concurrency behavior in prose or PlantUML/SVG; add diagrams only when they clarify the change.
- Evaluate alternatives against the actual constraints: read-only ZIP access, bounded work and responses, reusable core library, and MCP stdio integrity.
  Record significant tradeoffs.
- Update architecture and requirement mappings together.
  For an unchanged architecture, name the affected element and state that its existing contract still applies.

## SWE.3 — Detailed design and construction

- Define the changed unit's contract: relevant types/functions, valid ranges, byte/line units, error cases, state transitions, and interactions.
  Use concise API documentation for simple units and a linked design note for non-obvious behavior.
- Implement against those contracts and the repository's [Rust instructions](./rust.instructions.md) and, where relevant, [MCP instructions](./rust-mcp-server.instructions.md).
- Connect the detailed design to its `REQ-*` and `ARCH-*` IDs and identify implementing source paths and symbols.
  Use Rust item paths to identify units; a new unit-ID system is unnecessary.
- Keep source and design consistent.
  Avoid repeating obvious code in prose or adding a document for every function.
  For a refactor, explain which contracts remain the same.

## SWE.4 — Units

- Select checks against the changed unit contracts.
  Identify the units, expected outcomes, fixtures, and relevant boundary/error cases before execution.
- Use focused Rust tests for behavior changes and regressions.
  For a bug fix, demonstrate the failing behavior before the fix when practical, then confirm the corrected behavior.
- Use formatting, compiler checks, Clippy, and code review as appropriate supporting checks.
  A clean build or lint run alone does not demonstrate a behavioral requirement.
- Link unit checks to the unit/design being checked and retain their actual results.
  Existing tests may suffice; add tests when they cover a meaningful behavior or gap.

## SWE.5 — Components and integration

- Verify changed component contracts and interactions against architecture and detailed design.
  Check both component behavior and boundary behavior; compilation alone is insufficient.
- Follow the dependency sequence: core behavior, MCP handlers/server, then CLI-launched stdio.
  Confirm each affected dependency is ready before checking the consuming layer.
- Select regression checks based on the affected interfaces.
  Examples include archive-error to MCP-error mapping, tool schemas, truncation metadata, stdout purity, and startup/shutdown.
- Use shared synthetic ZIP fixtures for deterministic failure and limit cases.
  When transport or startup behavior changes, exercise the real CLI process and stdio protocol.
- Link checks to the affected `ARCH-*` elements and detailed design, and record outcomes.

## SWE.6 — Software behavior

- Verify the integrated tool against the affected committed `REQ-*` acceptance criteria, including relevant non-functional constraints.
  Historical notes calling this qualification refer to this activity; the 4.0 process name is Software Verification.
- Exercise user-visible behavior through the shipped CLI/MCP surface or public core library, according to the requirement.
  Check negative paths as well as successful results.
- For archive access changes, demonstrate read-only behavior and applicable limits.
  For bounded output changes, check completeness/truncation reporting and locator accuracy against fixtures.
- For a release or a claim that the entire committed scope is verified, walk every committed requirement and record its evidence and verdict.
  A targeted change needs the affected set and justified regression selection.
  Draft requirements remain explicitly outside that set.

## Traceability and evidence

Keep the [traceability matrix](../../docs/traceability.md) current for affected mappings.
Extend its rows or link a focused evidence table when implementation or verification detail is needed; do not create a competing requirements catalog.

Make these relationships navigable in both directions, using shared IDs and source/test symbols:

- User request/use case ↔ requirement ↔ architecture element.
- Requirement/architecture ↔ detailed design ↔ implementing unit.
- Unit design ↔ unit check; architecture/detailed design ↔ component/integration check; requirement ↔ software acceptance check.
- Each check ↔ its executed result.

A shared matrix row can provide both directions; backlinks in every function are unnecessary.
Check that assertions actually support the mapped contract.
A link or a passing test count alone is insufficient.
Reuse one check at multiple levels only when its assertions demonstrate each claimed contract.

For each selected check, record the command or review method, expected result, actual verdict, and evidence location.
Test names plus fixtures/assertions can serve as the check specification.
Identify the tested revision (including relevant uncommitted changes), date, toolchain/platform, and relevant fixture/configuration once per run.
Existing test harnesses supply setup/teardown; note special prerequisites and completion criteria when needed.

Keep the evidence summary small: affected IDs, checks selected and why, results, failures, and remaining gaps.
A PR/CI record can hold execution details; if no durable review record exists, store the summary in `docs/`.
The tracked `memory-bank/` holds working context and historical notes but must not be the sole location of shared verification evidence.
Keep real bugreport content and machine-specific sensitive data out of committed logs; prefer synthetic fixtures.

## Completion checklist

- [ ] Scope and affected requirement/architecture IDs are identified; drafts remain separate.
- [ ] Requirements, design, source, and traceability agree for the changed scope.
- [ ] Relevant unit, component/integration, and software checks are selected and executed; omitted activities have a reason.
- [ ] Results identify what was tested and distinguish passed, failed, and not run.
- [ ] Required checks pass before claiming verification complete.
  Report blocked checks and unresolved failures explicitly; a proposed test or an old run is not a new passing result.
- [ ] The change summary communicates behavior, verification evidence, and remaining limitations.

For Rust changes, run the applicable workspace gates from the repository root:

```bash
cargo fmt --all -- --check
cargo build --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

Use focused checks during iteration; run the applicable workspace gates before completion.
If offline dependencies are unavailable, report that limitation and use the project's authorized dependency workflow.
Documentation-only changes need Markdown, relative-link, and consistency checks rather than Cargo runs.
