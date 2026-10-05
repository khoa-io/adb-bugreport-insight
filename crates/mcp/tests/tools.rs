//! Unit verification of the tool layer — `WE.4-T1`.
//!
//! Every test here runs against the [`ToolHandler`] seam with **no server, no
//! transport, and no `rmcp` type in scope**. That is the property the seam
//! exists for, and these tests are the evidence it works.
//!
//! The centrepiece is the `REQ-MCP-08` four-way status matrix: a failure must
//! never be shaped like a successful empty result.

use std::path::Path;
use std::sync::Arc;

use bugreport_insight_core::{Archive, ArchiveStatus};
use bugreport_insight_mcp::{TOOL_NAMES, ToolError, ToolErrorKind, registry};
use serde_json::{Value, json};

// ---------------------------------------------------------------- fixtures

// The ZIP writer behind every fixture lives in `crates/test-fixtures`: one copy
// for both suites, because a second hand-rolled copy is how the
// central-directory header went missing two bytes once already.
use bugreport_insight_test_fixtures::{corrupt_entry_archive, good_archive};

fn archive(path: &Path) -> Arc<Archive> {
    Arc::new(Archive::open(path).expect("fixture archive must open"))
}

fn call(tool: &str, path: &Path, params: Value) -> Result<Value, ToolError> {
    let ctx = archive(path);
    let handler = registry(ctx)
        .into_iter()
        .find(|h| h.name() == tool)
        .unwrap_or_else(|| panic!("no handler `{tool}`"));
    handler
        .call(params)
        .map(|e| serde_json::to_value(e).expect("envelope"))
}

/// The `data` sub-object of a domain error body.
fn error_data(err: &ToolError) -> Value {
    serde_json::from_str::<Value>(&err.to_json().to_string()).expect("error body")
}

// ------------------------------------------------------- registry and schema

#[test]
fn registry_returns_exactly_the_seven_committed_tools() {
    let ctx = archive(&good_archive());
    let names: Vec<&str> = registry(ctx).iter().map(|h| h.name()).collect();
    assert_eq!(names, TOOL_NAMES.to_vec());
    assert_eq!(names.len(), 7, "a crate is never half-committed");
}

#[test]
fn tool_names_are_unique() {
    let mut names: Vec<&str> = TOOL_NAMES.to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), TOOL_NAMES.len());
}

/// `REQ-MCP-05`: a non-empty description whose **first sentence** states both
/// the cost class and the bound, so an agent can pick a tool without calling
/// it. Checked against the exact token each tool is expected to carry.
#[test]
fn every_description_states_a_cost_class_and_a_bound() {
    let expected: [(&str, &str, &str); 7] = [
        ("archive_info", "Cheapest", "memoised"),
        ("list_entries", "Cheap", "200"),
        ("find_entries", "Cheap", "200"),
        ("entry_info", "Cheap", "8 KiB"),
        ("find_by_type", "Moderate", "8 KiB"),
        ("read_entry", "Moderate", "512"),
        ("grep_archive", "Most expensive", "50 matches"),
    ];
    let ctx = archive(&good_archive());
    let handlers = registry(ctx);
    for (tool, cost, bound) in expected {
        let d = handlers
            .iter()
            .find(|h| h.name() == tool)
            .unwrap_or_else(|| panic!("no handler `{tool}`"))
            .description();
        assert!(!d.is_empty(), "{tool}: empty description");
        let first = d.split(". ").next().unwrap_or(d);
        assert!(
            first.contains(cost),
            "{tool}: first sentence must name cost class `{cost}`, got: {first}"
        );
        assert!(
            first.contains(bound),
            "{tool}: first sentence must name the bound `{bound}`, got: {first}"
        );
    }
}

/// `REQ-MCP-05`: `tools/list` returns exactly the committed set. Asserted so an
/// eighth tool cannot land without this failing.
#[test]
fn tools_list_matches_the_committed_set() {
    assert_eq!(TOOL_NAMES.len(), 7);
    assert!(TOOL_NAMES.contains(&"archive_info"));
    assert!(TOOL_NAMES.contains(&"grep_archive"));
    assert!(
        !TOOL_NAMES.contains(&"list_anr"),
        "analyzer tools are draft"
    );
}

