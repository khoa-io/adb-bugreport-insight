# Documentation conventions

## Authority and status

Requirements own obligations; architecture owns components/interactions; detailed design owns unit contracts.
Verification pages identify measures and evidence.
[Contributor guides](guides/index.md) own operational procedures; requirements and design retain contract authority.
Documentation must be self-contained and must not link to or rely on agent working notes.
Working notes may reference documentation.
[Traceability](./traceability.md) joins owners without restating obligations.
`swe.1-` through `swe.6-` denote lightweight ASPICE associations, not prescribed directories or approval gates.

Committed means binding scope.
Implemented, inspected, tested, and signed off are separate claims.
✅ marks committed scope/available elements; 🚧 marks deferred scope.
Neither badge proves conformance or qualification sign-off.
[Proposals](./proposals/index.md) are deferred and nonbinding.

## IDs and links

REQ identifies requirements, ARCH architecture elements, and UC use cases.
Apply `DRAFT-` once at mint time; never silently rename, reuse, or strip it.
A superseded ID keeps its original spelling and links to its successor.
An element serving a draft requirement is itself draft.
There are nine committed and eight active draft live elements; four superseded ARCH IDs and two superseded DRAFT-ARCH IDs remain addressable.
DGM remains unissued and retired; Rust item paths identify implementation units.

Each canonical definition has an explicit lowercase ID anchor.
Definitions and traceability rows link both ways.
Requirement allocation tables live in each scope’s `allocations/`; canonical requirements own obligation text.
Allocated elements describe responsibility, not passing verification.
`requirement`, `performanceRequirement`, `designConstraint`, and `interfaceRequirement` distinguish behavior, resource bounds, construction constraints and caller interfaces.
Risk and verification intent are metadata, not executed results.

## Filenames and maintenance

Use unnumbered `index.md` pages and two-digit reading-order prefixes elsewhere.
Avoid renumbering established files.
ADR numbers are permanent identities.
Use stable descriptive filenames; keep numbered decision identities permanent.
Documentation describes current contracts, rationale, procedures and verification limits.
Keep obsolete designs, migration/cleanup reports and completed-work narratives out of this tree.
Git preserves prior document versions; execution records may be maintained outside this repository.
Keep Markdown files within 500 lines; split by subject or examples.
Use CommonMark-compatible Markdown, descriptive relative links and explicit code-fence languages.
Write one prose sentence per source line, without hard wrapping within a sentence.
Preserve table, code, metadata and literal line structure.
Embed adjacent generated SVGs as Markdown images and link their PlantUML `.puml` sources.
Use UML use-case/component/package/class/sequence/state views when useful.
Use linked Markdown tables for requirement allocation; avoid SysML disguised as UML classes.
Exclude Rust snippets/declarations from `docs/`.
Use tables, prose or UML; link source symbols for implementation details.
Follow the [diagram guide](guides/03-diagrams.md#diagram-authoring-and-markdown-preview) for the pinned renderer, local includes, partial staging, hook setup and freshness checks.
SVGs are generated; never edit them manually.

Update requirements, architecture, design and traceability together when facts change.
Retained execution evidence keeps its original identity outside documentation; do not present an old run as newly executed.
Record semantic conflicts; editorial changes cannot resolve behavior decisions.

## Verification claims

Distinguish candidate tests, inspected assertions, historical runs and fresh results.
[Strategy](./verification/01-strategy.md) owns selection and reporting conventions.
Results name revision, date, environment, command/review method, verdict and evidence location; unrun checks remain unrun. References to the optional repository-root `verification/` archive are checked when it is present and tolerated when it is omitted.
Shared source/test links provide reverse traceability; per-function backlinks are unnecessary.

## Content review

Review each page's engineering question, upstream intent/contract, assumptions, failures and verifiable claims.
Requirements need observable criteria/units; design needs contracts/tradeoffs; measures need expected outcomes/evidence boundaries.

Keep one authoritative owner for each fact.
Tabulate comparable contracts; label non-obvious examples as illustrative or captured, with scope.
Prefer concrete subject/verb statements to historical narration in current specifications.
Record uncertain semantics, consequences and follow-up instead of inventing an interpretation.
Use the [verification strategy](verification/01-strategy.md#result-record-fields) for evidence criteria and applicability review.

## Date policy

Keep a date only when it explains or verifies current technical behavior.
Remove editorial update/acceptance dates, dated headings and document chronology.
Protocol-version identifiers and meaningful incident/capture timestamps retain their exact values.
Worked examples use labelled synthetic timestamps or consistent template placeholders; captured fixture measurements retain their evidence context.
Toolchain versions belong in setup guidance; execution dates and compiler/Cargo commit dates belong in external execution records.
