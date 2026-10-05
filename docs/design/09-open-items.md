# 09 — Open items

Re-verified 2026-10-05 against the working tree. Every item carries a verdict: **resolved**
(with the command that proved it), **still open** (with its blocker), or **moot** (with the
decision that settled it). Nothing is listed open on trust.

**Verdict counts: 9 resolved · 3 moot · 5 still open · 1 partially resolved.**

> **No GitHub.** Open items stay *listed, not created*, per §1 and the user directive. Work is
> tracked as memory-bank tasks.

---

## 1. Item ledger

| # | Original claim | Verdict | Evidence (one line) |
| --- | --- | --- | --- |
| 1 | `Cargo.toml` names 7 non-existent members; 7 dead `path` entries | **RESOLVED** | `Cargo.toml:3-7` = `crates/{core,mcp,cli}`; `cargo metadata --no-deps --offline` exit 0 |
| 2 | `requirement-diagram.md:447` — `REQ-GEN-04` missing `DRAFT-` prefix | **STILL OPEN** | `:447` still reads `id: "REQ-GEN-04"`; `:586` calls the same ID `DRAFT-REQ-GEN-04` |
| 3 | `traceability-matrix.md:65` — `UC-01` range includes draft `DRAFT-REQ-MCP-07` | **RESOLVED** | `traceability-matrix.md:84` enumerates 20 IDs, none `DRAFT-`; no `…` anywhere in file |
| 4 | `traceability-matrix.md:38` — "three mixed rows" | **RESOLVED** | `grep -ci mixed` → **0**; 14 arch rows, all ✅ or 🚧 |
| 5 | `architecture.md:53,62,96` — impossible `ripgrep` dependency | **MOOT** | User chose `grep-*` crates; declared in `Cargo.toml:20-24`, vendored |
| 6 | `architecture.md:53` + `REQ-MCP-ARC-02` — TAR promised, not committed | **RESOLVED** | `requirements.md:102` now reads "ZIP-only (`zip 2.4.2`); TAR is not supported" |
| 7 | `REQ-GEN-02`, `REQ-MCP-06` carry no numbers | **RESOLVED** | `requirements.md:27` gives 10,000 / 256 MiB / 1 GiB / 2 GiB; `:73` gives 200 / 50 / 32 MiB |
| 8 | `REQ-MCP-09` vacuous against the committed tools | **RESOLVED** | `requirements.md:81` names `entry_id`, `text|binary|nested_zip`, 512-byte cap — all committed |
| 9 | `REQ-MCP-05` claims a test it cannot have | **RESOLVED** | `requirements.md:72` scopes the claim to a `tools/list` regression test; deps now present |
| 10 | No implementing element for `REQ-GEN-01` / `REQ-GEN-02` | **RESOLVED** | `requirement-diagram.md:74-75` — `zip_reader - satisfies -> read_only_zip` / `safety_limits` |
| 11 | `docs/index.md` uses `<copilot-ref/>` pseudo-tags | **STILL OPEN** | `docs/index.md:1-6` — six `<copilot-ref/>` lines, unchanged |
| 12 | Root `README.md` documents the deleted tree | **STILL OPEN** | `README.md:31` still `scan_anrs()`; `:39` still `bugreport-insight-subcommand-support` |
| 13 | `.memory-bank/` is globally gitignored | **RESOLVED (evidence corrected)** | `git check-ignore -v .memory-bank` → `/Users/p2p/.gitignore_global:10:.memory-bank/` |
| 14 | `artifact-schemas.md` is 4,973 lines | **STILL OPEN** | `wc -l` → **4973**, exact; entirely draft, no diagrams, off the critical path |
| 15 | **`crates.io` unreachable** — `rmcp`, `schemars`, `regex`, `globset`, `sha2` absent | **RESOLVED (was false)** | All present — see §2. `tracing-subscriber` is the only one genuinely absent |
| 16 | Hand-written glob and literal search need review | **MOOT** | Superseded by #5: `grep-matcher`/`grep-searcher`/`grep-regex`/`grep-printer` replace it |
| 17 | `ring` used instead of `sha2` | **MOOT** | User decision + `sha2 0.10.9` cached; neither crate is in `Cargo.toml` yet |
| 18 | `zip/time` absent → `mtime` is raw MS-DOS fields | **STILL OPEN (reason corrected)** | `time 0.3.37` is absent from cache and vendor; `zip/chrono` is an untested alternative |