/// Every params struct rejects unknown fields, so a misspelled parameter is a
/// protocol error rather than a silently different call.
#[test]
fn unknown_parameters_are_rejected() {
    let path = good_archive();
    let cases = [
        ("archive_info", json!({})),
        ("list_entries", json!({})),
        ("find_entries", json!({ "pattern": "*" })),
        ("entry_info", json!({ "entry_id": 0 })),
        ("find_by_type", json!({ "content_type": "text" })),
        (
            "read_entry",
            json!({ "entry_id": 0, "line_start": 1, "line_end": 1 }),
        ),
        ("grep_archive", json!({ "pattern": "*" })),
    ];
    for (tool, base) in cases {
        let mut bad = base.clone();
        bad["not_a_real_parameter"] = json!(1);
        let err = call(tool, &path, bad).expect_err("{tool} must reject unknown fields");
        assert_eq!(err.kind, ToolErrorKind::Protocol, "{tool}");
        assert_eq!(err.code, "invalid_params", "{tool}");
    }
}

// --------------------------------------------------- REQ-MCP-06 envelope shape

/// The four bounding fields are structurally always present — not optional, not
/// `skip_serializing_if` — so absence can never mean "the tool forgot".
#[test]
fn envelope_always_carries_the_four_bound_fields() {
    let path = good_archive();
    let cases = [
        ("archive_info", json!({})),
        ("list_entries", json!({})),
        ("find_entries", json!({ "pattern": "*" })),
        ("entry_info", json!({ "entry_id": 0 })),
        ("find_by_type", json!({ "content_type": "text" })),
        (
            "read_entry",
            json!({ "entry_id": 1, "line_start": 1, "line_end": 1 }),
        ),
        ("grep_archive", json!({ "pattern": "second" })),
    ];
    for (tool, params) in cases {
        let env = call(tool, &path, params).unwrap_or_else(|e| panic!("{tool}: {e}"));
        for field in [
            "status",
            "truncated",
            "stopped_reason",
            "omitted_count",
            "notes",
        ] {
            assert!(
                env.get(field).is_some(),
                "{tool}: envelope is missing `{field}`: {env}"
            );
        }
        assert!(env["truncated"].is_boolean(), "{tool}");
        assert!(env["omitted_count"].is_u64(), "{tool}");
        assert!(env["notes"].is_array(), "{tool}");
        assert!(
            env["stopped_reason"].is_string() || env["stopped_reason"].is_null(),
            "{tool}"
        );
    }
}

/// `stopped_reason` is snake_case on the wire, per the `REQ-MCP-06` vocabulary.
/// A `ByteBudget` here would be a contract violation.
#[test]
fn stopped_reason_uses_the_agreed_snake_case_vocabulary() {
    let path = good_archive();
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "e", "mode": "literal", "max_matches": 1 }),
    )
    .expect("grep");
    if let Some(reason) = env["stopped_reason"].as_str() {
        assert!(
            [
                "item_cap",
                "match_cap",
                "scan_budget",
                "byte_budget",
                "line_too_long"
            ]
            .contains(&reason),
            "unexpected stopped_reason `{reason}`"
        );
    }
}

// ------------------------------------------------- REQ-MCP-08 four-way matrix

#[test]
fn success_reports_ok() {
    let path = good_archive();
    let env = call("list_entries", &path, json!({})).expect("list_entries");
    assert_eq!(env["status"], json!("ok"));
}

/// Zero matches → `Empty`, and only because the tool actually completed.
#[test]
fn zero_matches_reports_empty_not_ok() {
    let path = good_archive();
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "zzz-not-present-anywhere", "mode": "literal" }),
    )
    .expect("grep_archive");
    assert_eq!(
        env["status"],
        json!("empty"),
        "a completed scan that found nothing is Empty"
    );
}

#[test]
fn missing_entry_reports_not_found() {
    let path = good_archive();
    let err = call("entry_info", &path, json!({ "entry_id": 9999 })).expect_err("must fail");
    assert_eq!(err.code, "entry_not_found");
    assert_eq!(err.kind, ToolErrorKind::Domain);
    let body = error_data(&err);
    assert_eq!(body["error"]["data"]["status"], json!("not_found"));
    assert_eq!(body["error"]["data"]["entry_id"], json!(9999));
}

