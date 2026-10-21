# Lazy archive validation — 2026-10-18

## Scope and environment

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus the existing uncommitted insight/resource work, operational-document migration, and these lazy-validation changes.
Environment: macOS; rustc 1.99.0 (b940084d7), Cargo 1.99.0 (5f94df478), cached dependencies and offline Cargo commands.
REQ-GEN-02/ARCH-ARCHIVE now separate metadata admission from produced-byte validation, following the user's explicit lazy-validation decision.
[ADR 0004](../../docs/decisions/0004-lazy-archive-validation.md), [requirement](../../docs/swe.1-requirements/02-software-requirements.md#req-gen-02), [design](../../docs/swe.3-detailed-design/04-archive-safety.md) and [traceability](../../docs/traceability.md#req-gen-02) describe the accepted contract.

## Findings and correction

The earlier chunk iterator reopened the decoder for every chunk, repeated the payload prefix and reported successful EOF when its ceiling was reached.
Initial regression execution failed three of five new tests: payload continuity, chunk overflow and a lying declared size through chunks.
The iterator now borrows EntryReader and retains one decoder; shared Cursor checks produced bytes before returning a chunk.
Line/chunk refills use a one-byte probe beyond the remaining ceiling to distinguish exact EOF from overflow.
Byte ranges check produced bytes while skipping prefixes and collecting output; line walks count skipped lines and discarded tails.
The prior hostile test called oversized_entry_is_caught_by_the_streaming_guard checked honest oversized metadata at open, not a lying stream.
It was renamed to declared_oversized_entry_is_rejected_before_payload_read; new fixtures exercise actual streamed overflow separately.

## Selected checks and results

[Lazy-validation checks](../../crates/core/tests/lazy_validation.rs) exercise six cases through the public core API:

| Check | Contract demonstrated | Result |
| --- | --- | --- |
| chunk_stream_preserves_every_byte_for_stored_and_deflated_entries | Non-repeated exact bytes across decoder chunks for both methods | Passed |
| chunk_stream_reports_overflow_instead_of_eof_and_accepts_exact_limit | Zero/small ceilings fail on excess; exact ceiling succeeds; error retains ArchiveError | Passed |
| lying_declared_size_is_accepted_at_open_but_rejected_when_read | Understated size admitted as metadata, actual output rejected by line/span/sequential/chunk paths | Passed |
| skipped_bytes_and_lines_count_toward_the_read_ceiling | Prefix-discard and returned bytes both count | Passed |
| declared_total_and_stored_file_limits_accept_boundary_and_reject_above | Small coherent declared aggregate and stored-file limits accept exact values and reject excess | Passed |
| stopping_after_first_line_does_not_validate_corrupt_unread_tail | Early line/callback success does not certify the corrupt suffix; full chunk read fails | Passed |

`cargo test -p bugreport-insight-core --test lazy_validation --test hostile --offline`: 26 passed.
Existing hostile cases also cover oversized declared entry/count, invalid limits, corrupt/encrypted entries and fixed retained buffers.
Regression selection includes full workspace checks because core readers serve MCP and static insight consumers.

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | Passed |
| cargo build --workspace --offline | Passed |
| cargo test --workspace --offline | Passed: 153 tests, including private reference suites |
| cargo clippy --workspace --all-targets --offline -- -D warnings | Passed |
| python3 scripts/check_docs.py | Passed |
| python3 scripts/diagrams.py check | Passed: nine fresh SVGs |
| Scoped `git diff --check` for changed source and documentation | Passed for affected areas |

## Evidence limits

The targeted fixtures use small custom limits to exercise branches without allocating a hostile 256 MiB entry.
They do not demonstrate every production-sized boundary, peak process memory, CPU bounds or all adversarial ZIP combinations.
The 1 GiB aggregate is deliberately declared-only; actual whole-archive expansion and cumulative session work are not measured.
Unread entries/tails remain unverified, including CRC/readability.
EntryChunks now carries a reader-borrow lifetime instead of owning its archive; workspace callers compile against the revised API.
This closes the original eager-versus-lazy policy conflict and corrects the audited chunk path; other discrepancies and full SWE.6-T1 qualification remain open.

The unrestricted workspace whitespace check also reports trailing whitespace in unrelated working edits to an unrelated agent instruction file; those edits were preserved.
