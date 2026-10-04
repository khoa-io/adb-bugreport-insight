# 02 — Amendment 1

**Status:** accepted
**Amends:** [`01-decision-record.md`](./01-decision-record.md)
**Date:** 2026-10-04

Amendment 1 records two user rulings and one environmental blocker discovered while
implementing against the design. Where it corrects the original record, **the amended form is
authoritative** and lives in [`03`](./03-id-scheme.md) through
[`07`](./07-build-plan.md).

## Rulings accepted

| Ruling | Effect |
| --- | --- |
| **1 — `DRAFT-` prefix on component IDs** | Component IDs carry their status in the ID itself, not only in a status column. Applied across all four families. |
| **2 — the deleted `crates/` tree is intentional** | Build fresh. Stage 0 rewrites the manifest; it does **not** commit the deletions as a task. |

---

## 1. The `DRAFT-` prefix

### Decision

Status is legible **from the ID alone**, in every ID family:

> **`DRAFT-<KIND>-<NAME>`** — the prefix is part of the identity, not a decoration. A `DRAFT-`
> ID is draft whatever document it lives in and whatever its status column says; an unprefixed
> ID asserts committed scope. Requirements and components obey it identically.

This makes the component convention symmetric with `DRAFT-REQ-*`, which already existed at
`requirements.md:28`.

### Which IDs are prefixed

Nine components, all of which map to draft requirements: `DRAFT-ARCH-CORE-ART`,
`DRAFT-ARCH-ANR`, `DRAFT-ARCH-APPS`, `DRAFT-ARCH-STACKTRACE`, `DRAFT-ARCH-MEMORY`,
`DRAFT-ARCH-CPU`, `DRAFT-ARCH-SUPPORT`, `DRAFT-ARCH-MCP-RES`,
`DRAFT-ARCH-MCP-TOOLS-ANALYSIS`, `DRAFT-ARCH-CLI-CMD`; and four use cases,
`DRAFT-UC-03` … `DRAFT-UC-06`.

Four stay unprefixed because they map to committed requirements: `ARCH-ARCHIVE`,
`ARCH-CLI-FLAGS`, `ARCH-MCP-SRV`, `ARCH-MCP-TOOLS-ARC`, plus `UC-01` and `UC-02`.

### Why `DGM-*` is not prefixed

Every `DGM-*` ID is a **structural index into a document**, not a component. None maps to a
draft requirement on its own account: `DGM-ANALYSIS` and `DGM-ARTIFACTS` depict draft
components, but the _diagram_ is a committed artifact of the docs set.

Prefixing them would mark the specification itself draft — losing the ability to _specify_
draft work, which is the entire purpose of `requirements-draft.md`. A tool that cannot name
its own draft work cannot be worked on.

---

## 2. Ruling on superseded IDs (Rule R-ID-3)

The original record left this open: does a superseded ID keep its original form, or take
`DRAFT-` when its successor is draft?

### Decision

**A superseded ID never changes form. It keeps the exact prefix it was born with, forever.**

> **Rule R-ID-3:** A `DRAFT-` prefix is applied **once**, at the moment an element ID is minted,
> and never modified thereafter. A retired element keeps its original ID, unaltered, with its
> row marked `superseded by <new ID>`.
>
> **Corollary:** whether a superseded element was draft or committed is recorded in **its row's
> status cell**, not in its ID — because a superseded row's status cell is historical text, not
> a live claim.

### Rationale

A superseded ID is a **historical record**. Changing `ARCH-MCP` to `DRAFT-ARCH-MCP` would falsify
that record — it would assert something about a document that no longer exist, in order to
describe a decision made later. The `DRAFT-` prefix is a **live assertion about current scope**,
and a superseded ID is by definition no longer asserting anything.

