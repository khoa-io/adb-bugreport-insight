# Real-case user validation — 2026-10-19

## Scope and baseline

TASK020 implements the user's 2026-10-19 request to act as an AI agent, engineer and CI job using `tmp/reference-archive.zip`.
The reusable driver is [scripts/validate_reference_workflows.py](../../scripts/validate_reference_workflows.py).
It speaks JSON-lines MCP to the shipped executable and uses an isolated temporary copy for generated insight.
Real report text, names, identifiers and paths are not printed or committed.
The source archive and preexisting local insight remain untouched.

Baseline: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus the existing working-tree implementation of TASK015–TASK019, this validation driver and TASK021’s final server-task lifecycle correction in `crates/mcp/src/server.rs`.
Platform: Darwin arm64; Rust `1.99.0 (b940084d7 2026-09-28)`; Python 3.15.0.
The fixture is a local, untracked 14,710,126-byte ZIP; it is not redistributed.
A passing run requires a built binary and that fixture, and uses temporary storage for the isolated ZIP and seven generated artifacts.

## SWE process and acceptance selection

| Process | Selected activity and disposition |
| --- | --- |
| SWE.1 | Reuse committed REQ-GEN-01/02, REQ-MCP-02–12, REQ-MCP-ARC-01–07, REQ-CLI-03–05 and REQ-INSIGHT-01–05; validate their user workflows without promoting draft analysis tools or batch scope. |
| SWE.2 | Preserve ARCH-ARCHIVE, ARCH-MCP-SRV, ARCH-MCP-TOOLS-ARC, ARCH-CLI-FLAGS, ARCH-CLI-INSIGHT, ARCH-INSIGHT-ANALYZERS, ARCH-INSIGHT-WRITER and ARCH-MCP-RES; exercise their existing composed CLI boundary. |
| SWE.3 | Add only a standard-library Python acceptance driver, with request IDs, protocol-only stdout validation, bounded request waits, explicit cost-preview retries, orderly EOF, forced process cleanup on failure and owned temporary-output cleanup. |
| SWE.4 | No runtime unit changes originate from this task, so no new Rust unit tests are required; existing workspace gates are coordinated separately by TASK019/SWE.6-T1. |
| SWE.5 | Exercise real stdio discovery/routing, confirmation challenges, source locator follow-up, multi-page search, tool/resource errors and recovery, exact resource contents and lifecycle. |
| SWE.6 | Execute the three personas against the real reference archive, compare search locators to an independent streamed ZIP oracle, check SHA/mtime preservation and record actual outcomes below. |

This is workflow acceptance evidence, not an exhaustive hostile-input or parser qualification claim.
The [requirements](../../docs/swe.1-requirements/02-software-requirements.md), [architecture](../../docs/swe.2-architecture/02-components-and-dependencies.md), [MCP detailed design](../../docs/swe.3-detailed-design/03-mcp-server.md), [insight design](../../docs/swe.3-detailed-design/05-insight-generation.md) and [traceability matrix](../../docs/traceability.md) retain contract ownership.

## Measures

| Persona/check | Expected observable result | Requirements |
| --- | --- | --- |
| AI discovery and navigation | Initialize explains cost confirmation; exactly seven tools have object schemas; list/find return usable numeric entry IDs; all seven tools execute. | REQ-MCP-02/05/11, REQ-MCP-ARC-01–07 |
| AI request consent | Every content tool and every grep page first returns confirmation_required, cost and 300-second expiry; an explicit matching token retry executes it; index-only discovery executes immediately. | REQ-MCP-12 |
| AI bounded evidence | Lines and bytes are readable, emitted excerpts are at most 512 bytes, source_bytes supports follow-up, identical range reads are deterministic, and grep locators reproduce the same evidence. | REQ-MCP-03/04/06/09, REQ-MCP-ARC-04/05 |
| AI real triage | Search the complete main report for section markers, FATAL EXCEPTION and ANR in; each matching line number/byte offset agrees with independent streamed ZIP bytes and the terminal page reports complete coverage. | REQ-MCP-ARC-05, REQ-MCP-06/09 |
| AI errors and recovery | Invalid ranges, missing entry IDs and unsupported regex fail; a completed missing path search is empty; the same session subsequently answers a valid listing. | REQ-MCP-08, REQ-CLI-03 |
| Engineer insight | Generate exactly seven files, six JSON objects and a README of at most 400 lines; verify every serialized evidence excerpt is at most 512 UTF-8 bytes. | REQ-INSIGHT-01/03/05 |
| Engineer resource access | Missing insight lists empty after confirmation; generated resources use the archive SHA, all seven read back exact whole artifact bytes, a malformed URI fails and later listing recovers. | REQ-MCP-07/12 |
| CI collision and lifecycle | A second generation exits 1 and preserves every artifact SHA/mtime; help exits 0, bad usage 2, missing archive 1, normal EOF before/after initialize 0; diagnostics do not contaminate MCP or insight stdout. | REQ-CLI-03/04/05, REQ-MCP-10, REQ-INSIGHT-02 |
| CI immutability | Reference and isolated-copy SHA/mtime remain unchanged; resource reads preserve every output SHA/mtime. | REQ-GEN-01, REQ-INSIGHT-02 |

