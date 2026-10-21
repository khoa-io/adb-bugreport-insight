# Fixtures and environments

## Synthetic fixtures

[crates/test-fixtures](../../crates/test-fixtures/src/lib.rs) is a development dependency of MCP and CLI suites, not shipped runtime functionality.
`ZipBuilder` writes minimal stored-entry ZIPs; initialize shared fixtures once to avoid concurrent rewrites.
Outputs live under `target/test-fixtures`.
The [core hostile suite](../../crates/core/tests/hostile.rs) has its own raw-byte builder for malformed headers, non-UTF-8 names, encryption flags, limits, and corrupt data.

The hostile categories cover long lines, bounded expansion, lying headers, encryption, NUL bytes, nested archives, raw names, corrupt/empty archives, and limit validation.
Categories do not prove every permutation or four-by-seven status cell is asserted.
Exact executable measures and their scope live in [SWE.4](../swe.4-unit-verification/01-verification-measures.md) and [SWE.5](../swe.5-component-and-integration-verification/index.md).

## Private reference archive

[reference_archive.rs](../../crates/core/tests/reference_archive.rs) contains 19 tests.
By default, place the approved fixture at `tmp/reference-archive.zip` in the repository root.
The default is resolved from the core crate's manifest directory.
`BUGREPORT_ARCHIVE` overrides it; a relative override resolves against the test process working directory.
Do not commit the archive.

```bash
cargo test -p bugreport-insight-core --test reference_archive --offline -- --nocapture
```

If the file is absent, tests print a skip explanation and return early.
Record fixture availability/skip output; Cargo counts hide unexercised assertions.
The identity test expects SHA-256 `10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92`.
Measured-value assertions require that fixture, not an unrelated archive.

### Reference measurements

These measurements identify the approved private fixture and provide design inputs:

| Property | Recorded value |
| --- | --- |
| Stored size | 14,710,126 bytes |
| Entries | 398; zero directories/encrypted/unreadable entries |
| Total uncompressed bytes | 126,631,335 |
| Main text entry | 64,645,191 bytes; 682,951 lines |
| Longest archive-wide line | 192,666 characters |
| Archive-wide newline count | 1,243,691 |
| Line length p50 / p99 | 83 / 325 |

Newline and split-fragment counts differ; 1,243,691 counts newlines.
These measurements informed reader design but do not prove all current contracts.
The [real-case source oracle](../../verification/results/real-case-user-validation.md) independently checks main-report line and NUL counts.

## Environment and cleanup

Run Cargo from the repository root using a Rust 2024 capable toolchain.
Offline runs require the locked dependency sources to be cached.
Run CLI package protocol tests against their Cargo-built binary, not an arbitrary executable.
Synthetic archives are scratch outputs; reference setup/teardown is the operator's responsibility.
Reference setup does not install the private input or imply that Cargo suites have executed.
