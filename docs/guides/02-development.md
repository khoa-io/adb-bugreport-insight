# Development

## Development workflow

Before coding, read the [index](../index.md) and affected requirements, architecture and design.
[AGENTS.md](../../AGENTS.md) and applicable role/instruction files govern work.
Update documentation and [traceability](../traceability.md) together.

### Workspace

| Package | Location and responsibility |
| --- | --- |
| `bugreport-insight-core` | [crates/core](../../crates/core/Cargo.toml): reusable ZIP archive library |
| `bugreport-insight-mcp` | [crates/mcp](../../crates/mcp/Cargo.toml): tool handlers and stdio server |
| `bugreport-insight-cli` | [crates/cli](../../crates/cli/Cargo.toml): `bugreport-insight` executable and argument handling |
| `bugreport-insight-test-fixtures` | [crates/test-fixtures](../../crates/test-fixtures/Cargo.toml): shared development fixtures |

Use a Rust 2024 capable toolchain; the workspace uses Cargo resolver 2.
Inspect the manifests and Cargo configuration if offline dependency resolution fails.
[Cargo.lock](../../Cargo.lock) records resolutions; workspace [Cargo.toml](../../Cargo.toml) declares members and shared dependencies.
Offline operation requires those dependencies already cached.
Historical cache observations do not establish availability elsewhere.

### Checks

Select relevant [measures](../verification/01-strategy.md#select-checks-by-change); run affected core → handler → process checks.
The repository Rust gates are:

```bash
cargo fmt --all -- --check
cargo build --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

The [fixture guide](../verification/02-fixtures-and-environments.md) explains the optional private reference input and early-return skips.
Passing counts without that input do not prove reference behavior.
[CI](../../.github/workflows/rust.yml) currently runs build/tests on Ubuntu; formatting/Clippy and private fixture coverage are separate checks.

The documentation checker includes repository-root execution records when resolving evidence links and anchors.
For documentation-only edits, validate relative files and anchors, stable IDs, authority boundaries, Markdown structure/500-line limit, and [PlantUML/SVG freshness](03-diagrams.md#diagram-authoring-and-markdown-preview).
Record actual results and tooling limitations under [shared verification](../verification/index.md).
Editorial changes must neither change behavior nor promote deferred scope.

For content review, apply the [documentation quality criteria](../conventions.md#content-review).
Review ambiguity and unsupported claims before improving explanations/examples.
Editorial closure can leave behavior discrepancies open.
The [content-review criteria](../conventions.md#content-review) define the required clarity and evidence boundaries.

## Reference workflow

The [fixture guide](../verification/02-fixtures-and-environments.md) owns reference archive measurements and setup.
Run the privacy-preserving real-case workflow after building:

```bash
python3 scripts/validate_reference_workflows.py
```

The [driver](../../scripts/validate_reference_workflows.py) requires the private reference ZIP, uses real stdio and owned temporary generation output, and fails if the fixture is absent rather than silently skipping.
Its [execution record](../../verification/results/real-case-user-validation.md) supplies source-oracle, triage, resource, collision and SHA/mtime results.
Exact physical IO/CPU/time are outside the source/stored-byte cost estimates.
