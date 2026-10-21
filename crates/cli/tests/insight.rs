//! SWE.5/SWE.6 real-process checks with a synthetic report covering all domains.
use bugreport_insight_test_fixtures::{ZipBuilder, insight_archive};
use serde_json::Value;
use std::{fs, process::Command};
#[test]
fn insight_cli_publishes_when_meminfo_has_no_recognized_summary() {
    let root = std::env::temp_dir().join(format!("insight-missing-memory-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("report.zip");
    let bytes = ZipBuilder::new()
        .add("main_entry.txt", b"bugreport.txt")
        .add(
            "bugreport.txt",
            b"------ SERVICE meminfo ------\nOEM output unavailable\n",
        )
        .finish();
    fs::write(&source, &bytes).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bugreport-insight"))
        .arg("--insight")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let directory = root.join(".report/insight");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 7);
    let memory: Value =
        serde_json::from_slice(&fs::read(directory.join("memory.json")).unwrap()).unwrap();
    assert!(memory["system"]["total_ram_kib"].is_null());
    let finding = memory["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == "memory_snapshot")
        .unwrap();
    assert_eq!(finding["provenance"]["line_start"], 1);
    assert_eq!(bytes, fs::read(&source).unwrap());
    assert_eq!(modified, fs::metadata(&source).unwrap().modified().unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn insight_cli_analyzes_all_domains_and_preserves_input_bytes_and_metadata() {
    let root = std::env::temp_dir().join(format!("insight-domain-process-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("report case.zip");
    fs::copy(insight_archive(), &source).unwrap();
    let before = fs::read(&source).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bugreport-insight"))
        .arg("--insight")
        .arg(&source)
        .arg("--verbose")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("analyzing"));
    let directory = root.join(".report case/insight");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 7);
    let load = |name: &str| {
        serde_json::from_slice::<Value>(&fs::read(directory.join(name)).unwrap()).unwrap()
    };
    let system = load("system.json");
    assert_eq!(system["capture"]["is_cold_boot"], true);
    let crashes = load("crashes.json");
    assert_eq!(crashes["java_crash_incidents"].as_array().unwrap().len(), 5);
    assert_eq!(
        crashes["java_crash_groups"][0]["classification"],
        "crash_loop"
    );
    assert_eq!(
        load("anr.json")["anr_incidents"].as_array().unwrap().len(),
        1
    );
    assert_eq!(load("memory.json")["system"]["total_ram_kib"], 10000);
    assert_eq!(
        load("cpu.json")["per_cpu_core"][0]["busy_pct_since_boot"],
        30.
    );
    assert_eq!(load("apps.json")["inventory"]["counts"]["packages"], 1);
    assert_eq!(before, fs::read(&source).unwrap());
    assert_eq!(modified, fs::metadata(&source).unwrap().modified().unwrap());
    let readme = fs::read_to_string(directory.join("README.md")).unwrap();
    assert!(readme.lines().count() <= 400);
    assert!(readme.contains("Top findings"));
    assert!(!readme.contains("not implemented"));
    fs::remove_dir_all(root).unwrap();
}
