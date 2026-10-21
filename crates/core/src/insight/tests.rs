use super::*;
use bugreport_insight_test_fixtures::{ZipBuilder, insight_archive, scratch_dir};
use std::sync::atomic::{AtomicU64, Ordering};
fn output() -> Vec<Value> {
    let archive = Archive::open(insight_archive()).unwrap();
    generate_insight(&archive)
        .unwrap()
        .into_iter()
        .filter(|f| f.name.ends_with(".json"))
        .map(|f| serde_json::from_slice(&f.contents).unwrap())
        .collect()
}
fn fixture(text: &str) -> Archive {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = scratch_dir().join(format!(
        "insight-unit-{}-{}.zip",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(
        &path,
        ZipBuilder::new()
            .add("main_entry.txt", b"bugreport.txt")
            .add("bugreport.txt", text.as_bytes())
            .finish(),
    )
    .unwrap();
    Archive::open(path).unwrap()
}

#[test]
fn all_domains_emit_evidence_and_readme_has_nine_ordered_sections() {
    let archive = Archive::open(insight_archive()).unwrap();
    let files = generate_insight(&archive).unwrap();
    assert_eq!(files.len(), 7);
    let readme = String::from_utf8(files[0].contents.clone()).unwrap();
    assert!(readme.lines().count() <= 400);
    for n in 2..=9 {
        assert!(readme.contains(&format!("## {n}.")));
    }
    assert!(readme.contains("This archive is a cold boot"));
    let values = output();
    validate::artifacts(&values).unwrap();
    for a in values {
        assert!(
            !a["findings"].as_array().unwrap().is_empty(),
            "{}",
            a["artifact"]
        );
    }
}
#[test]
fn crash_records_are_deduplicated_and_native_history_stays_separate() {
    let a = output();
    let c = &a[3];
    assert_eq!(c["java_crash_incidents"].as_array().unwrap().len(), 5);
    assert_eq!(c["counts"]["fatal_exception_hits"]["raw_count"], 6);
    assert_eq!(c["counts"]["fatal_exception_hits"]["matched_evidence"], 5);
    assert_eq!(c["java_crash_groups"][0]["classification"], "crash_loop");
    assert_eq!(c["java_crash_groups"][0]["duplicate_hits"], 1);
    assert_eq!(c["native_crash_incidents"].as_array().unwrap().len(), 1);
    let native = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["attributes"]["language"] == "native")
        .unwrap();
    assert_eq!(native["severity"], "medium");
    assert_eq!(native["unconstrained_severity"], "high");
    assert_eq!(native["provenance"]["freshness"], "stale");
}
#[test]
fn retained_anr_files_do_not_create_current_incidents() {
    let a = output();
    let anr = &a[2];
    assert_eq!(anr["anr_incidents"].as_array().unwrap().len(), 1);
    assert_eq!(
        anr["anr_incidents"][0]["evidence_channels"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(anr["anr_incidents"][0]["confidence"], "inferred");
    assert_eq!(anr["trace_files"]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        anr["trace_files"]["files"][0]["provenance"]["freshness"],
        "stale"
    );
    assert_eq!(
        anr["trace_files"]["files"][0]["processes"][0]["subject_inference"]["subject_inferred"],
        true
    );
}
#[test]
fn memory_sections_without_summary_metrics_keep_finding_provenance() {
    for contents in [
        "",
        "OEM memory summary unavailable\n",
        "*** SERVICE meminfo DUMP TIMEOUT EXPIRED ***\n",
        "version,1\nram,10000,2000\n",
    ] {
        let archive = fixture(&format!("------ SERVICE meminfo ------\n{contents}"));
        let files = generate_insight(&archive).unwrap();
        let memory: Value = serde_json::from_slice(&files[5].contents).unwrap();
        let finding = memory["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["kind"] == "memory_snapshot")
            .unwrap();
        assert_eq!(finding["kind"], "memory_snapshot");
        assert_eq!(finding["severity"], "info");
        assert_eq!(finding["provenance"]["evidence_origin"], "dumpsys_meminfo");
        assert_eq!(finding["provenance"]["line_start"], 1);
        assert_eq!(finding["provenance"]["freshness"], "undated");
        assert!(finding["provenance"]["captured_at"].is_null());
        assert!(
            finding["metrics"]
                .as_object()
                .unwrap()
                .values()
                .all(Value::is_null)
        );
        assert!(memory["system"]["total_ram_kib"].is_null());
        assert!(!memory["evidence_gaps"].as_array().unwrap().is_empty());
    }
}

#[test]
fn memory_snapshots_keep_units_and_explicit_delta_edges() {
    let a = output();
    let m = &a[4];
    assert_eq!(m["system"]["total_ram_kib"], 10000);
    assert_eq!(m["system"]["free_ratio"], 0.4);
    assert_eq!(m["system"]["total_pss_kib"], 4000);
    let nodes = m["collection"]["swap_timeline"].as_array().unwrap();
    assert_eq!(nodes.len(), 4);
    for pair in nodes.windows(2) {
        assert_eq!(
            pair[1]["delta_from_previous"]["from_line_start"],
            pair[0]["line_start"]
        );
    }
    assert!(m["collection"]["page_scanning_ms_in_interval"].is_null());
    assert!(
        m["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["zswap_estimated"] == true)
    );
}
#[test]
fn cpu_windows_cumulative_counters_and_sampled_contention_are_distinct() {
    let a = output();
    let c = &a[5];
    assert_eq!(c["per_cpu_core"][0]["user_time_ms"], 100);
    assert_eq!(c["per_cpu_core"][0]["busy_pct_since_boot"], 30.);
    let cpu = c["utilization"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["channel"] == "dumpsys_cpuinfo")
        .unwrap();
    assert_eq!(cpu["window"]["duration_ms"], 2000);
    assert_eq!(cpu["provenance"]["collector_perturbed"], false);
    let top = c["utilization"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["channel"] == "top")
        .unwrap();
    assert!(top["total_pct"].is_null());
    assert_eq!(top["configured_cpus"], 2);
    assert_eq!(c["contention"]["art_java_monitor"]["wait_ms_max"], 1500.);
    assert_eq!(c["contention"]["dvm_lock_sample"]["sampled_event_count"], 1);
    assert!(c["contention"]["art_java_monitor"]["threshold_ms"].is_null());
    assert_eq!(c["stall_signals"]["proc_churn"]["rate_valid"], false);
    assert!(c["stall_signals"]["proc_churn"]["rate_per_s"].is_null());
}
#[test]
fn app_install_state_and_window_local_process_identity_are_explicit() {
    let a = output();
    let app = &a[1];
    assert_eq!(app["inventory"]["counts"]["packages"], 1);
    assert_eq!(
        app["inventory"]["packages"][0]["users"][0]["installed"],
        false
    );
    assert!(app["inventory"]["packages"][0]["users"][0]["first_install_time"].is_null());
    assert!(app["inventory"]["packages"][0]["users"][0]["ce_data_inode"].is_null());
    let p = &app["inventory"]["processes"][0];
    assert_eq!(p["uid"], 1010001);
    assert_eq!(p["user_id"], 10);
    assert_eq!(p["start_seq"], 7);
    assert!(p["process_key"].as_str().unwrap().contains("#1010001@"));
    let coverage = app["channels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "am_proc_event_log")
        .unwrap();
    assert!(coverage["coverage_ratio"].as_f64().unwrap() < 1.);
}
#[test]
fn unknown_clocks_and_emulator_properties_remain_null() {
    let archive = fixture(
        "== dumpstate: 2026-10-17 12:00:00\n------ SYSTEM LOG ------\n10-17 12:00:00.100 system 10 10 I test: event\n",
    );
    let values: Vec<Value> = generate_insight(&archive)
        .unwrap()
        .iter()
        .skip(1)
        .map(|f| serde_json::from_slice(&f.contents).unwrap())
        .collect();
    assert!(values[0]["capture"]["captured_at"].is_null());
    assert!(values[0]["capture"]["is_cold_boot"].is_null());
    assert!(values[0]["device"]["is_emulator"].is_null());
}
#[test]
fn logcat_year_boundary_is_resolved_without_host_clock() {
    let capture = parse_clock("2026-01-01T00:00:05+00:00", None).unwrap();
    assert!(
        stamp(&common::log_clock("12-31 23:59:59.000", Some(&capture)).unwrap())
            .starts_with("2025-12-31")
    );
    assert!(common::log_clock("12-31 23:59:59.000", None).is_none());
}
#[test]
fn uid_names_and_user_scoped_uids_are_decoded() {
    assert_eq!(uid("system"), Some(1000));
    assert_eq!(uid("u10a1"), Some(1010001));
    assert_eq!(uid("u10i7"), Some(1099007));
    assert_eq!(uid("unsupported_name"), None);
}
#[test]
fn excerpts_preserve_utf8_and_report_nul_incompleteness() {
    let text = "界".repeat(200);
    let (e, cut, complete) = common::excerpt(&text);
    assert!(e.len() <= 512);
    assert!(cut);
    assert!(!complete);
    assert_eq!(e.len() % 3, 0);
    assert!(!common::excerpt("a\0b").2);
}
#[test]
fn validator_rejects_corrupt_counts_and_freshness() {
    let mut a = output();
    a[3]["counts"]["fatal_exception_hits"]["raw_count"] = json!(0);
    assert!(validate::artifacts(&a).is_err());
    let mut a = output();
    a[3]["findings"][0]["severity"] = json!("critical");
    a[3]["findings"][0]["provenance"]["freshness"] = json!("undated");
    assert!(validate::artifacts(&a).is_err());
    let mut a = output();
    a[1]["source"]["archive_size_bytes"] = json!(1);
    assert!(validate::artifacts(&a).is_err());
}

#[test]
fn event_only_crash_stays_reported_without_inflating_primary_count() {
    let archive = fixture(
        "== dumpstate: 2026-10-17 12:00:00+00:00\n------ EVENT LOG (logcat) ------\n10-17 11:59:59.000 system 100 100 I am_crash: [0,123,example.app,0,java.lang.Error,failure,Example.java,42]\n",
    );
    let files = generate_insight(&archive).unwrap();
    let crashes: Value = serde_json::from_slice(
        &files
            .iter()
            .find(|f| f.name == "crashes.json")
            .unwrap()
            .contents,
    )
    .unwrap();
    assert_eq!(crashes["java_crash_incidents"], json!([]));
    assert_eq!(crashes["event_log_reports"].as_array().unwrap().len(), 1);
    let finding = crashes["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == "reported_crash_event")
        .unwrap();
    assert_eq!(finding["confidence"], "reported");
    assert_eq!(finding["severity"], "high");
}
#[test]
fn newline_count_excludes_unterminated_tail() {
    let archive = fixture("first\nlast");
    let r = Report::load(&archive).unwrap();
    assert_eq!(r.newline_count, Some(1));
}

#[test]
fn table_separators_do_not_replace_service_sections() {
    let archive = fixture(
        "------ DUMPSYS ------\nDUMP OF SERVICE HIGH meminfo:\n------   ------   ------\nTotal RAM: 1000K (status normal)\n",
    );
    let r = Report::load(&archive).unwrap();
    assert_eq!(r.title(3), "DUMP OF SERVICE meminfo");
}

#[test]
fn current_window_uses_first_and_last_lines_not_embedded_clock_minimum() {
    let archive = fixture(
        "== dumpstate: 2026-10-17 12:00:00+00:00\n------ SYSTEM LOG ------\n10-17 11:59:55.000 root 1 1 I init: first\n01-01 00:00:00.000 root 1 1 I init: device clock before correction\n10-17 12:00:10.000 root 1 1 I init: last\n",
    );
    let r = Report::load(&archive).unwrap();
    assert_eq!(r.window()["duration_seconds"], 15.);
}

#[test]
fn configurable_crash_thresholds_are_applied_and_published() {
    let archive = Archive::open(insight_archive()).unwrap();
    let options = InsightOptions {
        crash_thresholds: CrashThresholds {
            repeat_count: 10,
            ..Default::default()
        },
    };
    let files = generate_insight_with_options(&archive, &options).unwrap();
    let crashes: Value = serde_json::from_slice(
        &files
            .iter()
            .find(|f| f.name == "crashes.json")
            .unwrap()
            .contents,
    )
    .unwrap();
    assert_eq!(
        crashes["java_crash_groups"][0]["classification"],
        "recurring"
    );
    assert_eq!(
        crashes["java_crash_groups"][0]["classification_thresholds"]["repeat_count"],
        10
    );
}
#[test]
fn prior_digest_decline_does_not_remove_required_artifact_identity() {
    let archive = Archive::open(insight_archive()).unwrap();
    archive.identity(false).unwrap();
    assert!(generate_insight(&archive).is_ok());
}
#[test]
fn hostile_uid_and_uptime_numbers_do_not_overflow() {
    assert!(uid("u9223372036854775807a1").is_none());
    let archive = fixture(
        "== dumpstate: 2026-10-17 12:00:00+00:00\nUptime: up 9223372036854775807 weeks, 0 days, 0 hours, 0 minutes\n",
    );
    let values = generate_insight(&archive).unwrap();
    let system: Value = serde_json::from_slice(&values[1].contents).unwrap();
    assert!(system["capture"]["uptime_at_capture_seconds"].is_null());
}
#[test]
fn section_budget_failure_does_not_emit_partial_analysis() {
    let archive = fixture(&"------ SECTION ------\n".repeat(20001));
    assert!(
        generate_insight(&archive)
            .unwrap_err()
            .to_string()
            .contains("20,000")
    );
}
#[test]
fn raw_searches_include_outer_side_text_entries() {
    let path = scratch_dir().join(format!("side-search-{}.zip", std::process::id()));
    std::fs::write(
        &path,
        ZipBuilder::new()
            .add("main_entry.txt", b"bugreport.txt")
            .add("bugreport.txt", b"header\n")
            .add("FS/side.txt", b"watchdog symbol\n")
            .finish(),
    )
    .unwrap();
    let files = generate_insight(&Archive::open(path).unwrap()).unwrap();
    let a: Value = serde_json::from_slice(&files[1].contents).unwrap();
    assert_eq!(a["counts"]["watchdog_hits"]["raw_count"], 1);
    assert!(
        a["counts"]["watchdog_hits"]["regions"][0]["archive_path"]
            .as_str()
            .unwrap()
            .ends_with("side.txt")
    );
}

#[test]
fn identical_crashes_in_old_and_current_windows_form_separate_groups() {
    let mut text = String::from(
        "== dumpstate: 2026-10-17 12:00:00+00:00\n------ SYSTEM LOG (logcat) ------\n",
    );
    for (clock, pid) in [("10-16 12:00:00.000", 10), ("10-17 12:00:00.000", 11)] {
        for message in [
            "FATAL EXCEPTION: main".to_string(),
            format!("Process: pkg.app, PID: {pid}"),
            "java.lang.Error: failure".into(),
            "    at pkg.Main.run(Main.java:1)".into(),
        ] {
            text.push_str(&format!(
                "{clock} system {pid} {pid} E AndroidRuntime: {message}\n"
            ));
        }
    }
    let files = generate_insight(&fixture(&text)).unwrap();
    let crashes: Value = serde_json::from_slice(
        &files
            .iter()
            .find(|f| f.name == "crashes.json")
            .unwrap()
            .contents,
    )
    .unwrap();
    let groups = crashes["java_crash_groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|g| g["repeat_count"] == 1));
    assert_ne!(groups[0]["group_id"], groups[1]["group_id"]);
    assert!(
        groups
            .iter()
            .any(|g| g["provenance"]["freshness"] == "stale")
    );
    assert!(groups.iter().any(|g| matches!(
        g["provenance"]["freshness"].as_str(),
        Some("now" | "in_window")
    )));
}

#[test]
fn invalid_version_marker_does_not_become_format_zero() {
    use std::io::Read;
    let mut source = zip::ZipArchive::new(std::fs::File::open(insight_archive()).unwrap()).unwrap();
    let mut main = String::new();
    source
        .by_name("bugreport.txt")
        .unwrap()
        .read_to_string(&mut main)
        .unwrap();
    for invalid in [false, true] {
        let path = scratch_dir().join(format!("insight-marker-validity-{invalid}.zip"));
        let mut builder = ZipBuilder::new()
            .add("main_entry.txt", b"bugreport.txt")
            .add("bugreport.txt", main.as_bytes());
        if invalid {
            builder = builder.add("version.txt", b"2\xff");
        }
        std::fs::write(&path, builder.finish()).unwrap();
        let files = generate_insight(&Archive::open(path).unwrap()).unwrap();
        let system: Value = serde_json::from_slice(
            &files
                .iter()
                .find(|f| f.name == "system.json")
                .unwrap()
                .contents,
        )
        .unwrap();
        let finding = system["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["kind"] == "archive_identity")
            .unwrap();
        if invalid {
            assert!(finding["attributes"].get("format_version").is_none());
            assert_eq!(
                finding["attributes"]["format_version_marker_status"],
                json!("invalid_utf8")
            );
            assert!(
                system["evidence_gaps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["field"] == "source.format_version")
            );
        } else {
            assert_eq!(finding["attributes"]["format_version"], json!("0"));
        }
    }
}