The driver reviews the challenge shape, cost presence and expiry, then explicitly accepts each operation as authorized by this autonomous acceptance run.
It does not add approval policy to production clients or treat confirmation as proof of human identity.
It finishes each search to release cursor capacity and bounds the number of continuation pages to 2,000 per query.
The raw ZIP oracle exists only inside this validation driver; the product workflow uses the shipped CLI/MCP surface throughout.

## Executed results

Initial complete workflow passed before expanding the independent triage oracle.
It observed 398 indexed entries, a 200-item listing, 31 text candidates, 94 section-marker search pages, 4,641 matching lines and 64,645,191 fully covered main-report source bytes.
The AI session used 105 explicit previews and 216 JSON-RPC requests.
Insight generation and seven exact resource reads passed, serving 7,754,559 bytes; collision refusal, SHA/mtime preservation and exit codes 0/1/2 passed.

Expanded triage/oracle execution passed: FATAL EXCEPTION yielded 274 matching lines over seven pages; ANR in yielded zero over three pages.
Every returned line number/byte offset agreed with the independent ZIP oracle, and both terminal scans reported complete main-entry coverage.
The AI session used 115 explicit previews and 236 JSON-RPC requests.
The generated crash ledger recorded 274 raw fatal hits, 271 matched records and three suppressed hits, accounting for the difference rather than equating raw text with incidents.
The generated structured records contained 271 Java crashes, 50 native crashes and zero ANR incidents; these are generated record counts, not an independent incident classification or causal diagnosis.
All 1,804 serialized evidence excerpts across the six JSON artifacts satisfied the 512-byte emitted UTF-8 limit.

An independent main-entry byte oracle counted 682,951 LF-delimited records, one record containing NUL and four total NUL bytes.
These definitions supersede the ambiguous historical 6,617/6,643 NUL-line figures for current main-report evidence; they make no archive-wide binary-content claim.

The final rerun against TASK021's rebuilt server-task correction passed with the same operational and triage counts.
The crash ledger assertion confirmed `274 = 271 + 3 + 0`, and every source/output SHA/mtime assertion passed.

| Command/check | Expected result | Actual result |
| --- | --- | --- |
| `cargo build --workspace --offline` | Build shipped executable | PASS; initial build executed by this task and final rebuilt binary supplied by TASK021's passing workspace gates. |
| `python3 scripts/validate_reference_workflows.py` | Complete all three personas against the reference ZIP | PASS; initial workflow, expanded independent triage workflow, then final driver against corrected server. |
| `python3 -m py_compile scripts/validate_reference_workflows.py` | Parse/compile reusable acceptance driver | PASS. |
| [Final workspace gates](server-task-failures.md) | Formatting, build, unit/integration suites and Clippy | PASS; coordinated separately, 200 tests, after the server lifecycle correction. |

The final AI session executed 236 JSON-RPC requests and 115 explicit content previews.
The generated-resource session added 11 previews, and the earlier absent-output resource listing added one; every expensive request had a matching explicit retry.
Seven exact resource reads served 7,754,559 bytes.
A second generation refused the existing output, and the workflow's repeat range read remained byte-for-byte deterministic.
Temporary generated artifacts were removed by owned-directory cleanup; the original reference and its existing local insight were preserved.

## Interpretation and limits

Literal log matches are investigation leads, not verified incident counts or causal diagnoses.
A zero-match complete query establishes absence of that literal in the selected main entry only; it does not establish absence of incidents elsewhere in the ZIP.
The independent oracle checks search completeness and source locators; it does not independently validate every analyzer interpretation, freshness decision or device fact.
REQ-GEN-02 hostile-input limits remain covered by the synthetic suites rather than by this benign real archive.
Full conformance, transport fault injection and platform coverage remain owned by [SWE.6-T1](software-qualification.md) and its current successor work.
EOF/session handling and correctness are exercised without timing/CPU/physical-IO performance promises.
