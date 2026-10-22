//! Integration verification over **real stdio** — `WE.5-T1`.
//!
//! These tests spawn the actual `bugreport-insight` binary and speak JSON-RPC to
//! it over a pipe. Nothing is mocked, so they cover what the `ToolHandler` seam
//! cannot: the transport, the adapter's error mapping, `tools/list` schemas, and
//! stdout purity.
//!
//! **Why this lives in `crates/cli/tests/` and not `crates/mcp/tests/`:** cargo
//! only builds a package's own binaries before running *its* integration tests.
//! With the file under `crates/mcp/tests/`, `cargo test -p bugreport-insight-mcp`
//! ran against whatever `target/debug/bugreport-insight` happened to be left over,
//! so a stale binary produced failures that looked like code defects. Here the
//! binary under test is always the one just built.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

/// The committed tool set, asserted against `tools/list` over the wire.
///
/// Hard-coded here rather than imported so this suite stays a genuine
/// black-box check: it asserts the *published* contract, not whatever the
/// library currently happens to export.
const COMMITTED_TOOLS: [&str; 7] = [
    "archive_info",
    "list_entries",
    "find_entries",
    "entry_info",
    "find_by_type",
    "read_entry",
    "grep_archive",
];

// ------------------------------------------------------------------ harness

fn bin_path() -> PathBuf {
    // Set by cargo for integration tests of the package that defines the binary.
    PathBuf::from(env!("CARGO_BIN_EXE_bugreport-insight"))
}

struct Server {
    child: Child,
    reader: Option<BufReader<std::process::ChildStdout>>,
    next_id: i64,
}

impl Server {
    fn start(archive: &Path) -> Self {
        Self::start_with_args(archive, &["--mcp"])
    }

    fn start_with_args(archive: &Path, extra: &[&str]) -> Self {
        let mut cmd = Command::new(bin_path());
        cmd.arg(archive);
        cmd.args(extra);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn bugreport-insight");
        let reader = BufReader::new(child.stdout.take().expect("stdout is piped"));
        Self {
            child,
            reader: Some(reader),
            next_id: 0,
        }
    }

    fn start_insight(path: &Path) -> Self {
        let mut child = Command::new(bin_path())
            .args(["--mcp", "--insight"])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn combined insight mode");
        let reader = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            reader: Some(reader),
            next_id: 0,
        }
    }

    fn send(&mut self, msg: &Value) {
        let stdin = self.child.stdin.as_mut().expect("stdin is piped");
        writeln!(stdin, "{msg}").expect("write request");
        stdin.flush().expect("flush");
    }

    fn notify(&mut self, msg: &Value) {
        self.send(msg);
    }

    // Behavior tests explicitly accept a preview; confirmation tests use request_once.
    fn request(&mut self, msg: &Value) -> Value {
        let frame = self.request_once(msg);
        let challenge = if frame["error"]["data"]["code"] == "confirmation_required" {
            Some(frame["error"]["data"].clone())
        } else if frame["result"]["isError"] == true {
            frame["result"]["content"][0]["text"]
                .as_str()
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
                .filter(|body| body["error"]["code"] == "confirmation_required")
                .map(|body| body["error"]["data"].clone())
        } else {
            None
        };
        if let Some(challenge) = challenge {
            let mut accepted = msg.clone();
            self.next_id += 1;
            accepted["id"] = json!(self.next_id + 10000);
            if msg["method"] == "tools/call" {
                accepted["params"]["arguments"]["confirmation"] =
                    challenge["confirmation_token"].clone();
            } else {
                if accepted["params"].is_null() {
                    accepted["params"] = json!({});
                }
                accepted["params"]["_meta"] =
                    json!({"bugreport-insight/confirmation":challenge["confirmation_token"]});
            }
            self.request_once(&accepted)
        } else {
            frame
        }
    }

    /// Sends one request without automatically accepting its cost preview.
    fn request_once(&mut self, msg: &Value) -> Value {
        let id = msg["id"].clone();
        self.send(msg);
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .as_mut()
                .expect("stdout reader is open")
                .read_line(&mut line)
                .expect("read response");
            assert!(n > 0, "server closed stdout before answering id {id}");
            let frame: Value =
                serde_json::from_str(line.trim()).expect("every stdout line is one JSON frame");
            if frame["id"] == id {
                return frame;
            }
        }
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        self.request(&req)
    }

    /// The decoded envelope inside a `tools/call` result.
    fn envelope(&mut self, name: &str, arguments: Value) -> Value {
        let frame = self.call_tool(name, arguments);
        assert!(
            frame.get("error").is_none(),
            "{name} returned a protocol error: {}",
            frame["error"]
        );
        let result = &frame["result"];
        assert_ne!(
            result["isError"],
            json!(true),
            "{name} returned a tool error: {frame}"
        );
        let text = result["content"][0]["text"]
            .as_str()
            .expect("content[0] is text")
            .to_string();
        serde_json::from_str(&text).expect("envelope is JSON")
    }

    fn initialize(&mut self) -> Value {
        self.initialize_version("2025-06-18")
    }

    fn initialize_version(&mut self, version: &str) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let frame = self.request(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": version,
                "capabilities": {},
                "clientInfo": { "name": "integration-test", "version": "0.0.0" }
            }
        }));
        self.notify(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        frame
    }

    fn list_tools(&mut self) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.request(&json!({ "jsonrpc": "2.0", "id": id, "method": "tools/list" }))
    }

    /// Closes stdin, which is how a real client disconnects.
    fn finish(mut self) -> std::process::Output {
        drop(self.child.stdin.take());
        self.child.wait_with_output().expect("wait for server")
    }
}

