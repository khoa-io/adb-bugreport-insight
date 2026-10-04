# 09 — Open items

Everything here is **open**. Nothing in this file has been done.

## 1. GitHub issues — listed, not created

**These are deliberately not created.** Creating them is a separate action requiring explicit
authorisation. All 18 remain open.

| # | Title | Severity | Detail |
| --- | --- | --- | --- |
| 1 | Rewrite `Cargo.toml` to the two-member workspace; drop 7 `subcommand-*` paths | **Blocker** | `Cargo.toml:3-12` names 7 non-existent members; `:23-28` has 7 dead `path` entries. Stage 0 in [`07` §2](./07-build-plan.md#2-stage-0--workspace-a-owns-no-network-needed). **The deletion itself is not a task** |
| 2 | `requirement-diagram.md:447` — `REQ-GEN-04` missing the `DRAFT-` prefix | **High** | Silently reclassifies the artifact writer as committed. Rule 2 violation. One-character fix |
| 3 | `traceability-matrix.md:65` — `UC-01` range includes draft `DRAFT-REQ-MCP-07` | **High** | A committed use case claims a draft requirement. Rule 2 violation |
| 4 | `traceability-matrix.md:38` — "three mixed rows" contradicts `architecture.md` | **High** | Two documents disagree about which elements are mixed. Resolved by the element split |
| 5 | `architecture.md:53,62,96` — impossible `ripgrep` dependency | **High** | `ripgrep` is a binary; a library cannot depend on it |
| 6 | `architecture.md:53` + `REQ-MCP-ARC-02` — TAR promised, not committed | **High** | The tool promises what the build cannot deliver. v1 is ZIP-only |
| 7 | `REQ-GEN-02`, `REQ-MCP-06` carry no numbers | **High** | Untestable as written. Expensive after seven tools ship. Rewrites at [`01` §3.1](./01-decision-record.md#31-req-gen-02--the-most-under-specified-committed-requirement) |
| 8 | `REQ-MCP-09` vacuous against the committed tools | **Medium** | Names a draft artifact concept from a committed requirement |
| 9 | `REQ-MCP-05` claims a test it cannot have | **Medium** | Overclaims verifiability |
| 10 | No implementing element for committed `REQ-GEN-01` / `02` | **Medium** | The hole that `classes.md` + `SEQ-*` close |
| 11 | `docs/index.md` uses `<copilot-ref/>` pseudo-tags | **Medium** | Renders as nothing outside Zed. A broken index in the one file meant for navigation |
| 12 | Root `README.md` documents the deleted tree | **Medium** | Describes `scan_anrs()` and `subcommand-*` dispatch. Stage 0 |
| 13 | `.memory-bank/` is globally gitignored | **Medium** | Every settled fact lives there and ships to no one |
| 14 | `artifact-schemas.md` is 4,973 lines | **Low** | Needs splitting; entirely draft |
| 15 | **`crates.io` unreachable** — `rmcp`, `schemars`, `regex`, `globset`, `sha2`, `tracing-subscriber` absent | **High** | Blocks Stage 5.5 and `DRAFT-REQ-MCP-ARC-08`. See [`02` §4](./02-amendment-1.md#4-offline-dependency-rulings) |
| 16 | **Hand-written glob and literal search need review** | **Medium** | ~170 lines of security-adjacent matching code with no vetted crate. Second reviewer required before Stage 4 merges |
| 17 | **`ring` used instead of `sha2`** | **Low** | Pulls a C toolchain build. Swap to `sha2` when the grant lands; API surface unchanged |
| 18 | **`zip/time` absent → `mtime` is raw MS-DOS fields** | **Low** | Deliberately conservative. Add `modified_iso` alongside when available |

Issues 1–14 were found in the original pass; 15–18 were added by Amendment 1 as a consequence of
the offline cache.

## 2. Deferred decisions

| Deferred | Why it can wait |
| --- | --- |
| Splitting `artifact-schemas.md` | No diagrams, so rule 4 does not apply; entirely draft; off the critical path |
| Opening nested archives | `REQ-MCP-ARC-06` promises only _detection_. Opening is `DRAFT-REQ-GEN-03` |
| Progress notifications | Bounded by scan budget instead |
| Prompts and `logging` capabilities | No handler means no advertisement |
| SSE / Streamable-HTTP transports | `REQ-MCP-10` explicitly excludes them |
| `process_key`, freshness, severity, negative evidence | All draft, behind `DRAFT-REQ-ART-*` |
| `PatternMode::Regex` | `DRAFT-REQ-MCP-ARC-08`; typed refusal until `regex` is fetchable |
| `modified_iso` | `zip/time` absent; raw MS-DOS fields are more precise anyway |
| `sha2` swap for `ring` | One manifest line; API surface unchanged |
| Multi-archive sessions | **Forbidden** by `REQ-MCP-02`, not deferred. Never reopens |
| Third-party scanner plugin dispatch | `DRAFT-UC-05`; the flag-over-subcommand ruling may reshape it |

## 3. The one ruling I did not make

**`REQ-MCP-10`'s `verifymethod` cannot be satisfied until `rmcp` lands, and I did not decide
what to do about it.**

`requirement-diagram.md:159` marks `REQ-MCP-10` (stdio transport) as
`verifymethod: test`. But testing a stdio transport requires an `rmcp` client to speak to, and
`rmcp` is absent from the cache — see issue 15. Until Stage 5.5 lands, the requirement is marked
as something that will be tested and cannot be.

This is a **requirements-quality decision**, not an engineering one, and I left it to the user
rather than silently changing a committed requirement's verification method.

### Option A — the grant lands

`rmcp` becomes fetchable, Stage 5.5 runs, the stdio smoke test is written and passes, and
`REQ-MCP-10` keeps `verifymethod: test`. **Nothing changes in the documents.**

### Option B — downgrade to `inspection`, with an honest note

`REQ-MCP-10` becomes `verifymethod: inspection`, carrying a note that the stdio transport is
verified by inspection and by manual smoke test until the dependency is available, at which point
it returns to `test`.

### Recommendation

**Option A, and if it is not achievable, Option B applied immediately and visibly.**

The reasoning: a requirement marked `test` that cannot be tested is a small lie, and this project
has already been bitten by documentation claiming things the code does not do —
`architecture.md:53` claiming a `ripgrep` dependency that cannot exist, and `progress.md:130`
claiming a green build when `crates/` is deleted. Downgrading the method **with a visible note**
is a smaller, correctable claim than a promise that cannot be kept.

What must **not** happen is leaving it silently as `test` with no note. That is how the other two
defects survived.

**This is flagged for the user and remains open.** It is the only item in this design set where I
declined to decide.

## 4. Diagram gate caveats

Two limitations of the project's syntax gate were discovered while writing this set. Both are
recorded so a future reader does not rediscover them the hard way.

### `mermaid-svg` 0.7.0 does not support `componentDiagram` or `usecaseDiagram`

Verified by probe. Both fail with:

```
parse error: unknown diagram type: componentDiagram
```

The gate supports `stateDiagram-v2`, `classDiagram`, `sequenceDiagram`, `requirementDiagram`,
and `flowchart`. Per the standing rule, the two affected diagrams fall back to `flowchart` with
an inline comment naming the substitution. They are:

- [`03` §2](./03-id-scheme.md#2-committed-versus-draft) — committed versus draft component graph
- [`05` §1](./05-mcp-server-design.md#1-the-tool-surface) — if a use-case graph is added

**When the gate gains support, convert both back.** Rule 3 requires the UML type; the fallback is
a gate limitation, not a design preference.

### Operational details

- **`SVG_OUT_DIR` must already exist.** The validator does not create it and panics on write with
  `No such file or directory`.
- **The binary must be named.** The crate has two binaries (`doc-validate`, `direction_probe`),
  so cargo cannot choose. Use `--bin doc-validate`.

### `classDiagram` generics need tildes

`Vec~EntryMeta~`, not `Vec<EntryMeta>` — angle brackets break the parser.

## 5. Known false claims in the current documentation

Recorded so a reader knows these are **not** oversights in this design set, but pre-existing
defects the restructure must fix.

| Claim | Location | Reality |
| --- | --- | --- |
| "build succeeds, 13/13 unit tests pass" | `progress.md:130` | `crates/` is deleted; the workspace does not build |
| `core` depends on `ripgrep` | `architecture.md:53,62,96` | `ripgrep` is a binary; impossible |
| `core` depends on `tar` | `architecture.md:53` | no TAR source is committed |
| `mcp` crate exists with an `rmcp` dependency | `architecture.md:70` | no `mcp` member; no `rmcp` dependency |
| `ARCH-MCP` is ✅ while `ARCH-MCP-TOOLS` is mixed | `architecture.md:70` vs `:92` | same crate, two statuses, one page |
| `mcp` is registered as a built-in subcommand | `progress.md:44-46` | `BUILTIN_SUBCOMMANDS` does not exist; TASK004 §4.8 cancelled by the flag-over-subcommand ruling |
| `docs/index.md` navigation | `index.md:1-6` | `<copilot-ref/>` pseudo-tags render as nothing outside Zed |
| Root `README.md` usage | `README.md:16-41` | documents `scan_anrs()` and `subcommand-*` for a deleted tree |

**Issue 1 covers the workspace; issue 12 covers the README; issue 11 covers the index.** The
remaining rows are fixed by applying the restructure in
[`08`](./08-doc-restructure-manifest.md).