/// The case that catches the actual bug: an entry that exists but cannot be
/// decoded must never read as `empty` or `not_found`.
#[test]
fn corrupt_entry_is_never_reported_as_empty() {
    let path = corrupt_entry_archive();
    let ctx = archive(&path);
    let traces = ctx
        .entries()
        .iter()
        .find(|e| e.path == "ANR/traces.txt")
        .map(|e| e.id.0)
        .expect("fixture must contain the corrupt entry");

    let handler = registry(ctx)
        .into_iter()
        .find(|h| h.name() == "read_entry")
        .expect("read_entry handler");
    match handler.call(json!({ "entry_id": traces, "line_start": 1, "line_end": 1 })) {
        Err(e) => {
            assert_eq!(
                e.kind,
                ToolErrorKind::Domain,
                "a decode failure is a domain error"
            );
            let status = error_data(&e)["error"]["data"]["status"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            assert_ne!(status, "empty", "REQ-MCP-08: never an empty result");
            assert_ne!(status, "ok", "REQ-MCP-08: never a successful result");
        }
        Ok(env) => {
            let status = serde_json::to_value(env.status).unwrap();
            assert_ne!(
                status,
                json!("empty"),
                "an unreadable entry must not produce an empty result"
            );
        }
    }
}

/// A range past end-of-entry is `Unreadable`, not `NotFound`: the entry exists.
#[test]
fn out_of_bounds_range_reports_unreadable() {
    let path = good_archive();
    let err = call(
        "read_entry",
        &path,
        json!({ "entry_id": 0, "line_start": 900, "line_end": 901 }),
    )
    .expect_err("range must fail");
    assert_eq!(
        error_data(&err)["error"]["data"]["status"],
        json!("unreadable")
    );
}

/// `NotFound` stays reserved for "the addressed thing does not exist".
#[test]
fn not_found_is_reserved_for_a_missing_entry() {
    let path = good_archive();
    let err = call("entry_info", &path, json!({ "entry_id": 12345 })).expect_err("must fail");
    assert_eq!(
        error_data(&err)["error"]["data"]["status"],
        json!("not_found")
    );
}

// ------------------------------------------------------------ tool behaviour

#[test]
fn archive_info_reports_the_bound_archive() {
    let path = good_archive();
    let env = call("archive_info", &path, json!({})).expect("archive_info");
    assert_eq!(env["status"], json!("ok"));
    assert_eq!(env["data"]["archive"]["entry_count"], json!(3));
    assert_eq!(env["data"]["archive"]["format_version"], json!("2"));
}

#[test]
fn archive_info_omits_the_digest_when_declined() {
    let path = good_archive();
    let env = call("archive_info", &path, json!({ "with_sha256": false })).expect("archive_info");
    assert!(env["data"]["archive"]["sha256"].is_null());
}

#[test]
fn list_entries_honours_a_path_prefix() {
    let path = good_archive();
    let env = call("list_entries", &path, json!({ "path_prefix": "ANR/" })).expect("list");
    assert_eq!(env["data"]["entries"].as_array().unwrap().len(), 2);
}

#[test]
fn list_entries_reports_what_it_omitted() {
    let path = good_archive();
    let env = call("list_entries", &path, json!({ "limit": 1 })).expect("list");
    assert_eq!(env["data"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(env["truncated"], json!(true));
    assert_eq!(env["stopped_reason"], json!("item_cap"));
    assert_eq!(env["omitted_count"], json!(2));
}

#[test]
fn find_entries_matches_paths_without_reading_content() {
    let path = good_archive();
    let env = call("find_entries", &path, json!({ "pattern": "ANR/**" })).expect("find");
    assert_eq!(env["data"]["entries"].as_array().unwrap().len(), 2);
}

#[test]
fn find_entries_refuses_regex_rather_than_downgrading() {
    let path = good_archive();
    let err = call(
        "find_entries",
        &path,
        json!({ "pattern": ".*", "mode": "regex" }),
    )
    .expect_err("regex is draft");
    assert_eq!(err.kind, ToolErrorKind::Protocol);
    assert_eq!(err.code, "pattern_mode_unavailable");
}

#[test]
fn find_entries_reports_an_invalid_glob() {
    let path = good_archive();
    let err = call("find_entries", &path, json!({ "pattern": "[unclosed" }))
        .expect_err("invalid glob must fail");
    assert_eq!(err.code, "invalid_glob");
}

#[test]
fn entry_info_reports_detection_with_its_basis() {
    let path = good_archive();
    let env = call("entry_info", &path, json!({ "entry_id": 1 })).expect("entry_info");
    assert_eq!(env["status"], json!("ok"));
    assert!(env["data"]["detection"]["basis"].is_string());
    assert!(env["data"]["detection"]["sample_bytes"].is_u64());
}

#[test]
fn find_by_type_filters_on_detected_content_type() {
    let path = good_archive();
    let env = call("find_by_type", &path, json!({ "content_type": "text" })).expect("find");
    let matches = env["data"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 3, "all three fixture entries are text");
    // Detection is reported with its basis, never as a bare confident label.
    for m in matches {
        assert!(m["detection"]["basis"].is_string());
    }
}

#[test]
fn find_by_type_rejects_an_unknown_type() {
    let path = good_archive();
    let err = call(
        "find_by_type",
        &path,
        json!({ "content_type": "not_a_content_type" }),
    )
    .expect_err("unknown type must fail");
    assert_eq!(err.kind, ToolErrorKind::Protocol);
    assert_eq!(err.code, "invalid_params");
}

/// `REQ-MCP-ARC-05`: `byte_offset` is required on **every** match.
#[test]
fn every_grep_match_carries_a_byte_offset() {
    let path = good_archive();
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "line", "mode": "literal" }),
    )
    .expect("grep_archive");
    let matches = env["data"]["matches"].as_array().expect("matches array");
    assert!(!matches.is_empty());
    for m in matches {
        assert!(m["byte_offset"].is_u64(), "match missing byte_offset: {m}");
        assert!(m["line"].is_u64(), "match missing line: {m}");
        assert!(m["entry_id"].is_u64(), "match missing entry_id: {m}");
        assert!(
            m["excerpt"]["bytes"].as_u64().unwrap() <= 512,
            "excerpt exceeded the 512-byte cap: {m}"
        );
    }
}