/// Writes the whole script to stdin, closes it, and returns the output
/// untouched — no frame is read back, so stdout can be inspected in full.
fn run_scripted(archive: &Path, script: &[Value], extra: &[&str]) -> std::process::Output {
    let mut child = Command::new(bin_path())
        .arg(archive)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bugreport-insight");
    {
        let stdin = child.stdin.as_mut().expect("stdin is piped");
        for msg in script {
            writeln!(stdin, "{msg}").expect("write request");
        }
        stdin.flush().expect("flush");
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("wait for server")
}

// ----------------------------------------------------------------- fixtures

// Shared with `crates/mcp/tests/tools.rs` so there is exactly one ZIP writer in
// the tree; a second hand-rolled copy is how the central-directory header went
// missing two bytes once already.
use bugreport_insight_test_fixtures::{ZipBuilder, good_archive, not_a_zip};

// -------------------------------------------------------------------- tests

#[test]
fn initialize_succeeds_over_real_stdio() {
    let mut server = Server::start(&good_archive());
    let frame = server.initialize();
    let result = &frame["result"];
    assert_eq!(result["serverInfo"]["name"], json!("bugreport-insight"));
    assert!(result["protocolVersion"].is_string());
    server.finish();
}

/// Only the `tools` capability is advertised. No `prompts`, no `resources`,
/// no `logging` — advertising a capability with no handler is a lie.
#[test]
fn tools_and_read_only_resources_capabilities_are_advertised() {
    let mut server = Server::start(&good_archive());
    let frame = server.initialize();
    let caps = &frame["result"]["capabilities"];
    assert!(
        caps.get("tools").is_some(),
        "tools capability must be advertised"
    );
    assert!(caps.get("resources").is_some());
    assert!(caps["resources"].get("subscribe").is_none());
    assert!(caps["resources"].get("listChanged").is_none());
    for absent in ["prompts", "logging"] {
        assert!(
            caps.get(absent).is_none(),
            "{absent} must not be advertised: it has no handler"
        );
    }
    server.finish();
}

/// `REQ-MCP-11`: `initialize` carries `instructions` naming each tool, its cost
/// class, and the intended call order.
#[test]
fn instructions_name_every_tool_and_stay_short() {
    let mut server = Server::start(&good_archive());
    let frame = server.initialize();
    let instructions = frame["result"]["instructions"]
        .as_str()
        .expect("instructions must be present")
        .to_string();
    assert!(!instructions.is_empty(), "instructions must not be empty");
    assert!(
        instructions.lines().count() <= 40,
        "REQ-MCP-11 caps instructions at 40 lines, got {}",
        instructions.lines().count()
    );
    for tool in COMMITTED_TOOLS {
        assert!(
            instructions.contains(tool),
            "instructions must name `{tool}`"
        );
    }
    server.finish();
}

/// `REQ-MCP-05`: `tools/list` returns exactly the committed set.
#[test]
fn tools_list_returns_exactly_the_seven_committed_tools() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.list_tools();
    let tools = frame["result"]["tools"].as_array().expect("tools array");
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = COMMITTED_TOOLS.to_vec();
    expected.sort_unstable();
    assert_eq!(names, expected);
    server.finish();
}

/// `REQ-MCP-05`: every tool has a description and a schema with
/// `additionalProperties: false`, so a misspelled parameter is rejected.
#[test]
fn every_tool_declares_a_closed_schema() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.list_tools();
    for tool in frame["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(!description.is_empty(), "{name}: empty description");

        let schema = &tool["inputSchema"];
        assert_eq!(
            schema["type"],
            json!("object"),
            "{name}: MCP requires inputSchema root type 'object'"
        );
        assert_eq!(
            schema["additionalProperties"],
            json!(false),
            "{name}: schema must reject unknown fields"
        );
    }
    server.finish();
}

#[test]
fn all_seven_tools_answer_over_stdio() {
    let mut server = Server::start(&good_archive());
    server.initialize();

    let cases: [(&str, Value); 7] = [
        ("archive_info", json!({})),
        ("list_entries", json!({})),
        ("find_entries", json!({ "pattern": "*" })),
        ("entry_info", json!({ "entry_id": 0 })),
        ("find_by_type", json!({ "content_type": "text" })),
        (
            "read_entry",
            json!({ "entry_id": 1, "line_start": 1, "line_end": 1 }),
        ),
        (
            "grep_archive",
            json!({ "pattern": "second", "mode": "literal" }),
        ),
    ];
    for (tool, args) in cases {
        let env = server.envelope(tool, args);
        assert_success_bound_fields(&env);
        assert_eq!(
            env["status"],
            json!("ok"),
            "{tool} over stdio: unexpected envelope {env}"
        );
    }
    server.finish();
}

/// stdout carries JSON-RPC frames and nothing else — a stray `println!` anywhere
/// in the tree would put a non-JSON line here.
///
/// The script is fed in one shot and **nothing is read back through `request`**,
/// so every byte the server writes is captured and inspected. Reading through
/// `request` would consume the frames and leave nothing to check.
#[test]
fn stdout_carries_json_rpc_and_nothing_else() {
    let script = [
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "purity", "version": "0.0.0" }
            }
        }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "archive_info", "arguments": {} }
        }),
    ];
    let output = run_scripted(&good_archive(), &script, &["--mcp", "--verbose"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");

    let mut frames = 0;
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let frame: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("non-JSON line on stdout ({e}): {line}"));
        assert_eq!(
            frame["jsonrpc"],
            json!("2.0"),
            "every stdout line must be a JSON-RPC 2.0 frame: {line}"
        );
        frames += 1;
    }
    assert!(frames >= 3, "expected response frames, saw {frames}");

    // `--verbose` was on: the extra detail went to stderr, never stdout.
    assert!(
        !output.stderr.is_empty(),
        "--verbose diagnostics must go to stderr"
    );
}

/// EOF on stdin is a clean disconnect: exit 0, not a false failure.
#[test]
fn stdin_eof_before_initialization_exits_zero() {
    let output = run_scripted(&good_archive(), &[], &["--mcp"]);
    assert!(
        output.status.success(),
        "EOF before initialization must exit 0; stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// EOF after initialization is a clean disconnect: exit 0, not a false failure.
#[test]
fn client_disconnect_exits_zero() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let output = server.finish();
    assert!(
        output.status.success(),
        "EOF on stdin must exit 0; stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Broken stdout must fail the process with a transport diagnostic.
///
/// The response-pipe monitor is Unix-only: it watches stdout for a closed
/// pipe, which is how a broken response channel is detected here. The
/// non-Unix monitor does not detect response-pipe closure, so that native
/// behaviour stays an open obligation rather than an assertion
/// (`docs/verification/03-open-qualification.md#transport-exits`).
#[cfg(unix)]
#[test]
fn broken_stdout_after_initialization_exits_nonzero_with_transport_diagnostic() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    drop(server.reader.take());
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }));

    let output = server.finish();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "broken stdout must fail the process; stderr was: {stderr}"
    );
    assert!(
        stderr.contains("transport") || stderr.contains("stdio"),
        "expected a transport diagnostic; stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("cannot open archive"),
        "transport failure must not be reported as an archive-open failure: {stderr}"
    );
}