---

## 2. Item 15 — corrected. The claim was false.

The original item asserted `rmcp`, `schemars`, `regex`, `globset`, `sha2` and `tracing-subscriber`
are **absent** from the offline cache, and treated Stage 5.5 as blocked on that basis.

Five of the six are present. Verbatim:

```console
$ ls ~/.cargo/registry/cache/*/ | grep -Ei '^(rmcp|zip|schemars|sha2|globset|regex|tracing-subscriber|grep-|ring|glob)' | sort
globset-0.4.20.crate
regex-1.13.1.crate
regex-automata-0.4.18.crate
regex-syntax-0.8.11.crate
ring-0.17.14.crate
rmcp-0.8.5.crate
rmcp-macros-0.8.5.crate
schemars-1.2.2.crate
schemars_derive-1.2.2.crate
sha2-0.10.9.crate
zip-2.4.2.crate
```

Extracted sources agree — same set under `~/.cargo/registry/src/*/`.

**`tracing-subscriber` is genuinely absent.** `tracing 0.1.44`, `tracing-core`, and
`tracing-attributes` are present, but without a subscriber `tracing` emits nothing. The practical
consequence is unchanged and is already recorded in
[`02` §4.5](./02-amendment-1.md): log to **stderr** via `eprintln!`. Stdout is protocol-only.

**The cache is not small.** 291 `.crate` files, not the 65 a previous agent reported from the
wrong machine.

### Two scope corrections this item needs

1. **`rmcp` is 3.x now, not 0.8.5.** `Cargo.toml:25` declares `rmcp = "3.5.0"`. The cache holds
   only `0.8.5`, and the user is fetching 3.5.0. **Every statement in this design set that reasons
   from the rmcp 0.8.5 API is stale** and must be re-scoped to 3.x.
   `.agents/instructions/rust-mcp-server.instructions.md` has already been rewritten for 3.x and
   carries the warning at line 8.
2. **`grep-*` is a sixth dependency group**, vendored under `/Users/p2p/scm/local/tmp/bugreport-insight/vendor/`
   (307 entries): `grep-matcher-0.1.9`, `grep-regex-0.1.14`, `grep-pcre2-0.1.10`,
   `grep-searcher-0.1.17`, `grep-printer-0.3.1`, plus `bstr`, `regex-automata`, `regex-syntax`,
   `aho-corasick`, `memchr`, `log`. They are **libraries**, legitimately depended on — see §3.

---

## 3. Item 5 — moot by user decision, and the correction is *not cosmetic*

The original reasoning was half right and is worth preserving: **`ripgrep` is a binary and can
never be a library dependency.** A crate cannot depend on an executable.

What the item missed is that ripgrep's *constituent crates* are ordinary libraries. The user chose
to depend on them directly rather than hand-write the matching layer. `Cargo.toml:20-24`:

```toml
grep-matcher = "0.1.9"
grep-regex = "0.1.14"
grep-pcre2 = "0.1.10"
grep-searcher = "0.1.17"
grep-printer = "0.3.1"
```

All five are vendored. Confirmed present:

```console
$ ls -d /Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-*
/Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-matcher-0.1.9
/Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-pcre2-0.1.10
/Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-printer-0.3.1
/Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-regex-0.1.14
/Users/p2p/scm/local/tmp/bugreport-insight/vendor/grep-searcher-0.1.17
```

**This makes item 16 moot too.** The ~170 lines of hand-written glob and literal matching that
needed a second reviewer are replaced by a vetted engine. The security objection that motivated the
hand-written route — *a hand-rolled regex is a ReDoS surface against exactly the hostile input
class this project exists to survive* — is answered by using a real engine instead.

`grep-pcre2` is in the set; whether regex is exposed as `PatternMode::Regex` or stays deferred
behind `DRAFT-REQ-MCP-ARC-08` remains a **requirements decision for SA**, not an engineering one.
The dependency being available does not by itself make the requirement committed.

---

## 4. Item 13 — conclusion correct, evidence wrong

