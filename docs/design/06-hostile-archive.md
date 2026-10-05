# 06 — Hostile-archive behaviour

[`conventions-envelope.md`](../draft/artifact-schemas/conventions-envelope.md) records one hostile
archive. Assume every archive is hostile.

| Measured on the reference archive | Value |
| --- | --- |
| Main entry size | 64,645,191 bytes |
| Line count | 682,951 |
| Longest single line | **192,666 characters** |
| NUL bytes | **4**, at offsets 44,988,012–15 |

## 1. Decisions

| Threat | Decision |
| --- | --- |
| **Line bomb** | `MAX_LINE_BYTES = 1 MiB`. Exceeding it yields `ExcerptOutcome::LineTooLong`, `excerpt_truncated: true`, `reason: "line_too_long"`, and the reader **skips the remainder by counting newlines in a fixed 64 KiB scratch buffer** — it never buffers the tail. Line _N_ of a 682,951-line entry costs **O(1) memory**. |
| **NUL bytes** | Text output is **lossy UTF-8 with U+FFFD**. Never reject the read. Any result touching a NUL span carries `notes: ["CONTAINS_NUL"]`. A raw `\u0000` never reaches a JSON string. |
| **Invalid UTF-8** | Same treatment, plus `decoding: "utf8_lossy"` in the excerpt so the agent knows the bytes did not round-trip. |
| **Non-UTF-8 entry names** | Paths decoded lossily for display; `path_bytes_valid_utf8` and `path_bytes_len` recorded from `name_raw()`. Such an entry is still listable. |
| **Path-addressed tools** | **Every tool addresses entries by `entry_id`, never by path.** Paths are *output*. This removes path traversal, encoding mismatches, duplicate names, and glob-vs-literal ambiguity in one move. |
| **Lying header / zip bomb** | `MAX_ENTRY_BYTES` is checked against the *declared* size, which an attacker controls. Every read path additionally goes through `.take(MAX_ENTRY_BYTES + 1)` and treats overflow as `RatioExceeded`. `MAX_COMPRESSION_RATIO = 1_000`. |
| **Central directory lies** | 10,000 entries declared, 4 GB appended: at open, assert `file_size() <= MAX_ARCHIVE_FILE_BYTES` (2 GiB) and record `declared_entry_count` separately from what was walked. |
| **Encrypted entry** | `EntryEncrypted`, surfaced as `Unreadable`. **Never** as an empty result. |
| **Nested archives** | `entry_info` reports nested status via magic bytes on the first 8 KiB of **decompressed** data (`PK\x03\x04`, `\x1f\x8b`, `BZh`, `\xFD7zXZ`, `7z\xBC\xAF`). **v1 detects and reports; it does not open.** Opening belongs to `DRAFT-REQ-GEN-03`. |
| **`version.txt`** | Read verbatim, stripped, **capped at 256 bytes**, exposed as `format_version: String | null`. **Never parsed into a number, never branched on** — matching [`conventions-envelope.md`](../draft/artifact-schemas/conventions-envelope.md) §2.1.2. Absent gives `null` plus `notes: ["ARCHIVE_FORMAT_0"]`. |
| **`main_entry.txt`** | Read verbatim, capped the same way. Never interpreted. |

## 2. The line bomb is the design driver

A 192,666-character line is not large. The general case is: an archive can declare a 256 MiB
single line. Any implementation that does `lines().nth(n)` or `read_line()` into a `String` will
allocate it.

**The requirement is that reading line _N_ of an entry costs O(1) memory**, regardless of how
long that line is. Two mechanisms together:

1. `LineIndex` holds `(byte_offset, line_no)` **checkpoints** every _N_ lines, capped at
   `max_checkpoints`. Reading line _N_ seeks to the nearest checkpoint below it and walks
   forward — bounded work, bounded memory.
2. While walking, `LineTooLong` triggers once `MAX_LINE_BYTES` is exceeded. The remainder is
   **skipped** by counting `\n` in a reusable 64 KiB scratch buffer, and `byte_len_full`
   records the true length even though `bytes` is short.

This is the difference between a tool that survives the reference archive and one that does not.

## 3. Why `find_by_type` reports a sample, not a verdict

`REQ-MCP-ARC-07` requires detection "reported with its basis and never as a bare confident
label". `DetectionReport` therefore carries:

- `sample_bytes` — how much was actually inspected, always 8 KiB unless the entry is smaller.
- `basis` — `MagicBytes`, `ZipEntryName`, `UniformExtension`, or `None`.
- The rule: **`ContentType::Unknown` is legal only with `basis: None`**, and a specific type is
  claimed only when metadata or byte content supports it.

A classification of a 256 MiB entry from an 8 KiB sample is a **sample**, and the tool says so
rather than implying it read the whole entry.

## 4. `mtime` — raw MS-DOS fields

`zip`'s `DateTime` accessors sit behind its optional `time` dependency, which is **absent from
the offline cache**. With `default-features = false, features = ["deflate"]` no calendar
timestamp can be rendered.

