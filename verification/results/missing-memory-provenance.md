# Missing memory-summary provenance regression

The fresh fixed-AVD capture exposed a static-generation failure when a `SERVICE meminfo` section existed without recognized RAM summary lines.
The CLI exited 1 with `invalid insight data: memory: "memory_snapshot" provenance: missing evidence_origin`.
The section was still evidence, but `resources::memory` copied its finding provenance from a null summary object.

## Scope and SWE allocation

| Concern | Decision and evidence |
| --- | --- |
| SWE.1 | Restore [REQ-INSIGHT-03](../../docs/swe.1-requirements/02-software-requirements.md#req-insight-03) and [REQ-INSIGHT-04](../../docs/swe.1-requirements/02-software-requirements.md#req-insight-04): incomplete optional memory data must remain explicit while the complete artifact set can still be produced. Verify [REQ-INSIGHT-02](../../docs/swe.1-requirements/02-software-requirements.md#req-insight-02) input preservation. No new requirement or draft promotion. |
| SWE.2 | Existing `ARCH-INSIGHT-ANALYZERS` → `ARCH-INSIGHT-WRITER` → `ARCH-CLI-INSIGHT` allocation remains unchanged; no MCP interface change. |
| SWE.3 | [Detailed design](../../docs/swe.3-detailed-design/05-insight-generation.md#data-and-evidence-flow) now specifies the fallback in [resources.rs](../../crates/core/src/insight/resources.rs): use section-header snapshot provenance only when summary provenance is null. Existing metric-line provenance is retained otherwise. |
| SWE.4 | `memory_sections_without_summary_metrics_keep_finding_provenance` in [core tests](../../crates/core/src/insight/tests.rs) covers empty, unfamiliar, timeout-marker and check-in-style output. It asserts the header locator, null metrics, unresolved clock, undated freshness and explicit gaps. Existing memory tests preserve numeric-unit/delta coverage. |
| SWE.5 | `insight_cli_publishes_when_meminfo_has_no_recognized_summary` in [CLI tests](../../crates/cli/tests/insight.rs) exercises the real process and complete seven-file publication, null RAM, section provenance, empty stdout and unchanged input bytes/mtime. |
| SWE.6 | The same process assertions check the affected requirement behavior; a real fresh-report rerun additionally verifies that the original failure no longer aborts publication. Full-suite checks cover unchanged MCP and publication error paths. |

The source section may have a genuine dump timestamp; existing snapshot clock resolution still applies.
Without one, the finding remains undated.
The fix supplies no RAM values, source timestamps or healthy-device verdicts.
Skipping the finding or weakening the validator would hide existing evidence or permit invalid findings; retaining the section locator preserves both contracts.
No architecture diagram changes are needed for this local fallback.

## Execution

Executed on 2026-10-19, macOS arm64, Rust `1.99.0 (b940084d7 2026-09-28)`.
Base revision was `0a094a812a55b31d84f6646ddb0c55b2c726f8b3` with these uncommitted source/test/documentation changes and unrelated ongoing AOSP investigation artifacts.
Synthetic regression fixtures are generated in temporary directories; real bugreport content is excluded from this record.

| Check | Expected | Actual |
| --- | --- | --- |
| Baseline real CLI `--insight` run | Reproduce reported failure | Exit 1, missing `evidence_origin` in `memory_snapshot` provenance. |
| Added core regression before source fix | Fail on the missing provenance | Failed with the same invariant error. |
| `cargo test -p bugreport-insight-cli --offline --test insight` | Both process checks pass | 2 passed. |
| `cargo fmt --all -- --check` | No formatting differences | Passed. |
| `cargo build --workspace --offline` | All workspace components build | Passed. |
| `cargo test --workspace --offline` | No regressions, including new core and process checks | 202 passed; none failed. |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | No warnings | Passed. |
| Fresh fixed-AVD archive, rebuilt CLI `--insight` | Publish seven artifacts without altering ZIP | Exit 0, seven files, empty stdout; SHA-256 and nanosecond mtime unchanged. |

Private execution details are retained locally under `tmp/task023/static-baseline.txt` and `tmp/task023/static-fixed-result.json`.
This change verifies incomplete-memory handling; it does not correct the producer's service failures or imply that the captured AVD is healthy.
[Traceability](../../docs/traceability.md#req-insight-04) connects the affected requirements and architecture to this result.
