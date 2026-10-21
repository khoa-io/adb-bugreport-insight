# Transport exit behavior verification

Tested tree: `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus existing workspace edits and the TASK019 transport-exit changes.
Environment: macOS, Rust/Cargo 1.99.0, offline dependencies.
Scope: REQ-CLI-03/05, REQ-MCP-10, ARCH-MCP-SRV, and ADR 0014.

## Checks and results

| Check | Expected behavior | Result |
| --- | --- | --- |
| `stdin_eof_before_initialization_exits_zero` | Empty stdin during MCP initialization is a clean disconnect, exit 0 | Pass |
| `client_disconnect_exits_zero` | EOF after initialization exits 0 | Pass |
| `broken_stdout_after_initialization_exits_nonzero_with_transport_diagnostic` | Closed response pipe exits 1 with a transport diagnostic, not an archive-open diagnostic | Pass |
| `invalid_request_does_not_prevent_later_valid_request` | Unknown method receives an error, then initialize and tools/list succeed in the same process | Pass |
| `cargo test --offline -p bugreport-insight-cli --test protocol` | All real-process protocol regressions pass | Pass, 39 tests |
| `server_task_failure_exits_nonzero` | A typed server-task failure maps to process exit 1 | Pass |
| `cargo test --workspace --offline` | Workspace regressions pass | Pass, 198 tests |
| `cargo build --workspace --offline` | All workspace crates build | Pass |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | No warnings across workspace targets | Pass |
| `cargo fmt --all -- --check` | Workspace formatting is clean | Pass |

The broken-stdout test failed before implementation: the process exited 0 and logged
“stdin closed; shutting down” after the client dropped its stdout reader and
requested `tools/list`.
On Unix, `serve` now monitors stdout's poll state while rmcp serves requests so
the failed write is detected even when the service task itself completes
successfully.
Transport-start and service-task errors now remain distinct from archive-open
errors and propagate to the CLI's existing nonzero exit path.

The CLI exit mapping for `ServeError::ServerTask` is covered by an injected
typed-error unit test; an actual rmcp server-task panic/error is not
deterministically induced by the integration suite.
The non-Unix monitor is a no-op; broken-stdout detection is currently verified
and implemented only for Unix targets.

The Markdown-link checker was attempted but the runtime denied the command;
new/updated relative links were checked by direct path and anchor inspection.
Diagram freshness was not rechecked because no diagram sources or generated
diagrams changed.