#[test]
fn invalid_request_does_not_prevent_later_valid_request() {
    let script = [
        json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"integration-test","version":"0.0.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":1,"method":"unknown/method"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
    ];
    let output = run_scripted(&good_archive(), &script, &["--mcp"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let frames = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("JSON-RPC frame"))
        .collect::<Vec<_>>();

    assert!(
        frames
            .iter()
            .any(|frame| frame["id"] == 1 && frame["error"].is_object()),
        "invalid request should receive an error response; stdout was: {stdout}"
    );
    assert!(
        frames
            .iter()
            .any(|frame| frame["id"] == 2 && frame["result"].is_object()),
        "initialization should succeed; stdout was: {stdout}"
    );
    assert!(
        frames
            .iter()
            .any(|frame| frame["id"] == 3 && frame["result"]["tools"].is_array()),
        "tools/list should succeed afterward; stdout was: {stdout}"
    );
    assert!(
        output.status.success(),
        "request errors must not fail the session; stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_success_bound_fields(body: &Value) {
    assert!(body["truncated"].is_boolean());
    assert!(body.get("stopped_reason").is_some());
    assert!(body["stopped_reason"].is_null() || body["stopped_reason"].is_string());
    assert!(body["omitted_count"].as_u64().is_some());
    assert!(
        body["notes"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
    assert!(body.get("error").is_none());
}

#[test]
fn partial_success_retains_bounding_fields_without_becoming_an_error() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let env = server.envelope("list_entries", json!({ "limit": 1 }));
    assert_success_bound_fields(&env);
    assert_eq!(env["status"], json!("ok"));
    assert_eq!(env["truncated"], json!(true));
    assert_eq!(env["stopped_reason"], json!("item_cap"));
    assert_eq!(env["omitted_count"], json!(2));
    assert_eq!(env["data"]["entries"].as_array().unwrap().len(), 1);
    assert!(server.finish().status.success());
}

/// Domain errors have retry context, without successful-result completeness claims.
#[test]
fn domain_failure_is_a_tool_error_not_a_protocol_error() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    for (tool, arguments, code, id, status) in [
        (
            "entry_info",
            json!({ "entry_id": 4242 }),
            "entry_not_found",
            4242,
            "not_found",
        ),
        (
            "read_entry",
            json!({ "entry_id": 4242, "line_start": 1, "line_end": 1 }),
            "entry_not_found",
            4242,
            "not_found",
        ),
    ] {
        let frame = server.call_tool(tool, arguments);
        assert!(
            frame.get("error").is_none(),
            "domain failure became a protocol error"
        );
        assert_eq!(frame["result"]["isError"], json!(true));
        let body: Value =
            serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap())
                .expect("error body is JSON");
        assert_eq!(body["error"]["code"], json!(code));
        assert!(body["error"]["message"].is_string());
        assert_eq!(body["error"]["data"]["entry_id"], json!(id));
        assert_eq!(body["error"]["data"]["status"], json!(status));
        for field in [
            "status",
            "data",
            "truncated",
            "stopped_reason",
            "omitted_count",
            "notes",
        ] {
            assert!(
                body.get(field).is_none(),
                "failure must not mimic successful {field}"
            );
        }
        let success = server.envelope("archive_info", json!({ "with_sha256": false }));
        assert_success_bound_fields(&success);
        assert_eq!(success["data"]["archive"]["entry_count"], json!(3));
    }
    assert!(server.finish().status.success());
}

/// An unknown field is rejected before the handler runs.
///
/// The SDK's parameter wrapper reports typed deserialization failures as
/// caller-visible tool errors with a reason. The handler's protocol-error path
/// is verified separately below; both shapes follow the current MCP error contract.
#[test]
fn unknown_parameter_is_rejected_with_a_reason() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.call_tool("entry_info", json!({ "entry_id": 0, "bogus": 1 }));
    assert_eq!(
        frame["result"]["isError"],
        json!(true),
        "an unknown field must not succeed: {frame}"
    );
    assert!(
        frame.get("error").is_none(),
        "SDK parameter failure is a tool error here"
    );
    let text = frame["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("bogus"),
        "the caller must learn which field was wrong, got: {text}"
    );
    server.finish();
}

/// The protocol-error path is reachable over the wire: a call that names both
/// range pairs cannot be routed at all, so it is `-32602`.
#[test]
fn both_range_pairs_is_a_protocol_error() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.call_tool(
        "read_entry",
        json!({
            "entry_id": 1,
            "line_start": 1, "line_end": 1,
            "byte_start": 0, "byte_end": 4
        }),
    );
    assert!(
        frame.get("result").is_none(),
        "protocol failure must not carry success data"
    );
    for field in ["truncated", "stopped_reason", "omitted_count", "notes"] {
        assert!(frame["error"].get(field).is_none());
    }
    assert_eq!(
        frame["error"]["code"],
        json!(-32602),
        "expected invalid_params"
    );
    assert_eq!(
        frame["error"]["data"]["code"],
        json!("conflicting_range_params")
    );
    server.finish();
}

/// `regex` is a draft mode and is refused, never silently downgraded to literal.
#[test]
fn an_unavailable_pattern_mode_is_refused_over_the_wire() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.call_tool("grep_archive", json!({ "pattern": ".*", "mode": "regex" }));
    assert_eq!(frame["error"]["code"], json!(-32602));
    assert_eq!(
        frame["error"]["data"]["code"],
        json!("pattern_mode_unavailable")
    );
    server.finish();
}

/// An unknown tool name is a method error, not a silent empty success.
#[test]
fn unknown_tool_is_rejected() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.call_tool("no_such_tool", json!({}));
    assert!(
        frame["error"].is_object() || frame["result"]["isError"] == json!(true),
        "an unknown tool must not succeed: {frame}"
    );
    server.finish();
}

/// `REQ-MCP-04`: a full sweep of every tool leaves the archive byte-identical.
#[test]
fn a_full_tool_sweep_leaves_the_archive_unchanged() {
    let path = good_archive();
    let before = std::fs::read(&path).expect("read archive");

    let mut server = Server::start(&path);
    server.initialize();
    let _ = server.envelope("archive_info", json!({}));
    let _ = server.envelope("list_entries", json!({}));
    let _ = server.envelope("find_entries", json!({ "pattern": "*" }));
    let _ = server.envelope("entry_info", json!({ "entry_id": 1 }));
    let _ = server.envelope("find_by_type", json!({ "content_type": "text" }));
    let _ = server.envelope(
        "read_entry",
        json!({ "entry_id": 1, "line_start": 1, "line_end": 2 }),
    );
    let _ = server.envelope("grep_archive", json!({ "pattern": "e", "mode": "literal" }));
    server.finish();

    let after = std::fs::read(&path).expect("re-read archive");
    assert_eq!(before, after, "REQ-MCP-04: the archive was modified");
}

/// The archive is opened before the transport starts, so a bad archive is a
/// non-zero exit with a stderr diagnostic and **never** an MCP response. This
/// is the failure `REQ-MCP-08` exists to prevent.
#[test]
fn an_unopenable_archive_never_starts_a_server() {
    let server = Server::start_with_args(&not_a_zip(), &["--mcp"]);
    let output = server.finish();
    assert!(
        !output.status.success(),
        "a non-ZIP archive must exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "the stdio loop must never start: stdout was {stdout}"
    );
    assert!(!output.stderr.is_empty(), "a diagnostic must go to stderr");
}

/// `--mcp` with no archive is a usage error, not a silent no-op.
#[test]
fn missing_archive_is_a_usage_error() {
    let output = Command::new(bin_path())
        .arg("--mcp")
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "usage errors go to stderr");
}

#[test]
fn help_goes_to_stdout_and_exits_zero() {
    let output = Command::new(bin_path())
        .arg("--help")
        .output()
        .expect("run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "--mcp",
        "--insight <BUGREPORT_OR_DIRECTORY>",
        "--force",
        "--verbose",
        "--help",
        "bugreport-insight",
    ] {
        assert!(
            stdout.contains(expected),
            "--help must mention `{expected}`, got: {stdout}"
        );
    }
}

