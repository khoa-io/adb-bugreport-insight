//! Bounded human triage rendering; JSON remains the detailed evidence record.
use serde_json::Value;
use std::collections::BTreeSet;

fn cell(v: &Value) -> String {
    let s = v.as_str().map(str::to_string).unwrap_or_else(|| {
        if v.is_null() {
            "unavailable".into()
        } else {
            v.to_string()
        }
    });
    let s = s
        .replace(['\n', '\r', '\0'], " ")
        .replace('|', "\\|")
        .replace('`', "'")
        .replace(['<', '>'], "");
    let mut end = s.len().min(240);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    if end < s.len() {
        format!("{}…", &s[..end])
    } else {
        s
    }
}
fn severity(s: &str) -> u8 {
    match s {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}
pub fn readme(artifacts: &[Value]) -> String {
    let system = &artifacts[0];
    let source = &system["source"];
    let capture = &system["capture"];
    let mut rows = Vec::new();
    rows.push(format!(
        "# bugreport-insight — {}",
        cell(&source["device"]["product_name"])
    ));
    rows.push(String::new());
    rows.push(format!(
        "Archive: `{}` · format `{}` · build `{}`",
        cell(&source["archive_name"]),
        cell(&source["format_version"]),
        cell(&source["device"]["build_display_id"])
    ));
    rows.push(format!("SHA-256: `{}`", cell(&source["archive_sha256"])));
    rows.push(String::new());
    rows.push("## 2. Capture context".into());
    rows.push(format!(
        "Captured: `{}` · uptime seconds: `{}` · bounds: `{}`",
        cell(&capture["captured_at"]),
        cell(&capture["uptime_at_capture_seconds"]),
        cell(&capture["uptime"]["bounds_seconds"])
    ));
    rows.push(format!(
        "Log window: `{}` → `{}` ({} seconds)",
        cell(&capture["log_window"]["first_line_at"]),
        cell(&capture["log_window"]["last_line_at"]),
        cell(&capture["log_window"]["duration_seconds"])
    ));
    let mut findings: Vec<_> = artifacts
        .iter()
        .filter_map(|a| a["findings"].as_array())
        .flatten()
        .collect();
    findings.sort_by_key(|f| {
        (
            std::cmp::Reverse(severity(f["severity"].as_str().unwrap_or("info"))),
            f["provenance"]["age_seconds_at_capture"]
                .as_i64()
                .unwrap_or(i64::MAX),
        )
    });
    let current: Vec<_> = findings
        .iter()
        .filter(|f| {
            matches!(
                f["provenance"]["freshness"].as_str(),
                Some("now" | "in_window")
            )
        })
        .collect();
    let high = current
        .iter()
        .filter(|f| severity(f["severity"].as_str().unwrap_or("info")) >= 3)
        .count();
    let stale = findings
        .iter()
        .filter(|f| f["provenance"]["freshness"] == "stale")
        .count();
    let undated = findings
        .iter()
        .filter(|f| f["provenance"]["freshness"] == "undated")
        .count();
    rows.push(String::new());
    rows.push("## 3. Verdict".into());
    if capture["is_cold_boot"] == true {
        rows.push("### ⚠ This archive is a cold boot".into());
        rows.push(format!("The uptime bounds are `{}` seconds; this archive has no demonstrated steady-state window. Boot-time churn alone is not a defect, and absence here is weak evidence of absence on the device.",cell(&capture["uptime"]["bounds_seconds"])));
    }
    if current.is_empty() {
        rows.push(format!("No now/in_window findings. This is not a clean bill of health: {stale} findings are stale and {undated} are undated; see section 5."));
    } else if high > 0 {
        let age = current
            .iter()
            .filter(|f| severity(f["severity"].as_str().unwrap_or("info")) >= 3)
            .filter_map(|f| f["provenance"]["age_seconds_at_capture"].as_i64())
            .max()
            .unwrap_or(0);
        rows.push(format!(
            "{high} current high/critical findings; oldest is {age} seconds old."
        ));
    } else {
        let worst = current
            .iter()
            .map(|f| f["severity"].as_str().unwrap_or("info"))
            .max_by_key(|s| severity(s))
            .unwrap_or("info");
        rows.push(format!(
            "{} current findings, none above {worst}.",
            current.len()
        ));
    }
    rows.push(String::new());
    rows.push("## 4. Top findings".into());
    rows.push("| Finding | Severity | Freshness | Subject | Metrics |".into());
    rows.push("| --- | --- | --- | --- | --- |".into());
    for f in findings.iter().take(10) {
        rows.push(format!(
            "| {}: {} | {} | {} | {} | {} |",
            cell(&f["id"]),
            cell(&f["kind"]),
            cell(&f["severity"]),
            cell(&f["provenance"]["freshness"]),
            cell(&f["attributes"]["process"]),
            cell(&f["metrics"])
        ));
    }
    if findings.is_empty() {
        rows.push("| None validated; inspect coverage and gaps | — | — | — | — |".into());
    }
    rows.push(String::new());
    rows.push("## 5. Freshness ledger".into());
    rows.push("| Artifact | Now | In window | Stale | Undated |".into());
    rows.push("| --- | --- | --- | --- | --- |".into());
    for a in artifacts {
        let f = &a["freshness_summary"];
        rows.push(format!(
            "| {}.json | {} | {} | {} | {} |",
            cell(&a["artifact"]),
            f["now"],
            f["in_window"],
            f["stale"],
            f["undated"]
        ));
    }
    rows.push("Each finding and retained record has its own provenance; collection order is not execution-time order.".into());
    rows.push(String::new());
    rows.push("## 6. Noise register".into());
    rows.push("| Artifact / search | Raw | Validated | Suppressed |".into());
    rows.push("| --- | --- | --- | --- |".into());
    for a in artifacts {
        if let Some(counts) = a["counts"].as_object() {
            for (key, c) in counts {
                rows.push(format!(
                    "| {} / {} | {} | {} | {} |",
                    cell(&a["artifact"]),
                    key,
                    c["raw_count"],
                    c["matched_evidence"],
                    c["suppressed_noise"]
                ));
            }
        }
    }
    rows.push("Validated counts are retained records within each channel's coverage; raw keyword counts are not incident totals.".into());
    rows.push(String::new());
    rows.push("## 7. Evidence index".into());
    rows.push("| Finding | ZIP entry | Lines | Byte offset |".into());
    rows.push("| --- | --- | --- | --- |".into());
    let mut seen = BTreeSet::new();
    let mut shown = 0;
    for f in &findings {
        let e = &f["evidence"];
        let key = (
            cell(&e["archive_path"]),
            e["locator"]["line_start"].to_string(),
            e["locator"]["line_end"].to_string(),
        );
        if seen.insert(key) && shown < 90 {
            rows.push(format!(
                "| {} | {} | {}–{} | {} |",
                cell(&f["id"]),
                cell(&e["archive_path"]),
                e["locator"]["line_start"],
                e["locator"]["line_end"],
                e["locator"]["byte_offset"]
            ));
            shown += 1;
        }
    }
    if seen.len() > shown {
        rows.push(format!(
            "{} additional locators remain in JSON; this index is capped.",
            seen.len() - shown
        ));
    }
    rows.push(String::new());
    rows.push("## 8. What this archive cannot tell you".into());
    for a in artifacts {
        let gaps = a["evidence_gaps"].as_array().map(Vec::len).unwrap_or(0);
        rows.push(format!(
            "- {}.json: {gaps} declared gaps; unavailable sections: {}.",
            cell(&a["artifact"]),
            cell(&a["scope"]["sections_not_found"])
        ));
    }
    rows.push("Read the JSON evidence_gaps, negative_evidence, and notes before interpreting an empty channel. CPU utilization and contention have different windows; swap growth alone does not establish collector causation or thrashing.".into());
    rows.push(String::new());
    rows.push("## 9. Reproduction".into());
    rows.push(format!(
        "Generator: {} {} · started {} · finished {}",
        cell(&system["generator"]["name"]),
        cell(&system["generator"]["version"]),
        cell(&system["generator"]["run_started_at"]),
        cell(&system["generator"]["run_finished_at"])
    ));
    rows.push(format!("Run `bugreport-insight --insight BUGREPORT` with the archive whose SHA-256 is `{}`; BUGREPORT is the original input path.",cell(&source["archive_sha256"])));
    rows.push("The command reads the ZIP in place and refuses to replace an existing sibling insight directory.".into());
    rows.push(String::new());
    rows.join("\n")
}
