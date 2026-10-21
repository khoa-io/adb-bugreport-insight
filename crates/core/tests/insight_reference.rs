//! SWE.6 acceptance against the hash-identified, untracked reference archive.
use bugreport_insight_core::{Archive, insight::generate_insight};
use serde_json::Value;
#[test]
fn reference_report_produces_measured_domain_artifacts() {
    let path = std::env::var_os("BUGREPORT_ARCHIVE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tmp/reference-archive.zip")
        });
    if !path.exists() {
        eprintln!(
            "reference insight acceptance skipped: BUGREPORT_ARCHIVE or tmp/reference-archive.zip is required"
        );
        return;
    }
    let archive = Archive::open(path).unwrap();
    assert_eq!(
        archive.identity(true).unwrap().sha256.as_deref(),
        Some("10ec0b47dd7934462b6bbaa1e02e5bf47698f0f22368c93e37b2de020239df92")
    );
    let files = generate_insight(&archive).unwrap();
    assert_eq!(files.len(), 7);
    assert!(String::from_utf8_lossy(&files[0].contents).lines().count() <= 400);
    let a: Vec<Value> = files
        .iter()
        .skip(1)
        .map(|f| serde_json::from_slice(&f.contents).unwrap())
        .collect();
    assert_eq!(a[0]["source"]["main_entry_line_count"], 682951);
    assert!(
        (a[0]["capture"]["log_window"]["duration_seconds"]
            .as_f64()
            .unwrap()
            - 64.662)
            .abs()
            < 0.00001
    );
    assert_eq!(a[0]["capture"]["is_cold_boot"], true);
    assert_eq!(a[0]["users"]["list"].as_array().unwrap().len(), 2);
    assert_eq!(a[0]["displays"].as_array().unwrap().len(), 2);
    assert!(a[0]["hal"]["hidl_services"].as_array().unwrap().len() >= 29);
    assert_eq!(a[1]["inventory"]["counts"]["packages"], 282);
    assert_eq!(a[1]["inventory"]["counts"]["process_records"], 83);
    assert_eq!(
        a[1]["inventory"]["counts"]["package_user_state_blocks"],
        564
    );
    assert!(a[1]["channels"][2]["coverage_ratio"].as_f64().unwrap() > 0.3);
    assert!(a[1]["channels"][2]["coverage_ratio"].as_f64().unwrap() < 0.4);
    assert_eq!(a[2]["anr_incidents"].as_array().unwrap().len(), 0);
    assert_eq!(a[2]["trace_files"]["files"].as_array().unwrap().len(), 65);
    assert_eq!(a[2]["last_anr_trace"]["process_count"], 31);
    assert_eq!(a[2]["last_anr_trace"]["provenance"]["freshness"], "stale");
    assert_eq!(a[3]["java_crash_incidents"].as_array().unwrap().len(), 271);
    assert_eq!(a[3]["java_crash_groups"][0]["classification"], "crash_loop");
    assert_eq!(a[3]["native_crash_incidents"].as_array().unwrap().len(), 50);
    let mut repeats: Vec<_> = a[3]["native_crash_groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["repeat_count"].as_u64().unwrap())
        .collect();
    repeats.sort();
    assert_eq!(repeats, [3, 47]);
    assert_eq!(a[4]["system"]["total_ram_kib"], 3047508);
    assert_eq!(a[4]["system"]["total_pss_kib"], 2007345);
    assert_eq!(a[4]["procstats"].as_array().unwrap().len(), 3);
    assert_eq!(
        a[4]["collection"]["swap_timeline"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(a[5]["per_cpu_core"].as_array().unwrap().len(), 4);
    let cpu = a[5]["utilization"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["channel"] == "dumpsys_cpuinfo")
        .unwrap();
    assert_eq!(cpu["window"]["duration_ms"], 1772);
    assert_eq!(cpu["provenance"]["collector_perturbed"], false);
    assert_eq!(
        a[5]["contention"]["dvm_lock_sample"]["sampled_event_count"],
        168
    );
}
