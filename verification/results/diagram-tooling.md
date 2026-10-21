# TASK012 — PlantUML/SVG migration and commit freshness

## Scope and baseline

Executed 2026-10-15 on `main-ai`, base `7ed36d1707fdd047e328dc180bd7fb15bd24449e`, with the pre-existing TASK011 documentation changes plus TASK012 working changes.
A pre-TASK012 snapshot protected edits and supplied the semantic baseline.
No product Rust source, behavior, committed requirement, architecture definition or conformance verdict changed.
SWE.6-T1 remains open.
Docs/contributor-tooling changes excluded Cargo gates; Python tooling checks appear below.

The [manifest](../../docs/guides/03-diagrams.md) maps 17 Mermaid blocks to nine PlantUML/SVG UML views and eight requirement tables.
The canonical requirement/architecture/traceability files are byte-identical to the pre-TASK012 working baseline.
All 50 requirement IDs, kinds, risk/method fields, element metadata and 62 former view allocation edges were compared with their original blocks and retained.
The canonical matrix's separate 94 requirement/ARCH allocation edges are unchanged.

## Environment and implementation

Authoring host: macOS on Apple silicon, Python 3.15.0, Git and a running Linux Docker engine.
The 17-test suite and Markdown checker also passed with Python 3.11.15, exercising the documented minimum Python minor version.
The [lock](../../scripts/diagrams.lock) pins the official PlantUML 1.2026.8 Linux amd64 image, digest `sha256:9b9ee6af54a86ea6ac53805e9da4da7fd913711cfe6c19b6ac1a41c398223aeb`.
Inspection inside that image reported Temurin Java 21.0.12+8, Graphviz 14.0.1, and `fonts-dejavu-core` 2.37-2build1.
DejaVuSans.ttf SHA-256: `690243adfefe0ce154b547db6205794bd30ac4277275179517a90994f4980648`.
Rendering ran this exact amd64 image through emulation, with no container network, UTF-8/UTC, source metadata disabled, and a generic CSS font fallback for editor portability.

Compatibility testing exposed intermittent Java launch stalls under amd64 emulation, retaining `jspawnhelper` processes.
It was stopped, and the common renderer now renders serially and requests `jdk.lang.Process.launchMechanism=fork`, an option supported by [OpenJDK's Linux process implementation](https://github.com/openjdk/jdk21u/blob/master/src/java.base/unix/classes/java/lang/ProcessImpl.java). This is a mitigation based on the observed stalled helpers, not a proven upstream root-cause diagnosis.
Named scratch containers are removed after rendering, including timeouts.
The initial fork-only full run also stalled; serial rendering was then selected.
Final results use serial rendering; SVG bytes are unchanged.

[Renderer](../../scripts/diagrams.py), [index transaction](../../scripts/diagram_git.py), [tracked hook](../../.githooks/pre-commit), [Markdown checker](../../scripts/check_docs.py), [CI workflow](../../.github/workflows/documentation.yml), and [contributor guide](../../docs/guides/03-diagrams.md#diagram-authoring-and-markdown-preview) share the documented contract.
AGENTS.md and applicable instruction wording now follow PlantUML/SVG and allocation-table ownership.

There was no existing hooks path or default pre-commit hook.
`python3 scripts/diagrams.py setup` enabled `.githooks` in this clone's local Git configuration.
Installation staged/committed nothing and preserved the user's index.
Tests used disposable repositories, including linked worktrees and alternate indexes.
Renderer dependencies are documentation-only.

## Selected checks and outcomes

| Command or review | Expected result | Actual result |
| --- | --- | --- |
| `python3 scripts/diagrams.py render` followed by repeated render/check | All nine diagrams render; subsequent bytes stable | Passed; repeated render reported zero updates; exact-byte check passed |
| `python3 -m unittest discover -s scripts/tests -p test_diagrams.py -v` | Hook/index contract passes with a deterministic fake Docker boundary | 17 tests passed; actual Git staging/commit behavior exercised |
| `DIAGRAM_REAL_RENDERER=1 python3 -m unittest discover -s scripts/tests -p test_real_renderer.py -v` | Real image repeatability, partial staging, invalid syntax, style propagation and hook bypass detection | One integration test passed on final serial configuration (Python 3.11.15, 43.230 s), exercising all nine diagrams |
| `python3 scripts/check_docs.py` | Markdown links/anchors resolve, docs remain within 500 lines, no Rust or inline diagram fences | Passed: 142 pages and 1,590 local links across documentation, instructions and working context |
| Semantic comparison against scratch baseline | Canonical definitions/matrix unchanged; all table metadata and edges preserved | Passed: 50 requirements, 62 former-view edges; canonical owners byte-identical |
| `git diff --check` and source/image inventory | No whitespace errors; nine sources with nine owned same-stem SVGs | Passed |
| Visual inspection of all nine SVGs via white-background librsvg rasterization | Legible labels, UML symbols, direction, no clipping/overlap; status explicit | Passed; three broad views rearranged vertically, dependency layout constrained invisibly |
| Browser review of ordinary relative SVG image embeds and source links | Images display, alt text/source navigation present | Passed in the Codex in-app browser using scratch HTML wrappers of Markdown embeds |

### Hook and failure coverage

The [17 fast tests](../../scripts/tests/test_diagrams.py) verify new/changed sources, partial staging, staged shared style with unstaged style/configuration, unstaged renderer implementation, source deletion/rename, spaces/non-ASCII paths, unrelated SVG preservation, unowned-name conflicts, cyclic/missing/remote includes, named outputs, case collisions where the host filesystem supports them, unsupported notation, syntax/renderer failures, independent local SVG edits, staged SVG edits/deletions, stale/missing/orphaned checks, idempotence, unrelated commits without a renderer, hook-install conflicts/uninstall, alternate indexes, linked worktrees and a concurrent index change.
Failure assertions compare index bytes/prior previews where relevant, beyond exit status.

The [real integration](../../scripts/tests/test_real_renderer.py) compares every committed SVG against the pinned image, renders twice, stages A then edits the source to B, and checks that both the index and actual commit contain A's SVG while B and the prior working preview remain intact.
A proven PlantUML syntax error aborts without changing index/preview.
A staged font-size change changes all nine outputs; a repeated hook run preserves the staged entries.
Finally, a deliberately stale SVG is committed with the hook disabled and the same `check` command used in CI rejects it.
No deliberately bad commits were made in the real project.

## Limits and completion

VS Code and Zed CLIs are present, but native editor UI automation was unavailable in this session.
Editor Markdown previews were **not exercised**; browser SVGs/relative embeds were inspected.
No claim of identical native-editor rendering is made.
SVG text needs a local font; the generic sans-serif fallback preserves usability without requiring DejaVu on readers' hosts.

Workflow commands ran locally; no hosted GitHub Actions run occurred.
The Linux amd64 renderer itself was exercised on macOS's Linux engine, avoiding a host-native font/Graphviz comparison.
The image's optional GD plugin warning did not prevent SVG generation; exact output and every required notation passed.

Hooks require per-clone setup and remain bypassable; CI checks freshness without auto-committing.
Local generation is commit-time or manual, with no live watcher.
Relevant commits require the downloaded pinned image.
Ordinary unrelated commits use Python/Git without invoking the renderer.
TASK012 is complete within preview/hosted-CI limits, without software sign-off or closure of product discrepancies.