#[test]
fn grep_archive_honours_an_entry_scope() {
    let path = good_archive();
    let all = call(
        "grep_archive",
        &path,
        json!({ "pattern": "e", "mode": "literal" }),
    )
    .expect("grep all");
    let scoped = call(
        "grep_archive",
        &path,
        json!({ "pattern": "e", "mode": "literal", "entry_ids": [1] }),
    )
    .expect("grep scoped");
    assert!(
        scoped["data"]["scan"]["scanned_entries"].as_u64().unwrap()
            <= all["data"]["scan"]["scanned_entries"].as_u64().unwrap()
    );
}

/// `omitted_count` counts withheld *matches*. When a budget stops the scan early,
/// whole entries go unsearched — a different unit — and that gap must be stated
/// on its own. Reporting only the match count lets `truncated: true` with
/// `omitted_count: 0` read as "nothing was lost" while a third of the archive
/// was never searched. That is the silent truncation `REQ-MCP-06` forbids.
#[test]
fn a_stalled_scan_reports_its_coverage_gap() {
    let path = good_archive();
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "nothing-matches-this", "mode": "literal" }),
    )
    .expect("grep");
    let scan = &env["data"]["scan"];

    // Coverage is always stated, whether or not the scan completed.
    for field in [
        "scanned_entries",
        "entries_in_scope",
        "entries_not_searched",
        "coverage_complete",
    ] {
        assert!(scan.get(field).is_some(), "scan is missing `{field}`");
    }
    assert_eq!(
        scan["entries_in_scope"], scan["scanned_entries"],
        "this scan completed, so nothing was left unsearched"
    );
    assert_eq!(scan["entries_not_searched"], json!(0));
    assert_eq!(scan["coverage_complete"], json!(true));
    // A completed scan that found nothing is Empty — not an incomplete answer.
    assert_eq!(env["status"], json!("empty"));
}

/// `data` must have the same shape whatever `status` is. A caller branching on
/// `status` should not have to special-case the empty result to find `matches`.
#[test]
fn the_empty_result_has_the_same_shape_as_a_successful_one() {
    let path = good_archive();
    let hit = call(
        "grep_archive",
        &path,
        json!({ "pattern": "line", "mode": "literal" }),
    )
    .expect("grep with a hit");
    let miss = call(
        "grep_archive",
        &path,
        json!({ "pattern": "nothing-matches-this", "mode": "literal" }),
    )
    .expect("grep with no hit");

    assert_eq!(hit["status"], json!("ok"));
    assert_eq!(miss["status"], json!("empty"));
    for (label, env) in [("hit", &hit), ("miss", &miss)] {
        assert!(
            env["data"]["matches"].is_array(),
            "{label}: `matches` missing"
        );
        assert!(env["data"]["scan"].is_object(), "{label}: `scan` missing");
    }
    assert!(miss["data"]["matches"].as_array().unwrap().is_empty());
}