**Decision: `EntryMeta.modified` carries `ModifiedStamp { dos_date: u16, dos_time: u16 }`** —
the raw packed fields verbatim — plus `modified_valid: bool`.

**This is more correct, not less.** MS-DOS timestamps have 2-second granularity and no timezone.
Any rendered value would be false precision. The raw fields let the server — or a later artifact
— interpret them with an explicit basis, and `modified_valid: false` distinguishes "absent or
unparseable" from a real 1980-01-01 stamp.

When the network grant lands: add `zip/time` and a derived `modified_iso: Option<String>`
**alongside**, never replacing the raw fields. Recorded as issue 18.

## 5. Two claims in `architecture.md` that are not merely stale

- `architecture.md:53,62,96` claims `core` depends on **`ripgrep`**. `ripgrep` is a **binary**,
  not a library. A library crate cannot depend on it. The claim is impossible, not outdated.
- `architecture.md:53` claims `core` depends on `tar`. No TAR support is committed, yet
  `REQ-MCP-ARC-02` promises "ZIP **and** TAR entries". **v1 is ZIP-only**; "and TAR" is removed
  and returns with the draft extension work. Shipping a promise the build cannot honour is worse
  than shipping a smaller promise.

## 6. The ten fixtures

Engineer A, `crates/archive/tests/hostile.rs`. Each is a synthetically-built ZIP — no AOSP
dependency, no real archive required.

| # | Fixture | Asserts |
| --- | --- | --- |
| 1 | `line_bomb` — one 192,666-byte line | `LineTooLong` at `MAX_LINE_BYTES`; reading it does **not** allocate it |
| 2 | `nul_bytes` — 4 NULs at a known offset | U+FFFD substitution; `notes: ["CONTAINS_NUL"]` |
| 3 | `lying_header` — declares 1 KiB, actually 64 MiB deflate | `LimitExceeded` from the **streaming** guard, not the header |
| 4 | `too_many_entries` — 10,001 entries | `TooManyEntries` at open |
| 5 | `corrupt_deflate` — valid central directory, broken stream | `Unreadable`, **never** `empty` |
| 6 | `non_utf8_name` | listable; `path_bytes_valid_utf8: false`; addressable by `entry_id` |
| 7 | `encrypted_entry` | `EntryEncrypted`; `Unreadable`, never `empty` |
| 8 | `nested_gzip`, `nested_zip` | detected and reported; `open` is **not** offered |
| 9 | `no_version_txt` | `format_version: null` plus `ARCHIVE_FORMAT_0` |
| 10 | `zip_bomb_ratio` — 1000:1 exceeded | `RatioExceeded` |

**Fixture 5 is the important one.** It is the test that catches an implementation returning
`Empty` when it actually failed to read — the exact confusion `REQ-MCP-08` exists to prevent.

## 7. Verification appendix

The API facts this document relies on were established by probe, not by reading documentation.
Anyone who distrusts them can re-run the probes.

### The `zip` API

Probed against cached `zip 2.4.2` on a synthetic two-entry archive:

| Fact | Location | Probe result |
| --- | --- | --- |
| `ZipFile::data_start()` is public | `zip-2.4.2/src/read.rs:1776` | returned `43` and `103`, consistent with the local headers |
| `by_index()` returns a **borrowing guard** | `read.rs:1092` | two live borrows in one scope failed to compile: `E0499 cannot borrow as mutable more than once at a time` |
| `last_modified()` works without the `time` feature | `read.rs:1738` | returned `Some(DateTime::default())` |
| `CompressionMethodName::Unsupported(u16)` covers undecodable methods | `zip-2.4.2/src/compression.rs:16-52` | only `Stored` and `Deflated` exist as variants under `deflate`-only |
| `name_raw()` exposes raw path bytes | `read.rs:1550` | available |

**The `E0499` result is the load-bearing one.** It proves the two-pass design in
[`05` §3](./05-mcp-server-design.md#3-concurrency--no-lock-anywhere) is forced by the borrow
checker rather than merely preferred — which is why it is cheap to get right.

### `ring` for SHA-256

`sha2` is absent from the cache. `ring 0.17.14` is present with a complete dependency closure
(`cc`, `libc`, `untrusted`, `getrandom`, `cfg-if`) and builds offline.

```rust
ring::digest::digest(&ring::digest::SHA256, b"abc")
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
```

That matches the NIST published vector for SHA-256 of `"abc"` exactly. The implementation is
therefore correct, not merely present.

### Sandbox caveat

The shared `~/.cargo` is not writable inside the agent sandbox, and an unwritable `CARGO_HOME`
produces `failed to create directory ... Operation not permitted` — which **looks like a
missing dependency but is not**. Copy the registry to a writable path first:

```bash
mkdir -p "$TMPDIR/cargo-home"
cp -R "$HOME/.cargo/registry" "$TMPDIR/cargo-home/"
CARGO_HOME="$TMPDIR/cargo-home" cargo build --offline
```