**`.memory-bank/` is gitignored.** The original item got this right for the wrong reason, and the
wrong reason is the part that misleads. It is not ignored by the repo's `.gitignore` — that file
holds five unrelated entries. It is ignored by the **global** `core.excludesFile`.

Verbatim:

```console
$ git check-ignore -v .memory-bank
/Users/p2p/.gitignore_global:10:.memory-bank/	.memory-bank

$ git config --get core.excludesFile
/Users/p2p/.gitignore_global

$ cat .gitignore
/target/
/tmp/
/.bugreport-*/
/.BUGREPORT/
.DS_Store

$ git ls-files --error-unmatch .memory-bank/progress.md
Did you forget to 'git add'?
```

The `error: fsmonitor_ipc__send_query` line that precedes each of these is sandbox noise on this
worktree. Judge by exit code and content.

### The practical consequence

**On a machine without that global ignore file, `.memory-bank/` becomes tracked.** Simulated by
neutralising the excludes file:

```console
$ git -c core.excludesFile=/dev/null check-ignore -v .memory-bank
exit=1        # 1 = NOT ignored
```

So the memory bank is invisible to every other machine and every collaborator, and its invisibility
depends on a file **outside this repository**. A `.gitignore` entry here would make the intent
explicit and version-controlled rather than incidental to one developer's home directory.

This is a one-line change with a real benefit and no downside. It is listed as a proposal in §7,
not applied here, because it touches repository policy rather than this file.

---

## 5. Item 18 — still open, but the stated reason needs correcting

The deferral stands: raw MS-DOS fields are what `EntryMeta.modified` carries. What changed is
*why*.

The original item says `zip/time` is absent. More precisely: **`time` 0.3.37 is absent** — the
crate, not the feature. `zip 2.4.2` does declare it:

```console
$ grep -A4 '^\[dependencies\.time\]' ~/.cargo/registry/src/*/zip-2.4.2/Cargo.toml
[dependencies.time]
version = "0.3.37"
features = ["std"]
optional = true
default-features = false
```

With `default-features = false, features = ["deflate"]`, `features = ["time"]` is simply not
enabled, and every `OffsetDateTime` accessor in `zip` is behind `#[cfg(feature = "time")]`:

```console
$ grep -c 'cfg(feature = "time")' ~/.cargo/registry/src/*/zip-2.4.2/src/types.rs
17
```

`time` itself is not obtainable offline:

```console
$ ls ~/.cargo/registry/cache/*/ | grep -Ei '^time-|^deranged|^num-conv|^powerfmt'
(no output — absent)
```

**An untested alternative exists.** `zip 2.4.2` also ships a `chrono` feature providing
`TryFrom<NaiveDateTime> for DateTime`, and `chrono 0.4.45` **is** in the cache. I did not get a
build probe to complete before the terminal degraded, so this is recorded as a lead, not a fact:

> **Unverified.** Whether `zip = { features = ["deflate", "chrono"] }` resolves and builds offline
> needs one `cargo build --offline` in a scratch crate. If it does, `modified_iso` becomes
> available now rather than "when the grant lands", and §2's deferral entry becomes unnecessary.
> Worth one command before PSE re-writes the deferral.

Either way the design's position is sound and unchanged: MS-DOS fields are *more* precise for a
ZIP than a lossy calendar render, so this is a capability addition, not a defect.

---

## 6. rmcp 3.x scope correction

`Cargo.toml:25` declares `rmcp = "3.5.0"`. The cache holds only `0.8.5`; the user is fetching
3.5.0. **Any open item or design passage that reasons from the 0.8.5 API is stale.**

The live consequences:

| Location | Stale claim | Action |
| --- | --- | --- |
| `requirements.md:82` | "speaks MCP over stdio (`rmcp` 0.8.5, verified present in the offline cache)" | **Version and evidence both wrong.** 3.5.0 is declared; 0.8.5's presence does not verify it |
| [`02` §4](./02-amendment-1.md) | Engineer B cannot write `rmcp` code; tool layer is written against a local trait | Blocked-on-`rmcp` framing is stale — the blocker is 3.5.0 arriving, not `rmcp` being absent |
| `docs/architecture.md:70` | `mcp` depends on `rmcp`, status ✅ | Consistent, but the element ID is pre-split (see §8) |