#[test]
fn insight_writes_the_fixed_artifact_set_and_preserves_the_archive() {
    let base = std::env::temp_dir().join(format!("bugreport-insight-{}", std::process::id()));
    std::fs::create_dir_all(&base).expect("create test directory");
    let archive = base.join("sample.zip");
    let bytes = ZipBuilder::new()
        .add("version.txt", b"2.0")
        .add("main_entry.txt", b"report.txt")
        .add("report.txt", b"== dumpstate: sample ==\n------ SYSTEM LOG ------\nline\n------ END SYSTEM LOG ------\n")
        .finish();
    std::fs::write(&archive, &bytes).expect("write test archive");

    let output = Command::new(bin_path())
        .arg("--insight")
        .arg(&archive)
        .output()
        .expect("run insight");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "analysis payload is written to artifacts"
    );
    assert_eq!(std::fs::read(&archive).expect("read archive"), bytes);
    let directory = base.join(".sample/insight");
    let mut names = std::fs::read_dir(&directory)
        .expect("list artifacts")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "README.md",
            "anr.json",
            "apps.json",
            "cpu.json",
            "crashes.json",
            "memory.json",
            "system.json"
        ]
    );
    for name in [
        "system.json",
        "apps.json",
        "anr.json",
        "crashes.json",
        "memory.json",
        "cpu.json",
    ] {
        let value: Value = serde_json::from_slice(
            &std::fs::read(directory.join(name)).expect("read JSON artifact"),
        )
        .expect("valid JSON");
        assert!(value.is_object());
        assert!(!value["evidence_gaps"].as_array().unwrap().is_empty());
    }
    let sentinel = directory.join("keep.txt");
    std::fs::write(&sentinel, b"previous output").expect("write collision sentinel");
    let collision = Command::new(bin_path())
        .arg("--insight")
        .arg(&archive)
        .output()
        .expect("rerun insight");
    assert_eq!(collision.status.code(), Some(1));
    assert_eq!(
        std::fs::read(&sentinel).expect("preserve prior output"),
        b"previous output"
    );
    assert_eq!(
        std::fs::read(&archive).expect("read archive after collision"),
        bytes
    );
    std::fs::remove_dir_all(base).expect("clean test directory");
}

#[test]
fn insight_rejects_positional_archive_combinations() {
    let output = Command::new(bin_path())
        .args(["--insight", "sample.zip", "other.zip"])
        .output()
        .expect("run invalid invocation");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn insight_rejects_non_zip_without_publishing_output() {
    let archive = not_a_zip();
    let output_root = archive.parent().expect("fixture parent").join(format!(
        ".{}",
        archive.file_stem().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&output_root);
    let output = Command::new(bin_path())
        .arg("--insight")
        .arg(&archive)
        .output()
        .expect("run insight with invalid ZIP");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        !output_root.exists(),
        "failed analysis must not publish output"
    );
}

#[test]
fn no_mode_selected_is_a_usage_error() {
    let output = Command::new(bin_path())
        .arg(good_archive())
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "diagnostics go to stderr");
}

/// `--verbose` raises stderr detail and must never write to stdout.
#[test]
fn verbose_adds_stderr_detail_only() {
    let path = good_archive();
    let quiet = Server::start(&path);
    let quiet_out = quiet.finish();

    let mut server = Server::start_with_args(&path, &["--mcp", "--verbose"]);
    server.initialize();
    let verbose_out = server.finish();

    assert!(
        verbose_out.stderr.len() >= quiet_out.stderr.len(),
        "--verbose must not reduce stderr detail"
    );
    let stdout = String::from_utf8_lossy(&verbose_out.stdout);
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|e| panic!("--verbose polluted stdout ({e}): {line}"));
    }
}

fn resource_case() -> (PathBuf, PathBuf) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "insight-wire-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let archive = root.join("case report.zip");
    std::fs::copy(bugreport_insight_test_fixtures::insight_archive(), &archive).unwrap();
    (root, archive)
}

