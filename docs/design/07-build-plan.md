# 07 — Build plan

Two engineers, disjoint write scopes, one shared file with a single owner.

> **Status: COMPLETE.** Stages 0 through 6, including 5.5, shipped in Sprint 1. This file is kept
> as the record of how the work was divided and why, not as a plan. Note that the crate name below is
> `crates/archive` because that was the name in force when the plan was written; the crate was
> rebuilt as `crates/core` — see [`01` §5.3](./01-decision-record.md#53-the-deleted-crates-tree-is-intentional).

## 1. Roles and write scopes

- **Engineer A — Sage.** Infrastructure. `crates/archive/**`.
- **Engineer B — Nova.** Client and protocol. `crates/mcp/**` and `crates/cli/**`.

> **Disjoint write scopes — the rule that prevents collisions:**
>
> A writes only `crates/archive/**`. B writes only `crates/mcp/**` and `crates/cli/**`.
> **A owns the root `Cargo.toml` through Stage 1.** B requests additions; A applies them in one
> commit at the Stage 1 merge point.

`Cargo.toml` is the only shared file, and it has one owner for a fixed window. Everything else
is disjoint by construction.

## 2. Stage 0 — Workspace (A owns, no network needed)

**The `crates/` deletion is the user's intended end state. Do not restore it. Do not "commit it"
as a task.** Stage 0 is only the manifest and the README.

| Task | Detail |
| --- | --- |
| Rewrite root `Cargo.toml` | `members = ["crates/archive", "crates/mcp"]`; `crates/cli` added at Stage 1 |
| Delete the seven `subcommand-*` paths | `Cargo.toml:23-28` — all `path = "crates/subcommand-*"` entries go away |
| Add dependencies | `bugreport-insight-archive`, `bugreport-insight-mcp`, `zip 2.4.2`, `ring 0.17`, `thiserror 2.0.20`, `serde 1.0.229`, `serde_json 1.0.151`, `tokio 1.52.3`, `tracing 0.1.44`, `clap 4.6.7` |
| Do **not** add | `regex`, `globset`, `sha2`, `rmcp`, `schemars`, `tracing-subscriber` — absent, and their absence must be **visible in the manifest**, not hidden behind `vendor/` |
| Rewrite root `README.md` | It currently documents `scan_anrs()` and `subcommand-*` dispatch for a tree that no longer exists |

**Gate: `cargo metadata --offline` succeeds with zero network access.** Proven achievable by
probe — see [`06` §7](./06-hostile-archive.md#7-verification-appendix).

> **Why `crates/core` stays deleted.** `crates/core`, `crates/anr`, `crates/apps`,
> `crates/stacktrace`, `crates/memory`, `crates/cpu`, and `crates/support` were deleted because
> every one implements draft requirements. Restoring them would drag ~2,000 lines of artifact
> code into a committed build. `crates/archive` is created fresh instead, which makes rule 1 a
> **compile error** rather than a review comment.

## 3. Stage 1 — Contract

**A publishes the API; B builds the tool layer behind a local trait.**

| Engineer A | Engineer B |
| --- | --- |
| `limits.rs`, `caps.rs`, `error.rs` | `registry.rs` — the `ToolHandler` trait |
| `archive.rs` — `Archive`, `ArchiveIndex`, `EntryMeta`, `EntryId` | `tools/params.rs` — the seven `*Params`, `deny_unknown_fields` |
| **Publishes the [`04a`](./04a-core-archive-api.md) and [`04b`](./04b-core-archive-types.md) API in the `lib.rs` doc comment** | `tools/envelope.rs` — `ToolEnvelope`, `stopped_reason` |
| `diag.rs` — the stderr helper | `tools/handlers.rs` — all seven, stubbed to `NotFound` |
| Adds `crates/cli` to members, creates its manifest | `error.rs` → `ToolError`, **without** `rmcp::ErrorData` |
| | `lib.rs` — module wiring, re-exports |

### The seam

```rust
// crates/mcp/src/registry.rs
pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn call(&self, params: serde_json::Value) -> Result<ToolEnvelope, ToolError>;
}
pub fn registry() -> Vec<Box<dyn ToolHandler>>;   // exactly seven
```

When `rmcp` lands, one adapter file wraps each handler in `#[tool]` and `#[tool_router]`. **Zero
tool logic is rewritten.** This is a better architecture than the original plan's — the trait is
the port/adapter boundary, and it makes the tool layer unit-testable without any protocol stack,
which is exactly what the `REQ-MCP-08` four-way matrix needs.

### Revised merge gate

> Both crates compile with `cargo build --workspace --offline`. The [`04a`](./04a-core-archive-api.md) and [`04b`](./04b-core-archive-types.md)
> signatures in `crates/archive/src/lib.rs` match that document **verbatim** — Engineer A's, not
> a draft. `crates/mcp`'s registry returns exactly seven handler names, and each handler's
> parameter struct rejects unknown fields. **No `rmcp` symbol appears anywhere in the tree.**

**Corrected from the original record**, which assigned B "lifecycle + error mapping" at Stage 1.
That work is `rmcp`-dependent and cannot compile offline. It moves to Stage 5.5.

## 4. Stages 2–6

| Stage | Engineer A | Engineer B | Gate |
| --- | --- | --- | --- |
| **2 — Index** | `archive.rs` complete; central directory only | handlers real against the index | 5 of 7 tools schema-valid |
| **3 — Bytes** | `reader.rs`, `text.rs` — streaming bounded reader, `MAX_LINE_BYTES`, seek, lossy UTF-8 | `read_entry`, `entry_info` | round-trip: read line _N_ of a 682,951-line entry in bounded memory |
| **4 — Search** | `detect.rs`, `search/literal.rs`, **`search/glob.rs`** — hand-written | `grep_archive`, `find_entries`, `find_by_type` | `byte_offset` on every match |
| **5 — Identity** | `identity.rs` — `ring`, memoised | `archive_info`, `list_entries`, `instructions`, the `tools/list` exact-set assertion | all seven tools live |
| **5.5 — `rmcp` integration** | — | **B, one commit, needs network grant** | `initialize` over stdio |
| **6 — Tests** | `tests/hostile.rs` — the ten fixtures | `tests/protocol.rs` + stdio smoke test | full green |

**Stage 5.5 is new.** It did not exist in the original record. Everything before and after it
proceeds.

### Stage 5.5 adds

`Cargo.toml` deps (`rmcp`, `rmcp-macros`, `schemars`, `tracing-subscriber`), `src/adapter.rs`
(trait → `#[tool]` wrappers), `src/handler.rs` (`#[tool_handler] impl ServerHandler`),
`src/main.rs` (stdio, ctrl-C, EOF), and the `Cargo.lock` entries.

**Only this one commit is blocked.** See
[`02` §4.4](./01-decision-record.md#54-dependency-rulings).

## 5. Acceptance tests

### Engineer A — `crates/archive/tests/hostile.rs`

The ten fixtures in [`06` §6](./06-hostile-archive.md#6-the-ten-fixtures). Fixture 5
(`corrupt_deflate`) is the one that catches an implementation returning `Empty` when it actually
failed to read.

### Engineer B — `crates/mcp/tests/protocol.rs`

For each of the seven tools, the four-way matrix: success · zero matches (`Empty`) · missing
entry (`NotFound`) · corrupt entry (`Unreadable`).

Plus:

- `truncated`, `stopped_reason`, `omitted_count`, `notes` present in **every** result.
- `tools/list` returns **exactly** seven names.
- Every input schema has `additionalProperties: false`.
- `grep_archive` respects `match_cap` and `scan_budget` and reports `stopped_reason`.
- **stdout under `--mcp` is pure JSON-RPC** — drive the binary, parse every line.
- **No `println!` in `crates/mcp/**`** — grep assertion.
- The archive's SHA-256 and mtime are unchanged after a full sweep. This is what makes
  `REQ-MCP-04` testable rather than merely inspectable.

### Review requirement for Stage 4

`search/literal.rs` and `search/glob.rs` are ~170 lines of security-adjacent matching code
written without a vetted crate, because none is available offline. **A second reviewer must sign
off before Stage 4 merges.** Recorded as issue 16.

The glob must be **segment-wise, not character-wise** — otherwise `*` crosses `/` and matches
paths the caller never named. That is the one real correctness trap, and it gets its own test.

## 6. What is blocked

| Blocked | On | Unblocked by |
| --- | --- | --- |
| Stage 5.5 — `rmcp`, `rmcp-macros`, `schemars` | `crates.io` unreachable | a network grant |
| `DRAFT-REQ-MCP-ARC-08` — `PatternMode::Regex` | `regex` absent | a network grant; until then a typed refusal |
| `modified_iso` | `zip/time` absent | a network grant |
| `sha2` swap for `ring` | `sha2` absent | a network grant; API surface unchanged, one manifest line |

Everything else in Stages 0–6 proceeds offline.

## 7. What changed from the original record

| Change | Was | Now |
| --- | --- | --- |
| Stage 0 | "commit the deletions" | rewrite the manifest only; the deletion is intended state |
| Stage 1 gate | required `rmcp` | does **not**; B builds the tool layer behind a local trait |
| Stage 1 for B | lifecycle + error mapping | transport-independent tool layer |
| Stage 5.5 | did not exist | the `rmcp` integration, the only blocked stage |
| Stage 2 | assume a green build | `crates/` is deleted; the workspace does not build today |