The `.agents/instructions/rust-mcp-server.instructions.md` file is already correct for 3.x and
warns at line 8 that the implementer must re-verify against 3.5.0 source.

---

## 7. Proposals for PSE

Cross-document corrections. **None applied** — `06-hostile-archive.md`, `docs/architecture.md`,
`docs/requirements.md` and the rest of `docs/design/` are PSE-owned.

### P1 — `06-hostile-archive.md:8`: the line count is **correct**. Do not change it.

**This proposal contradicts the brief, and I am flagging it deliberately.**

The brief states `06-hostile-archive.md` claims 682,951 lines and that the measured value is
1,243,790. My independent measurement does not support that:

| Scope | Newlines | Source |
| --- | --- | --- |
| **Main entry only** | **682,951** | `bugreport-sdk_gcar_…-2026-09-24-00-27-17.txt` |
| Archive-wide, all 398 entries | 1,243,691 | every entry, including zero-length |

**682,951 is exactly the main entry's line count.** The doc's table header reads *"Measured on the
reference archive"* and its first row is *"Main entry size | 64,645,191 bytes"* — the whole table
describes the main entry, and 682,951 is its correct line count. 1,243,691 is the archive-wide
figure, a different measurement. The doc is right.

Two caveats on my own numbers, stated plainly:

- I measure **1,243,691**, not the 1,243,790 quoted in the brief — a 99-line discrepancy, likely a
  counting-convention difference (trailing newline on the last entry). **The brief's figure is
  also unreproduced by me.**
- p50/p99: I measure **85 / 291** for the main entry; the brief says 83 / 325. Neither matches the
  other, and neither appears in `06`. Given the 99-line disagreement above, the brief's p50/p99
  should be re-derived before being written into any document.

**Recommendation: no change to `06-hostile-archive.md:8`.** Everything else in the table verified
exactly:

| Claim | Doc value | Measured | Verdict |
| --- | --- | --- | --- |
| Main entry size | 64,645,191 bytes | 64,645,191 | ✅ |
| Line count | 682,951 | 682,951 | ✅ |
| Longest single line | 192,666 chars | 192,666 (main entry **and** archive-wide) | ✅ |
| NUL bytes | 4, at offsets 44,988,012–15 | 4, at exactly those offsets | ✅ |
| Archive facts | 398 entries · SHA-256 `10ec0b47…` · 14,710,126 bytes · 0 encrypted · max ratio 20.44 | all confirmed | ✅ |

The one genuinely useful addition: **NUL bytes are not confined to the main entry.** The main entry
has 4; the archive has **142,123 NUL bytes across 6,643 lines**, concentrated in
`proto/*.proto`, `FS/data/tombstones/*.pb`, `FS/data/misc/profiles/**/*.prof`, and
`FS/proc/config.gz`. Worth one sentence in §1's NUL row, since `CONTAINS_NUL` will fire far more
often than the doc implies.

### P2 — `docs/architecture.md`: remove the `ripgrep` claim

Three lines assert a dependency that cannot exist. PSE owns the edit; the exact targets:

- **`:53`** — "`core` itself depends on `ripgrep`, `zip`, and `tar`."
- **`:62`** — `ARCH-CORE` row, "Depends on" column: `` `ripgrep`, `zip`, `tar` ``.
- **`:96`** — "`core` … depends only on external crates (`ripgrep`, `zip`, `tar`)".

Replace with `zip` plus the adopted `grep-*` set. Note that `tar` is separately false and is **not**
covered by the `ripgrep` ruling — see P3.

`architecture.md` is already scheduled for a full rebuild under SWE.2-T1. If that rebuild lands
first these three edits are absorbed into it; if not, they are three line edits.

### P3 — `docs/architecture.md`: the `tar` claim is a **separate** false claim

Item 6 resolved only half the problem. `requirements.md:102` correctly states ZIP-only, but the
architecture still promises TAR in four places — **`:14`** (diagram: "ZIP + TAR access"), **`:53`**,
**`:62`**, **`:87`** — while `Cargo.toml` has no `tar` and cannot acquire one offline.

