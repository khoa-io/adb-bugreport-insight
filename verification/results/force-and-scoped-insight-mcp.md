# Raised limits and scoped insight MCP verification

## Execution identity

- User request: raise limits with an explicit warned `--force`, combine `--mcp` and `--insight`, discover bugreport ZIPs only in the supplied directory, and expose insight tools; apply ASPICE SWE with delegated work.
- Run: 2026-10-21, macOS Darwin arm64, Rust 1.99.0 / Cargo 1.99.0.
- Revision: base `0a094a812a55b31d84f6646ddb0c55b2c726f8b3` plus the relevant uncommitted changes described below; extensive pre-existing changes were retained.
- Configuration: Cargo offline dependencies, synthetic ZIP fixtures and the existing optional reference fixture configuration; no real bugreport content is retained in this record.
- Roles: delegated SWE.1–3 contracts/traceability, delegated core construction/unit checks, delegated MCP construction/unit checks; parent CLI integration/SWE.5–6 and cross-review.

## Scope and selected measures

REQ-CLI-12, REQ-MCP-13, REQ-GEN-02, REQ-INSIGHT-01/05 and affected MCP02/04/05/06/10/11/12 clauses are the targeted set.
ARCH-ARCHIVE, ARCH-CLI-FLAGS, ARCH-CLI-INSIGHT, ARCH-MCP-SRV and ARCH-MCP-INSIGHT implement it.
The [traceability matrix](../../docs/traceability.md) joins contracts, sources and checks.

| Layer | Measure and expected result | Actual result |
| --- | --- | --- |
| SWE.4 core | Selected default/forced admission, produced-byte and retained-text ceilings; integrity failure preservation. | Passed: `force_limits` (3), reader policy checks (2), retained-budget checks (2); full core suite (111). |
| SWE.4 MCP | Discovery confinement, marker recognition, confirmation before content IO, fixed artifact names and bounded pages. | Passed: 16 MCP unit tests including 7 insight-provider cases; 8 resources and 42 handler checks. |
| SWE.5–6 process | Default refusal and `--force` warning/success, combined file/directory syntax, source preservation, tool discovery and paging. | Passed: 42 real-process protocol checks, 2 static-insight process checks and 1 CLI failure-exit unit check. |
| Supporting gates | Formatting, offline build, workspace tests and Clippy with warnings denied. | Passed: all four required workspace gates; 222 tests, zero failures/ignored. |
| Documentation | Local links/anchors and consistency; diagram freshness if renderer available. | Passed: 143 Markdown pages, 1,869 links, zero errors; `git diff --check` clean. Diagram check blocked by unavailable Docker daemon; this task changes no UML sources/SVGs. |

Commands executed from the repository root on the final relevant source changes:

```bash
cargo fmt --all -- --check
cargo build --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
python3 scripts/check_docs.py
git -c core.fsmonitor=false diff --check
python3 scripts/diagrams.py check
```

The first six commands passed; the diagram command failed before rendering because its configured Docker socket was absent.
Existing user deletions of archived diagrams were preserved; the task did not edit remaining `.puml` or `.svg` files.
No renderer bypass or stale generated output was introduced.

### Assertion evidence

| Source/check | Demonstrated behavior |
| --- | --- |
| [force_limits.rs](../../crates/core/tests/force_limits.rs) | 10,001 entries default refusal/forced admission; actual deflate stream above default ratio; forced CRC/narrow-read failures retained. |
| [reader policy tests](../../crates/core/src/reader.rs), [report budget tests](../../crates/core/src/insight/report.rs) | Selected produced-byte/ratio ceilings accept exact bounds and reject overflow; exact forced profile raises retained limits, custom profiles retain default analysis budgets; incomplete lines remain errors. |
| [pinned_source.rs](../../crates/core/tests/pinned_source.rs) | Renaming/replacing the display path with an outside symlink cannot redirect reads or hashes; unlinking after declined digest retains mandatory insight identity; separate archives over cloned descriptors concurrently read/hash correctly. |
| [insight provider tests](../../crates/mcp/src/insights.rs) | `discovery_confines_scope_and_uses_report_markers`, `discovery_surfaces_too_large_and_forced_ceiling`, `generation_pages_exact_artifacts_without_writes_and_invalidates_changed_input`, `preview_is_io_free_and_tokens_bind_bugreport_and_operation`, `continuation_rejects_changed_source_and_evicted_generation`, `pinned_directory_survives_path_replacement_and_refuses_child_symlinks`, `discovery_bounds_work_and_returned_records`. |
| [protocol.rs](../../crates/cli/tests/protocol.rs) | `force_admits_large_entry_count_and_explains_default_rejection`, `combined_file_mode_exposes_insight_tools_without_publishing_artifacts`, `combined_directory_mode_discovers_only_immediate_bugreport_zips_and_denies_escape`; original seven-tool mode, schemas, stdout purity and confirmations remain covered. |
| [reference suites](../../crates/core/tests/insight_reference.rs) | Existing reference insight check passed in the final workspace run (25.27 s); 19 reference archive checks also passed. Their existing environment-sensitive behavior remains documented in the fixture guide. |

REQ-CLI-12 and REQ-MCP-13 pass their changed acceptance clauses on the executed macOS configuration.
Changed default/forced policy and existing read-only/archive/resource/protocol clauses pass the selected regressions.
The three insight tool schemas remain closed typed JSON objects; source review confirms unknown keys are refused and every enabled tool has a description.
All enabled tool names, workflow/cost and continuation instructions are included at initialization, within the existing 40-line ceiling.

## Review and iteration

The existing mutually-exclusive CLI definition was inspected before changes; its rejection regression was updated to the newly authorized combination rather than preserving an obsolete expectation.
A first force-process test placed `--force` between `--insight` and its required value, causing a Clap usage failure; the test invocation was corrected.
The first large-count static fixture used 10,001 ordinary empty files and caused excessive repeated entry detection; the final fixture uses empty directory entries to exercise the same admission boundary without irrelevant content analysis.
Review identified scope races in pathname-based validation and mixed-generation paging; pinned input handling and source/generation-bound continuations now have executed regressions.
A preliminary workspace run exposed the intentionally changed unlink behavior in an old confirmation assertion, missing verbose binding detail and a transient combined-mode failure while provider code was still changing.
The assertion now verifies approved access to the pinned source, verbose binding detail was restored, and the final stable-source workspace run passes all tests.
The obsolete slow synthetic run was stopped after its replacement passed.

## Evidence boundaries

Finite raised limits are explicit opt-in; unchanged line, integrity, schema and response bounds can still refuse a forced input.
Metadata discovery recognizes supported report structure; it does not certify payload integrity or every OEM format.
New process regression tests use synthetic fixtures and compare bytes/mtime or directory contents to demonstrate read-only behavior.
Secure directory mode is implemented for macOS, Linux, iOS and FreeBSD; unsupported platforms reject directory startup explicitly.
Only macOS was executed here; other platform implementations were not cross-compiled or run.
Native non-Unix broken-response-pipe qualification remains a pre-existing independent gap.
Pinned descriptors prevent path redirection, while concurrent in-place writes are not a filesystem snapshot; insight delivery checks source digests before/after generation and rejects detected changes.
No changed UML diagram is required: existing diagrams are explicitly scoped to ordinary MCP/static flows, and combined behavior is specified in prose/tables.