#[test]
fn mcp_resources_return_all_generated_artifacts_byte_for_byte() {
    let (root, archive) = resource_case();
    let before = std::fs::read(&archive).unwrap();
    let generated = Command::new(bin_path())
        .arg("--insight")
        .arg(&archive)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let mut server = Server::start(&archive);
    let initialized = server.initialize_version("2025-11-25");
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    let listed = server.request(&json!({"jsonrpc":"2.0","id":100,"method":"resources/list"}));
    assert_eq!(listed["result"]["ttlMs"], 0);
    assert_eq!(listed["result"]["cacheScope"], "private");
    let resources = listed["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 7);
    for resource in resources {
        let frame = server.request(&json!({"jsonrpc":"2.0","id":101,"method":"resources/read","params":{"uri":resource["uri"]}}));
        assert!(frame.get("error").is_none(), "{frame}");
        assert_eq!(frame["result"]["ttlMs"], 0);
        assert_eq!(frame["result"]["cacheScope"], "private");
        let contents = &frame["result"]["contents"][0];
        assert_eq!(contents["uri"], resource["uri"]);
        assert_eq!(contents["mimeType"], resource["mimeType"]);
        let expected = std::fs::read(
            root.join(".case report/insight")
                .join(resource["name"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(contents["text"].as_str().unwrap().as_bytes(), expected);
        assert_eq!(resource["size"].as_u64().unwrap(), expected.len() as u64);
    }
    let unknown = server.request(&json!({"jsonrpc":"2.0","id":102,"method":"resources/read","params":{"uri":"file:///etc/passwd"}}));
    assert_eq!(unknown["error"]["code"], -32002);
    assert_eq!(
        server.list_tools()["result"]["tools"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert!(server.finish().status.success());
    assert_eq!(before, std::fs::read(&archive).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mcp_missing_resources_are_empty_without_generating_files_and_refresh_after_publication() {
    let (root, archive) = resource_case();
    let mut server = Server::start(&archive);
    server.initialize();
    let listed = server.request(&json!({"jsonrpc":"2.0","id":100,"method":"resources/list"}));
    assert_eq!(listed["result"]["resources"], json!([]));
    assert!(!root.join(".case report").exists());
    let unknown = server.request(&json!({"jsonrpc":"2.0","id":101,"method":"resources/read","params":{"uri":"bugreport-insight://missing/insight/README.md"}}));
    assert_eq!(unknown["error"]["code"], -32002);
    let cursor = server.request(
        &json!({"jsonrpc":"2.0","id":102,"method":"resources/list","params":{"cursor":"invalid"}}),
    );
    assert_eq!(cursor["error"]["code"], -32602);
    let generated = Command::new(bin_path())
        .arg("--insight")
        .arg(&archive)
        .output()
        .unwrap();
    assert!(generated.status.success());
    let refreshed = server.request(&json!({"jsonrpc":"2.0","id":103,"method":"resources/list"}));
    assert_eq!(
        refreshed["result"]["resources"].as_array().unwrap().len(),
        7
    );
    assert!(server.finish().status.success());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn line_read_preserves_utf8_text_over_stdio() {
    let text = "café — Tiếng Việt — 日本語 — 🦀";
    let path = bugreport_insight_test_fixtures::scratch_dir().join("utf8-line-wire.zip");
    std::fs::write(
        &path,
        ZipBuilder::new()
            .add("utf8.txt", format!("{text}\n").as_bytes())
            .finish(),
    )
    .unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let env = server.envelope(
        "read_entry",
        json!({"entry_id": 0, "line_start": 1, "line_end": 1}),
    );
    let line = &env["data"]["lines"][0];
    assert_eq!(line["excerpt"]["text"], json!(text));
    assert_eq!(line["excerpt"]["decoding"], json!("Utf8"));
    assert_eq!(line["excerpt"]["bytes"], json!(text.len()));
    assert_eq!(line["byte_len_full"], json!(text.len()));
    assert_eq!(line["byte_offset"], json!(0));
    assert_eq!(line["excerpt"]["complete"], json!(true));
    assert_eq!(env["truncated"], json!(false));
    assert!(server.finish().status.success());
}

#[test]
fn excerpt_cap_supports_explicit_byte_follow_up_and_repeatable_grep() {
    let source = "café".repeat(120);
    let prefix = format!("{}ca", "café".repeat(102));
    let tail = format!("fé{}", "café".repeat(17));
    let path = bugreport_insight_test_fixtures::scratch_dir().join("excerpt-cap-wire.zip");
    std::fs::write(
        &path,
        ZipBuilder::new()
            .add("text", format!("{source}\n").as_bytes())
            .finish(),
    )
    .unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let args = json!({"entry_id": 0, "line_start": 1, "line_end": 1});
    let first = server.envelope("read_entry", args.clone());
    assert_eq!(first["data"]["lines"][0]["byte_len_full"], json!(600));
    let excerpt = &first["data"]["lines"][0]["excerpt"];
    assert_eq!(excerpt["text"], json!(prefix));
    assert_eq!(excerpt["bytes"], json!(512));
    assert_eq!(excerpt["source_bytes"], json!(512));
    assert_eq!(excerpt["complete"], json!(false));
    assert_eq!(first["truncated"], json!(true));
    assert_eq!(first["stopped_reason"], json!("byte_budget"));
    assert_eq!(first["omitted_count"], json!(0));
    assert_eq!(server.envelope("read_entry", args), first);
    let second = server.envelope(
        "read_entry",
        json!({"entry_id":0,"byte_start":512,"byte_end":600}),
    );
    assert_eq!(second["data"]["excerpt"]["text"], json!(tail));
    assert_eq!(second["data"]["excerpt"]["bytes"], json!(88));
    assert_eq!(second["truncated"], json!(false));
    assert_eq!(second["data"]["excerpt"]["complete"], json!(true));
    let byte_first = server.envelope(
        "read_entry",
        json!({"entry_id":0,"byte_start":0,"byte_end":600}),
    );
    assert_eq!(byte_first["data"]["excerpt"]["text"], json!(prefix));
    assert_eq!(byte_first["data"]["bytes_returned"], json!(600));
    let grep_args = json!({"entry_ids":[0],"pattern":"café","mode":"literal"});
    let grep = server.envelope("grep_archive", grep_args.clone());
    assert_eq!(grep["data"]["matches"].as_array().unwrap().len(), 1);
    assert_eq!(grep["data"]["matches"][0]["excerpt"]["text"], json!(prefix));
    assert_eq!(
        grep["data"]["matches"][0]["excerpt"]["source_bytes"],
        json!(512)
    );
    assert_eq!(grep["data"]["matches"][0]["byte_len_match"], json!(5));
    assert_eq!(grep["data"]["matches"][0]["byte_offset"], json!(0));
    assert_eq!(grep["truncated"], json!(true));
    assert_eq!(grep["omitted_count"], json!(0));
    assert_eq!(grep["data"]["scan"]["coverage_complete"], json!(true));
    assert_eq!(server.envelope("grep_archive", grep_args), grep);
    assert!(server.finish().status.success());
}

#[test]
fn sanitized_excerpt_follow_up_uses_source_bytes_instead_of_emitted_bytes() {
    let mut data = vec![b'a'; 500];
    data.extend_from_slice(&[0; 8]);
    let path =
        bugreport_insight_test_fixtures::scratch_dir().join("excerpt-source-progress-wire.zip");
    std::fs::write(&path, ZipBuilder::new().add("text", &data).finish()).unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let first = server.envelope(
        "read_entry",
        json!({"entry_id":0,"byte_start":0,"byte_end":508}),
    );
    let excerpt = &first["data"]["excerpt"];
    assert_eq!(excerpt["bytes"], json!(512));
    assert_eq!(excerpt["source_bytes"], json!(504));
    assert_eq!(excerpt["complete"], json!(false));
    let next = excerpt["source_bytes"].as_u64().unwrap();
    let second = server.envelope(
        "read_entry",
        json!({"entry_id":0,"byte_start":next,"byte_end":508}),
    );
    assert_eq!(
        second["data"]["excerpt"]["text"],
        json!("\u{fffd}".repeat(4))
    );
    assert_eq!(second["data"]["excerpt"]["bytes"], json!(12));
    assert_eq!(second["data"]["excerpt"]["source_bytes"], json!(4));
    assert_eq!(second["data"]["excerpt"]["truncated"], json!(false));
    assert_eq!(second["data"]["excerpt"]["complete"], json!(false));
    assert!(server.finish().status.success());
}

#[test]
fn archive_info_reports_marker_states_and_allows_source_follow_up() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let normal = server.envelope("archive_info", json!({"with_sha256":false}));
    assert_eq!(
        normal["data"]["archive"]["markers"]["version"]["status"],
        json!("valid")
    );
    assert_eq!(normal["data"]["archive"]["format_version"], json!("2"));
    assert_eq!(
        normal["data"]["archive"]["markers"]["main_entry"]["status"],
        json!("missing")
    );
    assert_eq!(normal["truncated"], json!(false));
    assert!(server.finish().status.success());
    for (index, marker, expected) in [
        (0, vec![b'x'; 300], "oversized"),
        (1, vec![b'2', 0xff], "invalid_utf8"),
        (2, vec![b'2', 0], "contains_nul"),
        (3, b"2\n".to_vec(), "unreadable"),
    ] {
        let path =
            bugreport_insight_test_fixtures::scratch_dir().join(format!("marker-wire-{index}.zip"));
        let mut bytes = ZipBuilder::new()
            .add("version.txt", &marker)
            .add("other", b"ok")
            .finish();
        if expected == "unreadable" {
            bytes[30 + "version.txt".len()] ^= 1;
        }
        std::fs::write(&path, bytes).unwrap();
        let mut server = Server::start(&path);
        server.initialize();
        let env = server.envelope("archive_info", json!({"with_sha256":false}));
        assert_success_bound_fields(&env);
        let identity = &env["data"]["archive"];
        assert_eq!(identity["entry_count"], json!(2));
        assert!(identity["format_version"].is_null());
        let state = &identity["markers"]["version"];
        assert_eq!(state["status"], json!(expected));
        assert!(state["error_code"].is_string());
        assert_eq!(env["truncated"], json!(expected == "oversized"));
        assert_eq!(env["omitted_count"], json!(0));
        assert!(!env["notes"].as_array().unwrap().is_empty());
        let args = json!({"entry_id":state["entry_id"],"byte_start":0,"byte_end":marker.len()});
        if expected == "unreadable" {
            let failure = server.call_tool(
                "read_entry",
                json!({"entry_id":state["entry_id"],"line_start":1,"line_end":1}),
            );
            assert_eq!(failure["result"]["isError"], json!(true));
            assert_eq!(
                server.envelope("list_entries", json!({}))["data"]["entry_count"],
                json!(2)
            );
        } else {
            let source = server.envelope("read_entry", args);
            assert_eq!(source["data"]["bytes_returned"], json!(marker.len()));
        }
        assert!(server.finish().status.success());
    }
}

#[test]
fn grep_pages_fit_wire_budgets_and_preserve_every_match() {
    let path = bugreport_insight_test_fixtures::scratch_dir().join("grep-page-wire.zip");
    let text = (0..75)
        .map(|i| format!("hit {i} café \"\\\t{}\n", "z".repeat(180)))
        .collect::<String>();
    let original = bugreport_insight_test_fixtures::ZipBuilder::new()
        .add("text", text.as_bytes())
        .finish();
    std::fs::write(&path, &original).unwrap();
    for version in ["2025-06-18", "2026-07-28"] {
        let mut server = Server::start(&path);
        server.initialize_version(version);
        let mut args = json!({"pattern":"hit", "mode":"literal", "response_budget":2048,
            "scan_budget":600, "max_matches":3});
        let mut hits = Vec::new();
        let mut total = 0;
        let mut done = false;
        let mut used = None;
        for _ in 0..200 {
            let frame = server.call_tool("grep_archive", args);
            assert!(frame.get("error").is_none(), "{frame}");
            assert_ne!(frame["result"]["isError"], true, "{frame}");
            let result = &frame["result"];
            assert!(
                serde_json::to_vec(result).unwrap().len() <= 2048,
                "{result}"
            );
            let env: Value =
                serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
            assert!(env["data"]["matches"].as_array().unwrap().len() <= 3);
            let bytes = env["data"]["scan"]["scanned_bytes"].as_u64().unwrap();
            assert!(bytes <= 600);
            total += bytes;
            for hit in env["data"]["matches"].as_array().unwrap() {
                hits.push(hit["line"].as_u64().unwrap());
            }
            if let Some(cursor) = env["data"]["next_cursor"].as_str() {
                assert_eq!(env["data"]["scan"]["coverage_complete"], false);
                if let Some(old) = used.take() {
                    let rejected = server.call_tool("grep_archive", json!({"cursor":old}));
                    assert!(
                        rejected.get("error").is_some(),
                        "replay must fail: {rejected}"
                    );
                }
                used = Some(cursor.to_string());
                args = json!({"cursor":cursor,"response_budget":2048,"scan_budget":600,"max_matches":3});
            } else {
                assert_eq!(env["data"]["scan"]["coverage_complete"], true);
                done = true;
                break;
            }
        }
        assert!(done);
        assert_eq!(hits, (1..=75).collect::<Vec<_>>());
        assert_eq!(
            total,
            text.len() as u64,
            "continuation must not re-decompress prefixes"
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn grep_empty_pages_continue_and_invalid_budgets_preserve_cursor() {
    let path = bugreport_insight_test_fixtures::scratch_dir().join("grep-empty-page-wire.zip");
    std::fs::write(
        &path,
        bugreport_insight_test_fixtures::ZipBuilder::new()
            .add("text", "café\nneedle".as_bytes())
            .finish(),
    )
    .unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let first = server.envelope(
        "grep_archive",
        json!({"pattern":"needle","mode":"literal","scan_budget":2}),
    );
    assert_eq!(first["status"], "ok");
    assert_eq!(first["data"]["matches"], json!([]));
    assert_eq!(first["stopped_reason"], "scan_budget");
    assert_eq!(first["data"]["scan"]["partial_entry_id"], 0);
    assert_eq!(first["data"]["scan"]["coverage_complete"], false);
    let cursor = first["data"]["next_cursor"].as_str().unwrap();
    for bad in [
        json!({"cursor":cursor,"scan_budget":0}),
        json!({"cursor":cursor,"response_budget":1}),
        json!({"cursor":cursor,"pattern":"different"}),
        json!({"cursor":cursor,"max_matches":51}),
    ] {
        assert!(server.call_tool("grep_archive", bad).get("error").is_some());
    }
    let second = server.envelope("grep_archive", json!({"cursor":cursor}));
    assert_eq!(second["data"]["matches"][0]["line"], 2);
    assert_eq!(second["data"]["matches"][0]["byte_offset"], 6);
    assert_eq!(second["data"]["scan"]["coverage_complete"], true);
    assert!(second["data"]["next_cursor"].is_null());
    assert_eq!(second["data"]["scan"]["total_scanned_bytes"], 12);
}

#[test]
fn grep_reports_long_line_and_encrypted_coverage_gaps_over_stdio() {
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
    let path = scratch_dir().join("grep-long-line-wire.zip");
    let mut text = vec![b'x'; 1024 * 1024 + 1];
    text.extend_from_slice(b"needle\nneedle\n");
    std::fs::write(&path, ZipBuilder::new().add("text", &text).finish()).unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let result = server.envelope("grep_archive", json!({"pattern":"needle","mode":"literal"}));
    assert_eq!(result["data"]["matches"].as_array().unwrap().len(), 1);
    assert_eq!(result["data"]["matches"][0]["line"], 2);
    assert_eq!(result["data"]["scan"]["skipped_lines"], 1);
    assert_eq!(result["data"]["scan"]["coverage_complete"], false);
    assert_eq!(result["stopped_reason"], "line_too_long");
    assert!(result["data"]["next_cursor"].is_null());
    assert_eq!(result["data"]["scan"]["scanned_bytes"], text.len());

    let encrypted = scratch_dir().join("grep-encrypted-wire.zip");
    let mut bytes = ZipBuilder::new().add("text", b"needle\n").finish();
    bytes[6] |= 1;
    let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    bytes[central + 8] |= 1;
    std::fs::write(&encrypted, bytes).unwrap();
    let mut server = Server::start(&encrypted);
    server.initialize();
    let result = server.envelope("grep_archive", json!({"pattern":"needle","mode":"literal"}));
    assert_eq!(result["status"], "ok");
    assert_eq!(result["data"]["scan"]["skipped_entries"], 1);
    assert_eq!(result["data"]["scan"]["coverage_complete"], false);
    assert_eq!(result["stopped_reason"], "skipped_content");
    assert!(result["data"]["next_cursor"].is_null());
}

#[test]
fn empty_collections_and_partial_detection_failures_are_distinct_over_stdio() {
    use bugreport_insight_test_fixtures::{ZipBuilder, mark_encrypted, scratch_dir};
    let clean = scratch_dir().join("empty-types-wire.zip");
    std::fs::write(
        &clean,
        ZipBuilder::new().add("notes.txt", b"readable\n").finish(),
    )
    .unwrap();
    let mut server = Server::start(&clean);
    server.initialize();
    for (tool, args, key) in [
        ("list_entries", json!({"path_prefix":"missing/"}), "entries"),
        (
            "find_entries",
            json!({"pattern":"missing", "mode":"literal"}),
            "entries",
        ),
        ("find_by_type", json!({"content_type":"json"}), "matches"),
    ] {
        let env = server.envelope(tool, args);
        assert_eq!(env["status"], "empty");
        assert_eq!(env["data"][key], json!([]));
        assert_eq!(env["truncated"], false);
    }
    let path = scratch_dir().join("partial-types-wire.zip");
    let bytes = mark_encrypted(
        ZipBuilder::new()
            .add("notes.txt", b"readable\n")
            .add("report.json", b"{}\n")
            .add("mystery", b"")
            .add("folder/", b"")
            .finish(),
        &[1],
    );
    std::fs::write(&path, &bytes).unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    let partial = server.envelope("find_by_type", json!({"content_type":"json"}));
    assert_eq!(partial["status"], "ok");
    assert_eq!(partial["data"]["matches"], json!([]));
    assert_eq!(partial["data"]["coverage"]["coverage_complete"], false);
    assert_eq!(partial["data"]["failures"][0]["entry_id"], 1);
    assert_eq!(
        partial["data"]["failures"][0]["error"]["code"],
        "entry_encrypted"
    );
    assert_eq!(partial["stopped_reason"], "skipped_content");
    let unknown = server.envelope("find_by_type", json!({"content_type":"unknown"}));
    assert_eq!(unknown["data"]["matches"][0]["entry"]["entry_id"], 2);
    let direct = server.call_tool("entry_info", json!({"entry_id":1}));
    assert_eq!(direct["result"]["isError"], true);
    let body: Value =
        serde_json::from_str(direct["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(body["error"]["data"]["status"], "unreadable");
    assert!(body.get("truncated").is_none());
    let surviving = server.envelope("list_entries", json!({}));
    assert_eq!(surviving["data"]["entries"].as_array().unwrap().len(), 4);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn eof_ranges_and_subtree_boundaries_work_over_stdio() {
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
    let path = scratch_dir().join("wire-range-boundaries.zip");
    let original = ZipBuilder::new()
        .add("ANR/trace.txt", b"alpha\nbeta\ngamma\n")
        .add("ANR2/trace.txt", b"sibling\n")
        .finish();
    std::fs::write(&path, &original).unwrap();
    let mut server = Server::start(&path);
    server.initialize();
    for prefix in ["ANR", "ANR/"] {
        let env = server.envelope("list_entries", json!({"path_prefix":prefix}));
        let entries = env["data"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["entry_id"], 0);
    }
    for args in [
        json!({"entry_id":0,"line_start":2,"line_end":5}),
        json!({"entry_id":0,"byte_start":14,"byte_end":20}),
        json!({"entry_id":0,"line_start":5,"line_end":8}),
    ] {
        let env = server.envelope("read_entry", args);
        assert_success_bound_fields(&env);
        assert_eq!(env["data"]["past_end"], true);
        assert_eq!(env["truncated"], true);
        assert_eq!(env["stopped_reason"], "end_of_entry");
        assert_eq!(env["omitted_count"], 0);
    }
    for args in [
        json!({"entry_id":0,"line_start":0,"line_end":1}),
        json!({"entry_id":0,"line_start":5,"line_end":2}),
        json!({"entry_id":0,"byte_start":5,"byte_end":2}),
    ] {
        let frame = server.call_tool("read_entry", args);
        assert_eq!(frame["error"]["code"], -32602);
        assert!(frame.get("result").is_none());
    }
    let env = server.envelope(
        "read_entry",
        json!({"entry_id":0,"line_start":3,"line_end":3}),
    );
    assert_eq!(env["data"]["past_end"], false);
    assert_eq!(env["data"]["lines"][0]["excerpt"]["text"], "gamma");
    assert!(server.finish().status.success());
    assert_eq!(std::fs::read(path).unwrap(), original);
}

fn confirmation_challenge(frame: &Value) -> Value {
    if frame["error"]["data"]["code"] == "confirmation_required" {
        return frame["error"]["data"].clone();
    }
    assert_eq!(frame["result"]["isError"], true, "{frame}");
    let body: Value =
        serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "confirmation_required");
    for key in ["status", "truncated", "stopped_reason", "omitted_count"] {
        assert!(
            body.get(key).is_none(),
            "preview must not claim content success"
        );
    }
    body["error"]["data"].clone()
}

#[test]
fn every_content_operation_previews_cost_before_execution_over_stdio() {
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
    for version in ["2025-06-18", "2025-11-25"] {
        let path = scratch_dir().join(format!("confirmation-no-io-{version}.zip"));
        std::fs::write(
            &path,
            ZipBuilder::new()
                .add("version.txt", b"1\n")
                .add("text", b"cafe\n")
                .finish(),
        )
        .unwrap();
        let mut server = Server::start(&path);
        server.initialize_version(version);
        // The immutable index survives, but content operations cannot open this path now.
        std::fs::remove_file(&path).unwrap();
        let list = server.envelope("list_entries", json!({}));
        assert_eq!(list["data"]["entries"].as_array().unwrap().len(), 2);
        for (name, args) in [
            ("archive_info", json!({})),
            ("entry_info", json!({"entry_id":1})),
            ("find_by_type", json!({"content_type":"text"})),
            (
                "read_entry",
                json!({"entry_id":1,"line_start":1,"line_end":1}),
            ),
            ("grep_archive", json!({"pattern":"cafe","mode":"literal"})),
        ] {
            let mut req = json!({"jsonrpc":"2.0","id":500,"method":"tools/call","params":{"name":name,"arguments":args}});
            let challenge = confirmation_challenge(&server.request_once(&req));
            assert_eq!(challenge["operation"], name);
            assert!(challenge["cost"].is_object());
            assert_eq!(challenge["expires_in_seconds"], 300);
            if name == "archive_info" {
                req["params"]["arguments"]["confirmation"] =
                    challenge["confirmation_token"].clone();
                let result = server.request_once(&req);
                let body: Value =
                    serde_json::from_str(result["result"]["content"][0]["text"].as_str().unwrap())
                        .unwrap();
                assert_ne!(body["error"]["code"], "confirmation_required");
                assert_ne!(
                    result["result"]["isError"], true,
                    "approved operation uses the pinned archive even after pathname removal"
                );
                assert_eq!(
                    server.request_once(&req)["error"]["data"]["code"],
                    "invalid_confirmation"
                );
            }
        }
        for (method, params) in [
            ("resources/list", json!({})),
            ("resources/read", json!({"uri":"file:///not-an-artifact"})),
        ] {
            let req = json!({"jsonrpc":"2.0","id":600,"method":method,"params":params});
            let challenge = confirmation_challenge(&server.request_once(&req));
            assert_eq!(challenge["operation"], method);
            assert!(challenge["cost"].is_object());
        }
        assert!(server.finish().status.success());
    }
}

#[test]
fn confirmations_bind_tool_budgets_cursor_pages_and_resource_uris() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let mut req = json!({"jsonrpc":"2.0","id":700,"method":"tools/call","params":{"name":"grep_archive","arguments":{"pattern":"line","mode":"literal","scan_budget":1}}});
    let challenge = confirmation_challenge(&server.request_once(&req));
    req["params"]["arguments"]["confirmation"] = challenge["confirmation_token"].clone();
    req["params"]["arguments"]["scan_budget"] = json!(2);
    assert_eq!(
        server.request_once(&req)["error"]["data"]["code"],
        "invalid_confirmation"
    );
    req["params"]["arguments"]["scan_budget"] = json!(1);
    let frame = server.request_once(&req);
    assert_ne!(frame["result"]["isError"], true);
    let page: Value =
        serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let cursor = page["data"]["next_cursor"].as_str().unwrap();
    let mut next = json!({"jsonrpc":"2.0","id":701,"method":"tools/call","params":{"name":"grep_archive","arguments":{"cursor":cursor,"scan_budget":1}}});
    let challenge = confirmation_challenge(&server.request_once(&next));
    // Issuing another preview must not consume the search cursor.
    confirmation_challenge(&server.request_once(&next));
    next["params"]["arguments"]["confirmation"] = challenge["confirmation_token"].clone();
    assert_ne!(server.request_once(&next)["result"]["isError"], true);
    assert_eq!(
        server.request_once(&next)["error"]["data"]["code"],
        "invalid_confirmation"
    );
    let mut resource = json!({"jsonrpc":"2.0","id":702,"method":"resources/read","params":{"uri":"bugreport-insight://a/insight/README.md"}});
    let challenge = confirmation_challenge(&server.request_once(&resource));
    resource["params"]["_meta"] =
        json!({"bugreport-insight/confirmation":challenge["confirmation_token"]});
    resource["params"]["uri"] = json!("bugreport-insight://b/insight/README.md");
    assert_eq!(
        server.request_once(&resource)["error"]["data"]["code"],
        "invalid_confirmation"
    );
    resource["params"]["uri"] = json!("bugreport-insight://a/insight/README.md");
    let frame = server.request_once(&resource);
    assert_eq!(
        frame["error"]["code"], -32002,
        "approved nonexistent resource now reaches provider"
    );
    assert_eq!(
        server.request_once(&resource)["error"]["data"]["code"],
        "invalid_confirmation"
    );
    assert!(server.finish().status.success());
}

#[test]
fn force_admits_large_entry_count_and_explains_default_rejection() {
    let root =
        bugreport_insight_test_fixtures::scratch_dir().join("force-directory-entries-process");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("many.zip");
    let mut builder = ZipBuilder::new();
    for index in 0..10_001 {
        builder = builder.add(&format!("entry-{index}/"), b"");
    }
    let bytes = builder.finish();
    std::fs::write(&path, &bytes).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    for mode in ["--mcp", "--insight"] {
        let rejected = Command::new(bin_path())
            .args([mode])
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(rejected.status.code(), Some(1));
        assert!(rejected.stdout.is_empty());
        let diagnostic = String::from_utf8_lossy(&rejected.stderr);
        assert!(diagnostic.contains("10001"), "{diagnostic}");
        assert!(diagnostic.contains("too large"), "{diagnostic}");
        assert!(diagnostic.contains("--force"), "{diagnostic}");
        let accepted = Command::new(bin_path())
            .args(["--force", mode])
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            accepted.status.success(),
            "{}",
            String::from_utf8_lossy(&accepted.stderr)
        );
        assert!(accepted.stdout.is_empty());
        assert!(String::from_utf8_lossy(&accepted.stderr).contains("warning: --force"));
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn combined_file_mode_exposes_insight_tools_without_publishing_artifacts() {
    let root = bugreport_insight_test_fixtures::scratch_dir().join("combined-file-process");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("report.zip");
    std::fs::copy(bugreport_insight_test_fixtures::insight_archive(), &path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let mut server = Server::start_insight(&path);
    let init = server.initialize();
    for name in ["list_bugreports", "list_insights", "read_insight"] {
        assert!(
            init["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains(name)
        );
    }
    let tools = server.list_tools();
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 10);
    let reports = server.envelope("list_bugreports", json!({}));
    assert_eq!(reports["data"]["bugreports"][0]["bugreport"], "report.zip");
    let listing = server.envelope("list_insights", json!({"bugreport":"report.zip"}));
    assert_eq!(listing["data"]["artifacts"].as_array().unwrap().len(), 7);
    let first = server.envelope(
        "read_insight",
        json!({"bugreport":"report.zip","artifact":"system.json","max_bytes":31}),
    );
    assert_eq!(first["truncated"], true);
    assert!(first["data"]["bytes_returned"].as_u64().unwrap() <= 31);
    let mut text = first["data"]["text"].as_str().unwrap().to_owned();
    let mut next = first["data"]["next_byte_start"].clone();
    while !next.is_null() {
        let page = server.envelope("read_insight", json!({"bugreport":"report.zip","artifact":"system.json","byte_start":next,"max_bytes":65536,"source_sha256":first["data"]["source_sha256"],"snapshot_id":first["data"]["snapshot_id"]}));
        text.push_str(page["data"]["text"].as_str().unwrap());
        next = page["data"]["next_byte_start"].clone();
    }
    let system: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(system["artifact"], "system");
    assert!(server.finish().status.success());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn combined_directory_mode_exposes_archive_and_insight_tools_for_discovered_reports() {
    let root = bugreport_insight_test_fixtures::scratch_dir().join("combined-directory-process");
    let directory = root.join("allowed");
    let nested = directory.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let bytes = ZipBuilder::new()
        .add("main_entry.txt", b"bugreport.txt")
        .add(
            "bugreport.txt",
            b"------ SERVICE meminfo ------\nOEM data\n",
        )
        .finish();
    for path in [
        root.join("parent.zip"),
        directory.join("inside.ZIP"),
        nested.join("nested.zip"),
    ] {
        std::fs::write(path, &bytes).unwrap();
    }
    std::fs::write(
        directory.join("unrelated.zip"),
        ZipBuilder::new().add("hello", b"world").finish(),
    )
    .unwrap();
    std::fs::write(directory.join("broken.zip"), b"not ZIP").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("parent.zip"), directory.join("escape.zip")).unwrap();
    let before: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let mut server = Server::start_insight(&directory);
    server.initialize();
    let tools = server.list_tools();
    let mut names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "archive_info",
            "entry_info",
            "find_by_type",
            "find_entries",
            "grep_archive",
            "list_bugreports",
            "list_entries",
            "list_insights",
            "read_entry",
            "read_insight",
        ]
    );
    let reports = server.envelope("list_bugreports", json!({}));
    assert_eq!(reports["data"]["bugreports"].as_array().unwrap().len(), 1);
    assert_eq!(reports["data"]["bugreports"][0]["bugreport"], "inside.ZIP");
    assert_eq!(reports["data"]["rejected"][0]["bugreport"], "broken.zip");
    for id in [
        "../parent.zip",
        "nested/nested.zip",
        "escape.zip",
        "/parent.zip",
        "..\\parent.zip",
        "unrelated.zip",
    ] {
        let frame = server.call_tool("list_insights", json!({"bugreport":id}));
        assert!(
            frame.get("error").is_some() || frame["result"]["isError"] == true,
            "{id}: {frame}"
        );
    }
    let memory = server.envelope(
        "read_insight",
        json!({"bugreport":"inside.ZIP","artifact":"memory.json","max_bytes":65536}),
    );
    let value: Value = serde_json::from_str(memory["data"]["text"].as_str().unwrap()).unwrap();
    assert!(value["system"]["total_ram_kib"].is_null());
    assert!(server.finish().status.success());
    assert_eq!(std::fs::read(directory.join("inside.ZIP")).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), before.len());
    std::fs::remove_dir_all(root).unwrap();
}