The requirement and the architecture now **contradict each other**. That is exactly the class of
defect this ledger exists to catch, so it is recorded here rather than closed under item 6.

### P4 — `requirements.md:82`: wrong rmcp version and wrong evidence

> "The server speaks MCP over stdio (`rmcp` 0.8.5, verified present in the offline cache)."

`Cargo.toml:25` declares `3.5.0`. This is SA-owned, so PSE cannot apply it — routed to SA in §9.

### P5 — `docs/index.md` and `README.md`: both still describe the deleted tree

Items 11 and 12 are still open and unchanged. `README.md:31` still names `scan_anrs()`;
`:39` still names `bugreport-insight-subcommand-support`. Both are quick edits and both are on the
Stage 0 critical path.

### P6 — `.gitignore`: add `.memory-bank/` (optional)

See §4. Explicit and version-controlled beats incidental to one developer's home directory.

---

## 8. Known false claims in the current documentation

Status as of 2026-10-05, after Stages 4–5 (SWE.3-T3 Stages 4–5 / SWE.3-T4) landed. **Every row is
now closed**; the evidence column states what the check is, so the next reader can re-run it rather
than take it on trust.

| Claim | Location | Reality | Status |
| --- | --- | --- | --- |
| "build succeeds, 13/13 unit tests pass" | `progress.md:130` | `crates/` was deleted; now 3 stub crates, 5 lines of source | ✅ Fixed on disk |
| `core` depends on `ripgrep` | `architecture.md:53,62,96` | `ripgrep` is a binary; impossible | ✅ **RESOLVED** — §3.1 now names `zip`, `serde`, `serde_json`, `thiserror`, `sha2`, `grep-{searcher,regex,printer}`. `grep -ci ripgrep docs/architecture.md` → 1, and that one hit is the word "ripgrep-derived" describing what `grep-*` *is*, not a dependency |
| `core` depends on `tar` | `architecture.md:14,53,62,87` | no TAR source; ZIP-only per `requirements.md:102` | ✅ **RESOLVED** — no `tar` dependency claim remains |
| `mcp` crate exists with an `rmcp` dependency | `architecture.md:70` | `crates/mcp` had **zero** dependencies | ✅ **RESOLVED** — `crates/mcp/Cargo.toml` declares `rmcp`, `tokio`, `schemars`, and the core path dep; it is the only crate naming an `rmcp` symbol |
| `ARCH-MCP` is ✅ while `ARCH-MCP-TOOLS` is mixed | `architecture.md:70` vs `:92` | same crate, two statuses | ✅ **RESOLVED** — split into `ARCH-MCP-SRV` + `ARCH-MCP-TOOLS-ARC`; `grep -ci mixed` → 0 |
| `mcp` is a built-in subcommand | `progress.md:44-46` | `BUILTIN_SUBCOMMANDS` does not exist | ✅ Task cancelled |
| `docs/index.md` navigation | `index.md:1-6` | `<copilot-ref/>` renders as nothing outside Zed | ✅ **RESOLVED** — rewritten as plain Markdown links; also described the deleted `subcommand-*` crates |
| Root `README.md` usage | `README.md:16-41` | documents `scan_anrs()` and `subcommand-*` | ✅ **RESOLVED** — rewritten; no `scan_anrs()`, no `subcommand-*`, and the `crates/archive` naming now carries an explicit superseded note |
| `.memory-bank/` is gitignored | repo `.gitignore` | the ignore came from the **global** `core.excludesFile`, not the repo | ✅ **RESOLVED** — the repo-local `.gitignore` now carries `/memory-bank/`, so the reason no longer depends on one developer's home directory (item 13) |
| `requirements.md` cites `rmcp` 0.8.5 | `requirements.md:82` | the build uses 3.5.0 | ✅ **RESOLVED** — corrected to 3.5.0 |

### Re-runnable checks

```bash
cargo build --offline            # exit 0
cargo test  --offline            # 108 passed, 0 failed
cargo clippy --offline --all-targets   # 0 warnings

~/.cargo/bin/rust-analyzer analysis-stats . --run-all-ide-things
#   dependency lines of code: 2_008_281, item trees: 4_525   <- must not be 0

grep -ci mixed docs/traceability-matrix.md   # 0
grep -c  '…'     docs/traceability-matrix.md # 0
```

