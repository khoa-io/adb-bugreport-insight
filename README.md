# bugreport-insight

Generate local insight artifacts from Android `adb bugreport` ZIP archives
without extracting their contents.

```console
cargo run -- ./tmp/bugreport.zip
```

The command writes `.bugreport/insight/` beside the input archive. Bugreports
often contain sensitive information; keep both archives and generated artifacts
local and review them before sharing.

## Focused subcommands

Beyond the default full report, `bugreport-insight <verb> <BUGREPORT_ZIP>`
dispatches to a `bugreport-insight-<verb>` binary (git/cargo-style plugin
dispatch) that prints a single topic's findings as JSON to stdout (or to
`--output FILE`):

```console
cargo run -- anr ./tmp/bugreport.zip          # Application Not Responding events
cargo run -- apps ./tmp/bugreport.zip         # installed/running apps: PID, package, data/install paths
cargo run -- stacktrace ./tmp/bugreport.zip   # native crashes/tombstones and Java stack traces
cargo run -- memory ./tmp/bugreport.zip       # memory pressure / low-memory-killer churn
cargo run -- cpu ./tmp/bugreport.zip          # CPU/I-O thrashing and lock contention
```

These built-in subcommands are separate lib+bin crates in this workspace: each
crate's `lib.rs` owns that verb's scanner (e.g. `bugreport-insight-anr` owns
`scan_anrs()`), and its `main.rs` is a thin binary wrapping it. This makes
every scanner independently reusable as a library — including by
`bugreport-insight`'s own default full report, which links all of them
alongside `bugreport-insight-core`. Third-party subcommands (for example a
`dumpstate` subcommand) work the same way: install a `bugreport-insight-<verb>`
executable on `PATH` — built against `bugreport-insight-core` directly (for
shared infra like `open_report`), and optionally against one or more
`bugreport-insight-<verb>` crates as libraries (to reuse an existing
scanner), or reusing `bugreport-insight-subcommand-support` for CLI/JSON-output
plumbing — and `bugreport-insight <verb> ...` will find and run it.
`bugreport-insight help <verb>` shows that subcommand's own `--help` text.
