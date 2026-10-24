# UML diagrams

## Diagram authoring and Markdown preview

Diagrams use adjacent `.puml` sources and generated `.svg` images under `docs/`.
Markdown embeds the SVG with descriptive alt text and links to its editable source.
VS Code/Zed Markdown previews display committed images without PlantUML extensions, Java or network services.
The source is authoritative; never edit generated SVGs.

Use UML use-case, component/package, class, sequence and state notation.
A source declares its type with a `diagram-type` comment.
Store requirement allocation tables in each scope’s `allocations/`, linked to canonical requirements and [traceability](../traceability.md); allocation does not prove verification.
Keep current and proposed scope explicit in pages and diagram titles.

### Setup and commands

Generation requires Python 3.11+ (stdlib only), Git and a running Docker Linux engine.
Run these commands at the repository root:

```bash
python3 scripts/diagrams.py pull
python3 scripts/diagrams.py setup
python3 scripts/diagrams.py render
python3 scripts/diagrams.py check
python3 scripts/check_docs.py
python3 -m unittest discover -s scripts/tests -v
```

`pull` downloads the renderer.
`setup` enables the tracked `.githooks/pre-commit` for the clone and its linked worktrees; repeat calls are safe.
Existing hooks paths/default pre-commit hooks require manual integration; third-party behavior is never replaced.
Hook setup is per clone; checkout does not install it.

`render` regenerates all managed diagrams and removes owned orphan images.
`check` regenerates into scratch, compares exact bytes, checks diagram links/notation and reports stale, missing or orphaned output.
Repository files and index remain unchanged.
CI runs the same check on pushes/PRs targeting `main`, including docs/tooling changes, without committing SVGs.
CI also runs `check_docs.py` for relative links/anchors and documentation presentation conventions, and exercises the real pinned renderer in a disposable Git repository.

### Pinned rendering environment

[The renderer lock](../../scripts/diagrams.lock) pins the official [PlantUML image](https://hub.docker.com/r/plantuml/plantuml/tags) by immutable amd64 image digest.
Linux CI and macOS use the same `linux/amd64` image; Apple silicon needs Docker's amd64 emulation.
Do not substitute host PlantUML: Graphviz/fonts can change layout.

The pinned image contains PlantUML 1.2026.8, Temurin Java 21.0.12+8, Graphviz 14.0.1 and DejaVu Sans from `fonts-dejavu-core` 2.37-2build1.
The image digest pins their contents together.
Rendering uses UTF-8, UTC, disabled source metadata and no container network.
Download the image beforehand; commits never silently pull toolchains.
See PlantUML's [command-line options](https://plantuml.com/command-line) for upstream behavior.
SVG font declarations add a generic sans-serif fallback for editors without DejaVu installed.
Rendering is serial; JVM `fork` reduces subprocess concurrency under amd64 emulation.
Scratch containers are named and cleaned up even after a renderer timeout.

Upgrade by changing the digest, regenerating every SVG, running checks and inspecting every layout.
Record the changed versions and verification in a dated result.
Host Python/Docker versions can differ; the pinned image determines layout.

### Sources, includes and ownership

All `docs/**/*.puml` files map to one same-directory, same-stem SVG.
Each requires one unnamed `@startuml`/`@enduml` pair.
Named/multiple outputs and case-conflicting stems are rejected.
All sources must have a Markdown image embed and source link.
Inline Mermaid/PlantUML fences in `docs/` are rejected.
[Shared style](../.style/plantuml.iuml) is a tracked local include in `docs/.style/`, reserved for diagram styling support rather than reader-facing diagrams.

Includes use an unquoted relative `!include path.iuml`.
Resolve to regular `.iuml` files inside `docs/`; reject missing/cyclic includes, other preprocessor directives/functions and external assets.
The tool expands snapshot includes before sandboxed PlantUML rendering.
Changing any source/include/renderer tool or lock regenerates the entire small set.

Generated SVGs begin with an ownership comment naming `scripts/diagrams.py`.
Cleanup removes only marked outputs without a source.
Preserve unowned SVGs; an unowned paired filename conflicts.
Rendering validates the whole batch before publishing each file atomically.
Invalid syntax, missing output or renderer errors retain all prior images.

### Commit behavior and partial staging

The hook materializes **the Git index** in scratch and executes its staged renderer scripts and lock.
Stage tooling changes with the source changes that require them.
Sources/includes/configuration use staged versions, regardless of working copies.
No stash/reset is used.
Source additions, changes, deletions/renames, or generated-output edits/deletions trigger generation.
Ordinary unrelated commits require Python/Git for the launcher but do not require Docker or PlantUML.

After all renders/validations, an index transaction stages only owned SVG updates/deletions.
A changed/locked index aborts with retry guidance.
Alternate indexes and linked worktrees are supported.
Unrelated staged and unstaged content remains intact.
Paths may contain spaces and non-ASCII characters; use `<relative path>` link destinations when they contain spaces.

With fully staged inputs, safe working previews are updated too.
Unstaged input/tool edits or staged deletions remaining on disk preserve all working previews; commits still receive SVGs for staged inputs.
It refuses independently edited working SVGs that match neither the index, HEAD nor the new render.
Save such edits elsewhere or restore the SVG before retrying.
If you rendered a preview for unstaged source B after staging A, restore that SVG before committing A; render B again afterwards.
A staged manual SVG edit/deletion is repaired safely.

The hook updates output at commit time, not continuously while typing.
Use `render` for an earlier preview.
Local hooks can be bypassed with Git's `--no-verify`; CI remains the freshness backstop.

### Recovery and disabling

For a stale/missing/orphaned SVG, run `render`, inspect the images, then stage source and output.
For a source/include syntax error, fix and stage that input before retrying.
For missing Docker or image, start the engine and run `pull`.
For macOS pull credential-helper failures, repair Docker credentials without changing the renderer lock.

Use Git's `--no-verify` only if the resulting commit still passes CI.
To remove this tool's per-clone hook setting:

```bash
python3 scripts/diagrams.py uninstall
```

Inherited/global hook settings still apply after uninstall.
Existing conflicting settings are never removed by this command.
For detailed checks and preview limits, see the [tooling execution record](../../verification/results/diagram-tooling.md).
