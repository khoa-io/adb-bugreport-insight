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
    reader: BufReader<std::process::ChildStdout>,
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
            reader,
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

    /// Sends a request and reads until the frame with the matching id arrives.
    fn request(&mut self, msg: &Value) -> Value {
        let id = msg["id"].clone();
        self.send(msg);
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).expect("read response");
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
        let text = result["content"][0]["text"]
            .as_str()
            .expect("content[0] is text")
            .to_string();
        serde_json::from_str(&text).expect("envelope is JSON")
    }

    fn initialize(&mut self) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let frame = self.request(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
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

/// Shared with `crates/mcp/tests/tools.rs` so there is exactly one ZIP writer in
/// the tree. A second hand-rolled copy is how the central-directory header went
/// missing two bytes once already.
#[path = "../../mcp/tests/common/mod.rs"]
mod common;

use common::{good_archive, not_a_zip};

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
fn only_the_tools_capability_is_advertised() {
    let mut server = Server::start(&good_archive());
    let frame = server.initialize();
    let caps = &frame["result"]["capabilities"];
    assert!(
        caps.get("tools").is_some(),
        "tools capability must be advertised"
    );
    for absent in ["prompts", "resources", "logging"] {
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

/// A domain failure is a caller-visible tool error with a stable code.
#[test]
fn domain_failure_is_a_tool_error_not_a_protocol_error() {
    let mut server = Server::start(&good_archive());
    server.initialize();
    let frame = server.call_tool("entry_info", json!({ "entry_id": 4242 }));
    assert!(
        frame.get("error").is_none(),
        "a missing entry is a domain failure, not a protocol error: {}",
        frame["error"]
    );
    assert_eq!(frame["result"]["isError"], json!(true));
    let body: Value = serde_json::from_str(frame["result"]["content"][0]["text"].as_str().unwrap())
        .expect("error body is JSON");
    assert_eq!(body["error"]["code"], json!("entry_not_found"));
    server.finish();
}

/// An unknown field is rejected before the handler runs.
///
/// **Deviation from `design/05-mcp-server-design.md` §4, recorded rather than
/// papered over.** The design puts malformed parameters in the *protocol* class
/// (`ErrorData::invalid_params`, which clients render opaquely). rmcp's `#[tool]`
/// wrapper deserialises `Parameters<T>` itself and converts the failure into a
/// caller-visible `isError` result instead. The caller therefore sees *why* the
/// call failed — strictly more useful — but it arrives as a tool error rather
/// than a JSON-RPC error. The handler's own protocol-error path is still live
/// and is covered by the two tests below.
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
    for expected in ["--mcp", "--verbose", "--help", "bugreport-insight"] {
        assert!(
            stdout.contains(expected),
            "--help must mention `{expected}`, got: {stdout}"
        );
    }
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