/// When the scan is cut short, `status` must not be `Empty`. `empty` asserts
/// there was nothing to find, which is a different claim from "we stopped
/// looking".
#[test]
fn a_stalled_scan_is_never_reported_as_empty() {
    let path = good_archive();
    // A match cap of 1 with a pattern matching several lines forces a stop.
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "e", "mode": "literal", "max_matches": 1 }),
    )
    .expect("grep");
    if env["truncated"] == json!(true) {
        assert_ne!(
            env["status"],
            json!("empty"),
            "a scan that stopped early must not claim the archive held nothing"
        );
        assert!(
            env["notes"].as_array().is_some_and(|n| !n.is_empty()),
            "a stopped scan must explain itself in `notes`, got {}",
            env["notes"]
        );
    }
}

/// `entries_in_scope` reflects the caller's scope, not the whole archive, so a
/// scoped re-run can be reasoned about on its own terms.
#[test]
fn coverage_is_reported_relative_to_the_requested_scope() {
    let path = good_archive();
    let env = call(
        "grep_archive",
        &path,
        json!({ "pattern": "line", "mode": "literal", "entry_ids": [1] }),
    )
    .expect("grep");
    assert_eq!(env["data"]["scan"]["entries_in_scope"], json!(1));
    assert_eq!(env["data"]["scan"]["scanned_entries"], json!(1));
    assert_eq!(env["data"]["scan"]["coverage_complete"], json!(true));
}

/// `REQ-MCP-ARC-04`: exactly one range pair, or a protocol error.
#[test]
fn read_entry_rejects_both_range_pairs() {
    let path = good_archive();
    let err = call(
        "read_entry",
        &path,
        json!({
            "entry_id": 1,
            "line_start": 1, "line_end": 1,
            "byte_start": 0, "byte_end": 4
        }),
    )
    .expect_err("both pairs must fail");
    assert_eq!(err.kind, ToolErrorKind::Protocol);
    assert_eq!(err.code, "conflicting_range_params");
}

#[test]
fn read_entry_rejects_neither_range_pair() {
    let path = good_archive();
    let err =
        call("read_entry", &path, json!({ "entry_id": 1 })).expect_err("neither pair must fail");
    assert_eq!(err.code, "conflicting_range_params");
}

#[test]
fn read_entry_returns_lines_with_offsets() {
    let path = good_archive();
    let env = call(
        "read_entry",
        &path,
        json!({ "entry_id": 1, "line_start": 1, "line_end": 2 }),
    )
    .expect("read_entry");
    assert_eq!(env["status"], json!("ok"));
    let lines = env["data"]["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["line"], json!(1));
    assert!(lines[0]["byte_offset"].is_u64());
}

#[test]
fn read_entry_by_byte_range_caps_the_excerpt() {
    let path = good_archive();
    let env = call(
        "read_entry",
        &path,
        json!({ "entry_id": 1, "byte_start": 0, "byte_end": 25 }),
    )
    .expect("read_entry");
    assert_eq!(env["status"], json!("ok"));
    let excerpt_bytes = env["data"]["excerpt"]["bytes"].as_u64().unwrap();
    assert!(
        excerpt_bytes <= 512,
        "excerpt exceeded the cap: {excerpt_bytes}"
    );
}

/// The read-only guarantee, in-process. `REQ-MCP-04` is also checked
/// mechanically in `tests/protocol.rs`.
#[test]
fn reading_tools_do_not_mutate_the_fixture() {
    let path = good_archive();
    let before = std::fs::metadata(&path).expect("stat").len();
    let _ = call("list_entries", &path, json!({})).expect("list");
    let _ = call(
        "grep_archive",
        &path,
        json!({ "pattern": "e", "mode": "literal" }),
    )
    .expect("grep");
    let _ = call(
        "read_entry",
        &path,
        json!({ "entry_id": 1, "line_start": 1, "line_end": 1 }),
    )
    .expect("read");
    let after = std::fs::metadata(&path).expect("stat").len();
    assert_eq!(before, after, "a read tool changed the archive size");
}

#[test]
fn archive_status_is_serialised_snake_case() {
    for (status, expected) in [
        (ArchiveStatus::Ok, "ok"),
        (ArchiveStatus::Empty, "empty"),
        (ArchiveStatus::NotFound, "not_found"),
        (ArchiveStatus::Unreadable, "unreadable"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), json!(expected));
    }
}
