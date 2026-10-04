# 01 — Design Decision Record

**Status:** accepted, amended by [`02-amendment-1.md`](./02-amendment-1.md)
**Applies to:** the `bugreport-insight` redesign, committed path only
**Author role:** Principal Software Engineer

This is the original decision record. Where Amendment 1 corrects it, **the amendment wins and
only the amended form appears in [`03`](./03-id-scheme.md) through
[`07`](./07-build-plan.md)**. What a correction *changed and why* is recorded here in the
rationale, so the reasoning survives even though the stale conclusion does not.

Three corrections landed in Amendment 1 and are called out inline below:

1. `EntryMeta.data_offset` is `u64`, not `Option<u64>` — `ZipFile::data_start()` is public.
2. `ArchiveStatus` lives in `crates/archive`, not `crates/mcp`.
3. Stage 1 does not require `rmcp`.

---

## 1. Rule 1 conflict resolution

### The conflict

Rule 1 says a component whose requirement is draft is draft, with no mixed status.
`ARCH-MCP` has [`DRAFT-REQ-MCP-07`](../requirements.md#draft-req-mcp-07) attached, so rule 1
marks the whole MCP crate draft — but the MCP server is needed *now*. The same collision hits
`ARCH-CORE` (committed archive half, draft artifact half, one crate) and `ARCH-CLI` (three
committed flags, draft command and toggles).

### Decision

**Rule 1 is scoped to the traceability _element_, not to a crate, file, or document. `mixed`
is deleted as a status and replaced by a split.**

### Rationale

**The rule was being read at the wrong granularity, and the docs already prove the fix.**
`traceability-matrix.md:47-48` carries two rows — `ARCH-CORE-ARC` ✅ and `ARCH-CORE-ART` 🚧 —
for one crate. That is not "mixed"; it is one crate hosting two elements with independent
status. The word `mixed` appears nowhere as a status in that table. It appears only in
`architecture.md:62,69` and `ARCH-MCP-TOOLS`, and the prose at `architecture.md:72-75` argues
itself in circles: *"The crate is one unit in Cargo, so it ships as one unit; the split is a
statement about which obligations are committed, not about two crates."* That sentence admits
the model is wrong and then declines to fix it.

**A crate boundary is an implementation fact. A requirement boundary is a design fact.**
Coupling status to the container is what created this collision. Uncoupling costs nothing and
was already half-done.

**Splitting crates cannot solve it, and this is the decisive point.** `DRAFT-REQ-MCP-07` is a
property of *the server's resource surface*. Wherever the resource endpoint lives, the
requirement attaches to it, and the crate hosting `call_tool` is that same crate. To make rule
1 bite on crates, the tool registry and the resource registry would have to live in different
binaries — which [`REQ-MCP-05`](../requirements.md#req-mcp-05) (one typed tool surface) forbids.
**Splitting elements can solve it, and splitting crates cannot. Elements split.**

The rule as adopted:

> **Rule 1 (as adopted):** An *element* — a row with an ID in
> [`traceability-matrix.md`](../traceability-matrix.md) — is 🚧 if and only if every
> requirement mapped to it is `DRAFT-`-prefixed. An element is ✅ if at least one mapped
> requirement is committed. **No element may carry two statuses.**
>
> A crate is not an element and has no status. A crate hosting both statuses **must** declare
> per-module elements for each, and its row carries a `Committed elements` column listing them
> — not a status word.

This is *stricter* than the status quo. `mixed` permitted one row to hide a draft obligation
behind a committed label. The split forbids that by construction.

### Rejected: accept the draft marking and redefine "committed"

Rejected because it declares the MCP server draft, and the server is the deliverable the user
has repeatedly said is needed now. It also produces a perverse outcome: a shipped, working
server marked as not-committed work.

### Rejected: retire `DRAFT-REQ-MCP-07`

Rejected because it is a real capability the user wants eventually, and retiring it would
misrepresent the roadmap. It is **re-homed** instead, to `DRAFT-ARCH-MCP-RES`. See
[`03` §4](./03-id-scheme.md#4-superseded-ids).

### The consequence

The element split is tabulated in [`03` §3](./03-id-scheme.md#3-family-2-arch). Thirteen live
elements: 4 ✅, 9 🚧.

### Two live defects this fixes

- `traceability-matrix.md:65` — `UC-01` lists `REQ-MCP-01` … `10` as a **range**, which
  silently includes draft `DRAFT-REQ-MCP-07`. A committed use case claiming a draft
  requirement. Rule 2 violation. Fix: enumerate explicitly.
- `traceability-matrix.md:38` says *"The three mixed rows"* and names `ARCH-CORE`,
  `ARCH-CLI`, `ARCH-MCP-TOOLS` — but the matrix has no `ARCH-CORE` mixed row, and
  `architecture.md:70` marks `ARCH-MCP` ✅ while `architecture.md:92` marks `ARCH-MCP-TOOLS`
  mixed. Four marks in one document, three in another, none agreeing. The split deletes the
  category and with it the disagreement.

### One defect rule 1 does not excuse

`requirement-diagram.md:447` declares `id: "REQ-GEN-04"`. Every other document says
`DRAFT-REQ-GEN-04` (`requirements.md:29`, `requirement-diagram.md:530,586`,
`traceability-matrix.md:33,48,67,68,70`). The diagram's node therefore claims a **committed**
requirement, which by rule 1 makes the artifact writer committed — while the index row on the
same page calls it draft. One missing prefix; it silently reclassifies an entire element.

---

## 2. Holes in the committed design

Ordered by whether they block a build.

### 2.1 `crates/archive` decomposition — must precede the build

**Corrected in Amendment 1.** The original record asked whether `core` needed an internal
module split, and answered that `crates/archive` should be created fresh while `core` stays
unbuilt. That answer stands. What changed is one field:

> `EntryMeta.data_offset` is `u64`, not `Option<u64>`.
>
> **Evidence:** `zip-2.4.2/src/read.rs:1776` exposes `pub fn data_start(&self) -> u64` on
> `ZipFile`. Verified by building a probe crate against the cached `zip 2.4.2` and reading
> `data_start` from a synthetic two-entry archive — returned `43` and `103`, consistent with
> the local headers. The original record had flagged this as provisional because
> `plans/mcp_detailed.md` (deleted) could not be compiled to confirm. It can now.
>
> **Consequence:** the O(entry-size) sequential decompress-and-discard fallback is **deleted**.
> Byte-range reads are O(1) seeks.

The module layout and full API are in [`04a-core-archive-api.md`](./04a-core-archive-api.md)
and [`04b-core-archive-types.md`](./04b-core-archive-types.md).

### 2.2 stdio lifecycle — must precede the build

Unspecified anywhere in `docs/`. Decisions, with the reasoning that is not obvious:

| Question | Decision |
| --- | --- |
| stdout discipline | stdout carries JSON-RPC frames and nothing else. Any `println!` in `crates/mcp/**` is a build-blocking defect, enforced by a grep assertion in the test suite. |
| archive open timing | Opened and validated **before** the transport starts. |
| open failure | Diagnostic to stderr, exit non-zero, **do not start the stdio loop**. A server that cannot see its archive must not answer `archive_info`. This is the exact failure `REQ-MCP-08` exists to prevent: the one place where "no result" must not read as "empty result". |
| `initialize` | Advertise the `tools` capability **only**. No `prompts`, no `resources`, no `logging`. Advertising a capability with no handler is a lie to the client. |
| EOF on stdin | Normal termination, exit 0. This is what a client disconnect actually looks like; treating it as an error produces false failures in every agent harness. |
| SIGINT | Graceful shutdown, exit 0. |
| progress notifications | **Not in v1.** `grep_archive` is the only slow call and is bounded by a scan budget instead. `rmcp`'s `notify_progress` adds protocol surface with no consumer. |

**Corrected in Amendment 1:** logging does not use `tracing`, because `tracing-subscriber` is
absent from the offline cache. See [`02` §3.5](./02-amendment-1.md#35-tracing-subscriber-absent).

### 2.3 One archive, seven tools, no lock — must precede the build

Largest performance and correctness consequence; entirely unspecified.

`ZipArchive<BufReader<File>>` is `!Sync` and needs `&mut self` to read an entry. Seven tools,
one process, an async runtime. The naive answer (`Arc<Mutex<ZipArchive>>`) serialises
everything and blocks the executor.

**Decision — split the archive into an immutable index and per-call handles:**

- At startup, walk the **central directory once** into `Arc<ArchiveIndex>`. No decompression.
- **Five of seven tools become pure functions over `&ArchiveIndex`** — `list_entries`,
  `find_entries`, `find_by_type`, `entry_info`, `archive_info` — touching no handle, no lock.
- **Two tools need bytes.** `read_entry` and `grep_archive` re-open the file per call and index
  by `EntryId`. Cost: one `open` plus a central-directory parse, milliseconds at 10k entries.
  Benefit: **no shared mutable state, no `Mutex`, no `RwLock`, no blocking.**
- Content reads run inside `tokio::task::spawn_blocking`. Mandatory.

**Evidence this constraint is real:** `ZipArchive::by_index` returns a borrowing guard. A
probe that held one borrow and requested a second in the same scope failed to compile with
`E0499 cannot borrow as mutable more than once at a time`. The two-pass shape is therefore
forced by the borrow checker, not merely preferred — which is why it is cheap to get right.

`sha256` is computed **once per server lifetime**, memoised in a `OnceLock`, stamped
`computed_at_ms`. Hashing a 400 MB bugreport on every call would cost seconds per agent turn.

### 2.4 Error model — must precede the build, expensive to retrofit

Retrofitting after seven tools ship means touching seven result shapes and every client.

`ArchiveError` is a 13-variant `thiserror` enum, each carrying a **stable machine `code`**.
Mapping to MCP is one function. The rule that matters most:

> **A tool failure is never shaped like a successful empty result.**
> `Empty` is reachable only when the tool completed and determined there is nothing.
> `NotFound` only when the addressed thing does not exist. `Unreadable` only when it exists and
> could not be read.

**Corrected in Amendment 1:** `ArchiveStatus` is defined in **`crates/archive`**, not
`crates/mcp`. It is the four-way outcome of an _archive operation_, so it belongs beside
`ArchiveError`; `crates/mcp` maps it onto the wire without defining it. Engineer B needs it to
compile the tool layer.

### 2.5 Hostile-archive behaviour

Full table in [`06-hostile-archive.md`](./06-hostile-archive.md).

### 2.6 Two defects in `architecture.md` that would actively mislead an implementer

- `architecture.md:53,62,96` claims `core` depends on **`ripgrep`**. `ripgrep` is a **binary**,
  not a library. A library crate cannot depend on it. The claim is not drift, it is impossible.
- `architecture.md:53` claims `core` depends on `tar`. No TAR support is committed, yet
  `REQ-MCP-ARC-02` promises "ZIP **and** TAR entries". **Decision: v1 is ZIP-only.** "and TAR"
  is removed from `REQ-MCP-ARC-02` and returns with the draft extension work. Shipping a
  promise the build cannot honour is worse than shipping a smaller promise.

### 2.7 Can wait

Nested-archive opening; TAR-in-ZIP; progress notifications; prompts capability; SSE/HTTP
transport; non-ZIP content-type _opening_ (detection ships); pagination beyond a single cap;
multi-archive sessions (**forbidden** by `REQ-MCP-02`, not deferred).

---

## 3. Requirement quality

Ranked by cost to change later. Items 3.1–3.4 are expensive; do them now.

### 3.1 `REQ-GEN-02` — the most under-specified committed requirement

> **Now:** "Enforce size and entry count limits to prevent resource exhaustion."

No numbers. An engineer choosing 10 MB and one choosing 10 GB both satisfy it.
`verifymethod: test` asserts testability the text does not provide. Every limit becomes a
wire-format constant in seven tools once they ship.

> **Replace:** "`bugreport-insight` refuses any archive that exceeds, at open time: **10,000**
> entries in the central directory; **256 MiB** uncompressed size of any single entry,
> enforced against the decompressed stream and not only the declared header; **1 GiB** total
> uncompressed across all entries; **2 GiB** archive file size. A violation is an **error**,
> never a silent truncation. Limits are constants of the library and are not configurable per
> run."

### 3.2 `REQ-MCP-06` — one number out of seven needed

> **Replace:** "Every tool result carries `truncated: bool`, `stopped_reason: enum | null`,
> `omitted_count: int ≥ 0`, and `notes: string[]`, **all four always present, never omitted**.
> `stopped_reason` ∈ `item_cap | match_cap | scan_budget | line_too_long | byte_budget`.
> Defaults: **200** items per list result, **50** grep matches, **32 MiB** scanned per grep
> call. Excerpt cap **512 bytes**, hard. No tool returns an archive section verbatim."

### 3.3 `REQ-MCP-05` — claims a test it cannot have

"A tool that no backing capability justifies does not exist" is a design principle;
`verifymethod: test` is a lie about it. Replace the testable half and delete the
philosophical clause — §[04](./02-amendment-1.md) of the tool surface adds the `instructions`
guide that makes the principle real.

### 3.4 `REQ-MCP-09` — not enforceable for raw archive tools as written

It requires that a tool never return "a claim the corresponding artifact would not publish,
including a causal claim linking a delta to its collector." Archive tools make **no claims and
no deltas**. There is no artifact behind them. The clause is written for analyzer tools and is
vacuous against the committed seven.

**Decision: split it.** `REQ-MCP-09` (committed) keeps the locator discipline; the
causal-claim prohibition moves to `DRAFT-REQ-MCP-TOOLS-ANALYSIS`, where it belongs.

`REQ-MCP-08`, by contrast, **is** enforceable once `ArchiveStatus` is committed, and each tool
is tested on four fixtures: success, zero matches, missing entry, corrupt-deflate entry.

### 3.5 Remaining rewrites

| ID | Defect | Fix |
| --- | --- | --- |
| `REQ-GEN-01` | Design constraint typed as functional requirement; `verifymethod: test` overclaims | State the testable property: no entry written outside the output directory; streaming, bounded reads; central directory walked without decompressing. |
| `REQ-MCP-01` | "It depends on `core` and the analyzer crates" re-couples the server to draft crates | Committed dependency set is `bugreport-insight-archive` and `rmcp`. Analyzer dependencies are `DRAFT-` and recorded as such. |
| `REQ-MCP-03` | Dangling cross-reference to `REQ-GEN-01`, which states no limits | Repoint to `REQ-GEN-02` alone. |
| `REQ-MCP-04` | Correct but only inspectable | Add: a test asserts the archive's SHA-256 and mtime are unchanged after a full sweep. |
| `REQ-MCP-10` | "optional **and** out of scope" — self-contradictory | "SSE and HTTP transports are not part of this requirement set and are not implemented." **See the open ruling in [`09` §3](./09-open-items.md#3-the-one-ruling-i-did-not-make).** |
| `REQ-MCP-ARC-04` | "line range **or** byte range" is a union-typed parameter set an agent can get wrong | One tool, two explicitly-named mutually-exclusive pairs. **Addressing key becomes `entry_id`, not `path`.** |
| `REQ-MCP-ARC-05` | Typed `performanceRequirement` while its six siblings are `interfaceRequirement` | Reclassify to `interfaceRequirement`; performance belongs to `REQ-MCP-06`. **Add `byte_offset` to every match.** |
| `REQ-MCP-ARC-06` | "detected content type" is defined in **no document anywhere** | Add the `ContentType` (15 values) and `DetectionBasis` (4 values) enums, `sample_bytes`, and the rule that a type is emitted only when `basis != none`. |
| `REQ-MCP-ARC-02` | Promises TAR entries with no committed TAR source | ZIP-only. |
| `REQ-CLI-03` | The mangled bullet | See §3.6. |

### 3.6 `REQ-CLI-03` and the subcommand contradiction

Current text at `requirements.md:159`:
*"--mcp: Start an MCP server for live inspection of the bugreport. saved. Default is create a
new directory named `.<BUGREPORT>`."*

The second half is not about `--mcp`; it is the default output location of
`DRAFT-REQ-CLI-01`. Split:

> **`REQ-CLI-03`:** "`bugreport-insight BUGREPORT --mcp` starts an MCP server bound to
> `BUGREPORT` and speaks MCP over stdio for the process lifetime. The flag is the interface
> contract; the MCP Server section is the implementation contract."
>
> **`DRAFT-REQ-CLI-01` receives:** "…Default is create a new directory named `.<BUGREPORT>`."

**And this closes an open contradiction** recorded at `progress.md:70-75` — flag or
subcommand?

> **Decision: a flag, `bugreport-insight BUGREPORT --mcp`.** `REQ-CLI-03` is committed and the
> subcommand form is required by nothing committed; `REQ-MCP-02` binds one archive for the
> server lifetime, making the archive a parameter of the _mode_, not of a verb; and one command
> with one flag keeps `--help` honest. The `BUILTIN_SUBCOMMANDS` plan for `mcp` is **cancelled**.

`REQ-CLI-05` also needs a clause: **"`--verbose` sets log level on stderr. stdout carries
JSON-RPC under `--mcp` and is never a logging channel."**

### 3.7 One added requirement

**`REQ-MCP-11`** — the `initialize` response carries `instructions`: at most 40 lines naming
each tool, its cost class, its bound, and the intended call order. Published via a stdio
capability that already exists, publishing no archive content, and materially improving agent
success.

**`DRAFT-REQ-MCP-ARC-08`** — pattern mode `Regex`, split out in Amendment 1 because it is
blocked on the absent `regex` crate. See [`02` §3.2](./02-amendment-1.md#32-regex-and-globset--replace-do-not-defer).

These change the tally to **23 ✅ / 25 🚧 / 48**, from 22 / 24 / 46.

---

## 4. MCP tool surface

**Seven tools is right. Do not merge; do not add an eighth except `instructions`, which is not
a tool.**

### Granularity

`list_entries`, `find_entries`, `find_by_type` are three routes to "what is in here". The
tempting fix is one tool with a `mode` parameter and a union-typed result. **Rejected:** a
union-typed result is strictly worse for an agent — it must read a schema describing three
shapes to use one, and the return type tells it nothing about which shape arrived.

**What is genuinely missing is a discriminator, not a tool.** Each description's first
sentence names which question it answers:

- `list_entries` — what is here, with sizes. Use to orient.
- `find_entries` — which paths match a pattern. No content, no sizes. Cheapest call.
- `find_by_type` — which paths are of this content type, with sizes.

### Result shape — one envelope, uniformly

`schema_version`, `tool`, `status`, `data`, `truncated`, `stopped_reason`, `omitted_count`,
`notes`. The last four are **always present**. A uniform envelope is what makes `REQ-MCP-08`
mechanically checkable: one test loops all seven tools and asserts the keys exist and `status`
is one of four values.

### The highest-value addition

**`byte_offset` on every `grep_archive` match.** Without it, an agent that greps a 682,951-line
entry and gets 50 matches must issue 50 more calls, each rescanning from the start. With it,
`read_entry(entry_id, byte_offset, …)` resumes. This is the difference between a usable and an
unusable tool on the reference archive.

### Are `REQ-MCP-08` and `REQ-MCP-09` enforceable with no artifact schema behind them?

- **`REQ-MCP-08`: yes**, once the four-value `ArchiveStatus` is committed. The draft document
  stops being a dependency of a committed requirement.
- **`REQ-MCP-09`: no, as written** — the causal-claim clause is vacuous. Split, per §3.4.

Both have the same answer shape: **a committed requirement must not name a draft concept.**
That is rule 1 applied to requirement text rather than element status.

---

## 5. Doc restructure plan

**Superseded in detail by [`08-doc-restructure-manifest.md`](./08-doc-restructure-manifest.md),
which is the authoritative manifest.** This section records why the restructure was decided.

Rule 4's trigger: `requirement-diagram.md` is 602 lines with six canvases. `artifact-schemas.md`
is 4,973 lines but has no diagrams, so rule 4 does not apply to it.

Rule 3 forces UML types: `flowchart TD` in `architecture.md` becomes a component diagram; the
one in `use-cases.md` becomes a usecase diagram. The six existing `requirementDiagram` canvases
are already correct UML and stay.

**Naming.** The user's rule 4 names the split form `-diagram-<ID>.md`. The base document is
itself called `requirement-diagram.md`, so applying the rule literally yields
`requirement-diagram-diagram-*.md`. The naming `docs/diagram-<ID>.md` was proposed and
**accepted**, delivering what the rule intends without the stutter.

**`docs/activities.md` is retired, not revived.** An activity diagram models workflow and
decision logic. There is no committed workflow beyond "serve MCP over stdio" — that is a
state diagram. Reviving it would produce a document of invented filler.

**A new ID prefix is required by rule 4**, which the docs previously lacked: every diagram
gets a `DGM-*` ID, because otherwise there is nothing to name a file after.

---

## 6. Build order

**Authoritative and current in [`07-build-plan.md`](./07-build-plan.md).** Three things from
the original plan were corrected in Amendment 1:

1. Stage 0 does **not** commit the `crates/` deletion — the deletion is the user's intended
   end state and is not a task.
2. Stage 1 does **not** require `rmcp`. Engineer B builds the entire seven-tool layer behind a
   local `ToolHandler` trait.
3. A new **Stage 5.5** exists solely for the `rmcp` integration, and it is the only blocked
   stage.

The original Stage 0 also assumed a green build existed. It does not: `crates/` is deleted in
the working tree and `Cargo.toml:3-12` names seven non-existent members. `progress.md:130`
claims a green build with 13/13 tests passing, which is false as of 2026-10-04.

---

## 7. Deferred decisions

| Deferred | Why it can wait |
| --- | --- |
| Splitting `artifact-schemas.md` (4,973 lines) | No diagrams, so rule 4 does not apply; entirely draft; off the committed critical path. |
| Opening nested archives | `REQ-MCP-ARC-06` promises only _detection_. Opening is `DRAFT-REQ-GEN-03`. |
| Progress notifications | Bounded by scan budget instead. |
| Prompts and `logging` capabilities | No handler means no advertisement. |
| SSE / Streamable-HTTP transports | `REQ-MCP-10` explicitly excludes them. |
| `process_key`, freshness, severity, negative evidence | All draft, behind `DRAFT-REQ-ART-*`. |
| Which concrete `rmcp` 0.8.x patch version | Cannot be resolved offline; see [`02` §3.4](./02-amendment-1.md#34-rmcp-version--pinned-by-request-blocked-with-a-working-fallback). |
| Multi-archive sessions | **Forbidden** by `REQ-MCP-02`, not deferred. Never reopens. |
| Third-party scanner plugin dispatch (`DRAFT-UC-05`) | Draft, and the flag-over-subcommand ruling may reshape it anyway. |

---

## 8. Technical debt

The 18 GitHub issues are consolidated in
[`09-open-items.md`](./09-open-items.md#1-github-issues--listed-not-created). Issues 1–14 were
identified in this pass; issues 15–18 were added by Amendment 1 as a consequence of the
offline cache constraint. They are **listed, not created** — creating them is a separate,
explicitly-authorised action.