The fourteen live element anchors match between the two documents. The four R-ID-3 superseded IDs
(`ARCH-CORE-ARC`, `ARCH-CLI`, `ARCH-MCP`, `ARCH-MCP-TOOLS`) appear in `architecture.md` §9 only, so
old links still resolve while the live set stays at fourteen.

The matrix rebuild (SWE.1-T2) is **done**: `grep -ci mixed` → **0**, no `…` ranges anywhere, and
counts verified at **23 committed / 25 draft / 48 total** — `Count` column sums to 48 across 30
rows, matching `03-id-scheme.md:109`.

---

## 9. Deferred decisions

Deferred ≠ open. These are records of deliberate postponement, not defects.

| Deferred | Why it can wait |
| --- | --- |
| Splitting `artifact-schemas.md` | No diagrams, so rule 4 does not apply; entirely draft; off the critical path (item 14) |
| Opening nested archives | `REQ-MCP-ARC-06` promises only _detection_; opening is `DRAFT-REQ-GEN-03` |
| Progress notifications | Bounded by scan budget instead |
| Prompts and `logging` capabilities | No handler means no advertisement |
| SSE / Streamable-HTTP transports | `REQ-MCP-10` explicitly excludes them |
| `process_key`, freshness, severity, negative evidence | All draft, behind `DRAFT-REQ-ART-*` |
| `modified_iso` | `time` 0.3.37 not obtainable offline; raw MS-DOS fields are more precise (item 18) |
| Third-party scanner plugin dispatch | `DRAFT-UC-05`; the flag-over-subcommand ruling may reshape it |
| Multi-archive sessions | **Forbidden** by `REQ-MCP-02`, not deferred. Never reopens |

### Closed deferrals

| Was deferred | Now | Why |
| --- | --- | --- |
| `PatternMode::Regex` behind `DRAFT-REQ-MCP-ARC-08` | **Deferred on scope, not availability** | `regex 1.13.1` **is** cached and `grep-regex` is vendored. Availability was never the blocker — committing the requirement is. SA decides. |
| `sha2` swap for `ring` | **Settled: `sha2`** | `sha2 0.10.9` cached; pure Rust, no C toolchain. Neither is in `Cargo.toml` yet (item 17) |
| `tracing-subscriber` | **Settled: unavailable** | Genuinely absent. Log to stderr via `eprintln!` |

---

## 10. The one ruling I did not make — now closed

**`REQ-MCP-10`'s `verifymethod`.** Previously flagged because `rmcp` was absent from the cache.

That premise was false (item 15). `rmcp` is available, so **Option A stands**: `REQ-MCP-10` keeps
`verifymethod: test`. Confirmed unchanged in `requirement-diagram.md:159` and `:562`.

**But the sentence justifying it in `requirements.md:82` now cites the wrong version** (`0.8.5`
against a declared `3.5.0`) — see P4. The ruling is right; its recorded reason needs updating.

The reasoning that motivated the ruling stands as guidance: a requirement marked `test` that cannot
be tested is a small lie, and this project has been bitten by documentation claiming things the
code does not do. What must not happen is leaving a claim silently wrong with no note.

---

## 11. Re-verification method

Every verdict above rests on a command run on the macOS host on 2026-10-05. No Lima, no MCP
`run_shell_command` / `lima_*`, no network. Archive measurements were taken with
`python3` + `zipfile` against
`/Users/p2p/scm/local/tmp/bugreport-insight/bugreport-sdk_gcar_arm64-AAG6.260127.001-2026-09-24-00-27-17.zip`,
read-only; the archive was not modified.

### Not verified

| Item | Gap |
| --- | --- |
| **18** | `zip/chrono` offline build not proven — see the lead in §5 |
| **5, 16** | `grep-*` crates confirmed **present and vendored**; not **compiled**. `cargo build` against them has not run |
| **15** | Cache **presence** verified. Whether `rmcp 3.5.0` resolves is not — it has not been fetched |
| — | The brief's `1,243,790` and p50/p99 `83/325` are **unreproduced** by my measurement. See P1 |