Renaming would also strain the ID-stability convention at `traceability-matrix.md:95` ("an ID
is never reused for a different thing"): the reader could no longer tell which IDs were ever
simultaneously live.

### In practice, moot

**None of the four superseded IDs was ever draft.** `ARCH-CORE-ARC`, `ARCH-CLI`, `ARCH-MCP`,
and `ARCH-MCP-TOOLS` each mapped to committed requirements at retirement, and each has a
_committed_ successor. Had one been succeeded by a draft element, the answer would be
nonetheless the same: keep the original form, because the successor already carries its own
status in its own ID.

---

## 3. Ruling: the deleted `crates/` tree is intentional

**The deletion is the user's intended end state. Do not restore it. Do not "commit it" as a
task.**

Stage 0 is therefore only: rewrite the root `Cargo.toml` to a two-member workspace, and fix the
root `README.md`. All seven `path = "crates/subcommand-*"` entries at `Cargo.toml:23-28` go
away.

**This is deliberate and must stay that way.** A future reader — human or agent — may notice a
deleted `crates/` tree against a manifest that names it and "helpfully" restore it. Do not.
`crates/core`, `crates/anr`, `crates/apps`, `crates/stacktrace`, `crates/memory`,
`crates/cpu`, and `crates/support` were all deleted because every one of them is draft work:
they implement `DRAFT-REQ-GEN-03`, `DRAFT-REQ-GEN-04`, and `DRAFT-REQ-ART-*`, none of which is
committed. **Restoring them would drag ~2,000 lines of artifact code into a committed build
and defeat the crate-boundary decision** recorded in
[`01` §2.1](./01-decision-record.md#21-cratesarchive-decomposition--must-precede-the-build).

---

## 4. Offline dependency rulings

### 4.0 Verification method and evidence

**The cache inventory was verified empirically, not assumed.** Three probe crates were built
against a writable copy of the registry; all three were deleted afterwards.

> **Sandbox caveat for anyone re-running this.** The shared `~/.cargo` is not writable inside
> the agent sandbox, and an unwritable `CARGO_HOME` produces
> `failed to create directory ... Operation not permitted` — which looks like a missing
> dependency but is not. Copy the registry to a writable path first:
>
> ```bash
> mkdir -p "$TMPDIR/cargo-home"
> cp -R "$HOME/.cargo/registry" "$TMPDIR/cargo-home/"
> CARGO_HOME="$TMPDIR/cargo-home" cargo build --offline
> ```

| Probe | Result | What it proves |
| --- | --- | --- |
| `zip 2.4.2` (deflate-only) + `serde 1.0.229` + `serde_json 1.0.151` + `thiserror 2.0.20` + `tokio 1.52.3` + `tracing 0.1.44` + `clap 4.6.7` | **Builds offline.** 59 packages locked and resolved, zero fetches. | Engineer A's whole Stage 1 dependency set is available. |
| `ring 0.17.14` → `ring::digest::SHA256` | **Builds offline.** Computed `SHA256("abc")` = `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`. | Matches the NIST published vector exactly. SHA-256 is available. |
| `zip` API surface on a synthetic 2-entry archive | `data_start`, `by_index`, `last_modified`, `compression`, `encrypted`, `name_raw`, `central_directory_start` all public and working. Source: `zip-2.4.2/src/read.rs:1776,1092,1738,1718,1723,1550,954`. | The two API risks in `01` §2.1 and §2.5 are resolved. |

A reader who distrusts these findings can re-run all three probes; the commands are above.

**Absent from the cache:** `rmcp`, `rmcp-macros`, `schemars`, `regex`, `regex-automata`,
`regex-syntax`, `globset`, `sha2`, `tracing-subscriber`, and every glob crate of any name.

### 4.1 Stage 1 — confirmed unblocked for A, restructured for B

**Engineer A is unblocked.** `limits.rs`, `caps.rs`, `error.rs`, `archive.rs`, and `text.rs`
depend only on `thiserror`, `serde`, and `zip` — all cached, all verified building offline.

**Engineer B cannot write `rmcp` code**, and the original Stage 1 assignment — "lifecycle +
error mapping" — was wrong under this constraint. B's revised Stage 1 is the
**transport-independent tool layer**: params, envelope, all seven tool bodies, and the error
mapping, all written against a local trait rather than against `rmcp`. Full table in
[`07` §2](./07-build-plan.md#2-stage-1--contract).

**This is a better architecture than the original plan's.** The trait is the port/adapter
boundary. It makes the tool layer unit-testable without any protocol stack — which is exactly
what the `REQ-MCP-08` four-way test matrix needs — and it means the later `rmcp` integration is
a thin shim rather than a rewrite. B is never idle.

### 4.2 `regex` / `globset` — replace, do not defer

Neither is cached. No glob crate of any name is cached.

**Decision: hand-write `Literal` and `Glob`; defer `Regex`.**

| Engine | Grammar | Size | Status |
| --- | --- | --- | --- |
| `Literal` | byte-substring | ~20 lines | ✅ shipped |
| `Glob` | `*`, `?`, `**`, `[a-z]`, `[!a-z]`, literal, `/`-separated | ~150 lines | ✅ shipped |
| `Regex` | — | — | 🚧 `DRAFT-REQ-MCP-ARC-08` |

**Why deferring all search was rejected:** `grep_archive` and `find_entries` are two of seven
committed tools and the whole point of `UC-02`. Shipping five and calling the server done is
not acceptable.

**Why hand-writing glob is safe but hand-writing regex is not:** the glob grammar we need is
small, closed, and has no backtracking. A regex engine does. A hand-rolled regex is a ReDoS
attack surface against exactly the hostile input class this project exists to survive. That is
a real engineering objection, not a resource excuse, and it is why regex alone is deferred.

**The deferral is typed, not silent.** `PatternMode::Regex` is present in the enum and in the
requirement, and returns:

```
ArchiveError::PatternModeUnavailable { mode: Regex, reason: "no regex engine; see DRAFT-REQ-MCP-ARC-08" }
```

An agent asking for regex gets a truthful answer rather than a missing feature or, far worse, a
silent wrong result. This is why `DRAFT-REQ-MCP-ARC-08` was split out into the ID list: **the
capability is specified and versioned, just not compiled.**

**One real correctness trap:** the glob must be **segment-wise, not character-wise**, or `*`
would cross `/` and match paths the caller did not name. That gets its own test.

### 4.3 `sha2` absent — use `ring`, verified

`sha2` is absent. `sha1` and `digest` are present but are SHA-1 and a trait crate; neither
gives SHA-256.

**`ring 0.17.14` is cached with a complete closure** (`cc`, `libc`, `untrusted`, `getrandom`,
`cfg-if`) and builds offline. Verified against the NIST vector — see §4.0.

```toml
ring = { version = "0.17", default-features = false, features = ["digest", "alloc"] }
```

**No deferral.** `archive_info`'s digest ships with the tool. The `OnceLock` means it is
computed at most once per `Archive` regardless of how many tools ask.

**Swap path when the grant lands:** replace one manifest line with `sha2 = "0.10"`. The API
surface is unchanged — `ArchiveIdentity.sha256: Option<String>` — so nothing else moves. Noted
as issue 17.

### 4.4 `rmcp` version — pinned by request, blocked, with a fallback that does not block

**No version is named here, deliberately.** `rmcp 0.8.x` is what
`.agents/instructions/rust-mcp-server.instructions.md:18,22` specifies; the deleted plan's code
was never compiled (`.memory-bank/tasks/TASK004-list-archive-mcp-tool.md:114-119`); and with no
`rmcp` crate cached, what 0.8.x actually looks like cannot be resolved. Guessing a patch version
and writing API calls against it is how the deleted plan went wrong.

**The fallback that changes the Stage 1 gate:**

> `rmcp` is fetched by a separate, explicit network request and integrated by one engineer, in
> one commit, after both Stage 1s merge.
>
> **Stage 1's merge gate is: `crates/archive` compiles and publishes its API; `crates/mcp`'s
> tool layer compiles and its seven handlers pass unit tests via the `ToolHandler` trait.
> Neither requires `rmcp`.**
>
> The `rmcp` integration commit adds: manifest deps (`rmcp`, `rmcp-macros`, `schemars`),
> `tracing-subscriber`, `src/adapter.rs`, `src/handler.rs`, `src/main.rs`, and the
> `Cargo.lock` entries.
>
> **Only that one commit is blocked.** Everything else proceeds.

### 4.5 `tracing-subscriber` absent — no logging framework in Stage 1

`tracing` without a subscriber is inert, and no subscriber is cached. **Decision: direct
`eprintln!` to stderr** via a ~20-line `crates/mcp/src/diag.rs` honouring `--verbose`.
`tracing-subscriber` is added in the `rmcp` integration commit if it is wanted.

This also **strengthens** the stdout-purity guarantee: `eprintln!` cannot reach stdout by
construction, whereas a misconfigured subscriber can.

### 4.6 `zip/time` optional feature absent — `mtime` stays raw

`zip`'s `DateTime` accessors (`zip-2.4.2/src/types.rs:351-396`) sit behind the optional `time`
dependency, which is absent. With `default-features = false, features = ["deflate"]` no
calendar timestamp can be rendered.

**Decision: `EntryMeta.modified` carries `ModifiedStamp { dos_date: u16, dos_time: u16 }` — the
raw MS-DOS packed fields verbatim — plus `modified_valid: bool`.**

**This is not a compromise; it is more correct.** MS-DOS timestamps have 2-second granularity
and no timezone. Any rendered value would be false precision. The raw fields let the server — or
a later artifact — interpret them with an explicit basis, and `modified_valid: false`
distinguishes "absent or unparseable" from a real 1980-01-01 stamp.

When the grant lands, add `zip/time` and a derived `modified_iso: Option<String>` **alongside**
— never replacing the raw fields. Noted as issue 18.

---

## 5. Two corrections to `01` worth restating

| Correction | Was | Now | Evidence |
| --- | --- | --- | --- |
| `EntryMeta.data_offset` | `Option<u64>`, with an O(entry) fallback | **`u64`**, fallback deleted | `ZipFile::data_start()` is public, `zip-2.4.2/src/read.rs:1776`; probe returned `43` and `103` |
| `ArchiveStatus` location | `crates/mcp` | **`crates/archive`** | It is the outcome of an _archive_ operation; B needs it to compile the tool layer |
| Stage 1 gate | required `rmcp` | **does not**; new Stage 5.5 | §4.4 |

The first was flagged provisional in the original record precisely because
`plans/mcp_detailed.md` had never been compiled. It can now be confirmed, and the provisional
was wrong in the pessimistic direction.