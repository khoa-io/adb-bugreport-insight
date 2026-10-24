# Verification strategy

Verification has three purposes: [SWE.4](../swe.4-unit-verification/index.md) checks detailed unit contracts, [SWE.5](../swe.5-component-and-integration-verification/index.md) checks components and interfaces, and [SWE.6](../swe.6-software-verification/index.md) checks committed software obligations.
Reuse tests only where assertions support each claim.

## Select checks by change

| Changed concern | Selected regression set |
| --- | --- |
| Index, reading, detection, limits, glob/search | Core unit/hostile measures, affected MCP handlers, related committed requirement criteria |
| Handler parameters, schemas, envelopes, error mapping | MCP tools suite plus wire validation/process checks; core checks when its contract changes |
| CLI, transport, diagnostics, lifecycle | CLI protocol suite, affected handler checks, startup/EOF/stdout requirements |
| Documentation only | Markdown structure, relative links/anchors, preserved ID sets, source consistency, PlantUML/SVG freshness, diagram links and notation via the [diagram guide](../guides/03-diagrams.md#diagram-authoring-and-markdown-preview) |
| Full committed-scope claim or release | All committed requirements with explicit evidence and verdicts; proposals excluded |

Use synthetic fixtures for deterministic negative cases and the approved private reference archive for measured real-input behavior.
Missing-fixture early returns leave assertions unexercised despite Cargo success.

## Execution procedure

1. Identify affected requirement/architecture IDs and detailed-design contracts.
2. Select named tests or a defined inspection method, expected outcomes, prerequisites, and any gaps.
3. Run relevant checks, first core, then handlers, then real process integration.
4. Record actual verdicts and failures.
   Explain skipped/unavailable checks and remaining limitations.

For Rust changes, the repository gates are:

```bash
cargo fmt --all -- --check
cargo build --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

Offline commands require dependencies already in the local Cargo cache.
Repository guidance does not guarantee host cache availability.
Current [CI](../../.github/workflows/rust.yml) runs `cargo build --verbose` and `cargo test --verbose` on `ubuntu-latest` for pushes/PRs targeting `main`; it does not run the formatting, Clippy, or offline gates and does not provision the private reference archive.

## Result records

Store execution records under repository-root `verification/results/` or link durable PR/CI evidence.
Keep only current verdicts, methods and open limits in `docs/verification/`.
Record date, tested revision plus relevant uncommitted changes, Rust/Cargo version, OS/platform, fixture/configuration, selected commands/review methods, expected and actual outcomes, failures, skips, and gaps.
Mark missing historical fields unknown; do not infer them.
Never publish private bugreport contents or machine-specific sensitive paths.

Distinguish planned, inspected, passed, failed, and not run.
Counts alone are insufficient proof.
A complete software conformance claim requires the per-requirement walkthrough and written report; [the current qualification assessment](assessment.md) owns those verdicts.

### Result-record fields

Share run-level identity/setup; use one row per measure or equivalent linked logs without new IDs.

| Field | Required information |
| --- | --- |
| Scope and rationale | Affected REQ/ARCH/unit contracts; why each measure and regression set was selected |
| Tree and environment | Commit plus relevant uncommitted changes; date, tool versions and OS/platform; fixture identity/configuration without private contents |
| Prerequisites and sequence | Dependencies/fixture availability; core → handler → process order when applicable; any special setup/cleanup |
| Measure and expected outcome | Exact test symbol/command or bounded inspection procedure; assertion/pass-fail criterion and evidence location |
| Actual result | Passed, failed, not run or skipped, with observed result/log; identify early returns and conditional assertions that never exercised the target case |
| Disposition | Failure/gap consequence, follow-up reference, and whether the result can support the claimed scope |

Entry criteria are available prerequisites and a defined expected outcome.
Exit criteria are recorded outcomes for every selected measure, failure/skip dispositions and cleanup of temporary setup.
These criteria close verification activity without implying failed software conformance.

Excerpt changes need core text/reader, handler and real wire checks with an above-cap fixture.
The existing 25-byte byte-read check cannot replace an above-cap case.
Documentation clarifications need link/ID checks and source/assertion inspection; record Cargo unrun for that reason.
This is selection guidance, not a new run.

Before reusing evidence, compare tested tree, source/tests, fixture/configuration and expected outcome with the change.
State why the evidence still applies and which clauses it supports.
Missing tree/setup limits reuse; unknown historical fields cannot become current passes.
