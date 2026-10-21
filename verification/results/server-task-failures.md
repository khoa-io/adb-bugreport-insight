# SDK server-task failure verification

Executed **2026-10-19** on `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing workspace changes and this `server.rs` lifecycle correction.
Environment: macOS `aarch64-apple-darwin`, Rust/Cargo 1.99.0, offline dependencies.
Scope: REQ-MCP-10, REQ-CLI-03/05 and ARCH-MCP-SRV under [ADR 0014](../../docs/decisions/0014-transport-failures-and-process-exits.md).
This extends the [earlier transport-exit evidence](transport-exits.md) with actual SDK task failures rather than only injecting a constructed ServeError at CLI exit mapping.

## Finding and correction

Cached rmcp 3.5.0 source inspection established that RunningService::waiting returns Result<QuitReason, JoinError>.
An outer service task panic produces Err(JoinError), while a failed nested send task makes the service loop terminate with Ok(QuitReason::JoinError).
The former implementation treated every Ok termination reason as success, incorrectly logging clean shutdown and returning Ok for the nested failure.

Two test-only transports exercise these distinct SDK paths through actual rmcp services started with serve_directly.
One panics during receive in the SDK service loop; the other keeps receive pending and panics in the SDK send task after a tool-list-change notification.
serve_directly skips initialization only, allowing deterministic fault injection without production flags, environment variables or fault hooks.
The production lifecycle now shares a private supervisor with these tests; it accepts Closed/Cancelled, propagates both task-failure forms as ServeError::ServerTask, and rejects unknown future SDK termination reasons.
Archive admission, initialization, stdout monitoring and request semantics remain unchanged.

## Executed checks

| Check | Expected behavior | Actual result |
| --- | --- | --- |
| Before correction: `cargo test --offline -p bugreport-insight-mcp actual_sdk -- --nocapture` | Both real SDK failure forms reach ServerTask | One passed, one failed: nested send-task panic incorrectly returned Ok(()) and logged clean shutdown |
| After correction: `cargo test --offline -p bugreport-insight-mcp actual_sdk` | Both failure forms reach ServerTask and retain the injected failure description | Pass, 2 tests |
| CLI `server_task_failure_exits_nonzero` | ServerTask maps to process exit 1 with a server-task diagnostic | Covered by workspace run; existing unit measure joins the real SDK propagation evidence above |

Fault tests are `server::tests::actual_sdk_service_task_panic_is_a_server_failure` and `server::tests::actual_sdk_send_task_panic_is_a_server_failure` in [server.rs](../../crates/mcp/src/server.rs).
They assert both the typed ServerTask result and the original injected failure text.
The nested test also verifies that the SDK notification caller observes its failed send.
The expected task panics are caught by SDK/Tokio join handling and do not fail the corrected tests.

## Regression gates and remaining coverage

| Command | Expected result | Actual result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Clean workspace formatting | Pass |
| `cargo build --workspace --offline` | All crates build | Pass |
| `cargo test --workspace --offline` | Lifecycle correction preserves all workspace behavior | Pass, 200 tests, zero failures/ignored tests; includes 39 real-process protocol tests, 9 MCP unit tests and private reference suites |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | No warnings across workspace targets | Pass after moving the test module to the file end and constructing the test send future without borrowing its transport |
| `python3 scripts/check_docs.py` | Current Markdown links and structure validate | Pass, 190 pages and 2,225 local links |

The two focused SDK fault tests were rerun after the test-only Clippy repairs and both passed.
No diagram sources or allocation edges changed in this lifecycle correction, so diagram rendering was unnecessary.
Native non-Unix broken-stdout behavior remains unverified and is not established by these portable fault tests.
The current cfg(not(unix)) monitor merely waits for shutdown and stdout_is_disconnected returns false; no independent non-Unix pipe-disconnect detector is implemented.
The SDK can swallow failed response writes, so native testing and an appropriate platform-specific implementation remain necessary before claiming the broken-stdout contract on non-Unix targets.
Existing Unix real-process protocol checks cover ordinary EOF before/after initialization, broken stdout, stage-appropriate failures and request recovery.
These tests prove real SDK task propagation into the shared supervisor plus existing CLI exit mapping; they do not trigger a fault through the released executable's normal stdio transport.
