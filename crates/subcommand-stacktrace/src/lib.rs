//! Native crash/tombstone and Java stack-trace scanning, owned by the
//! `bugreport-insight stacktrace` subcommand. Exposed as a library so both the
//! `bugreport-insight-stacktrace` binary and other consumers can call [`scan_java_crashes`] and
//! [`scan_native_crashes`] directly.

use bugreport_insight_core::{
    error::Error,
    model::{Evidence, Finding},
    scanners::{MAX_EVIDENCE_PER_FINDING, lookahead, unique_sample},
    text::bounded,
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use zip::ZipArchive;

const MAX_TOMBSTONE_BYTES: usize = 4 * 1024 * 1024;

// https://source.android.com/docs/core/tests/debug/read-bug-reports: a fatal Java exception is
// logged by `AndroidRuntime` as `FATAL EXCEPTION: <thread>`, `Process: <pkg>, PID: <pid>`, then
// the exception class/message.
pub fn scan_java_crashes(source: &str, report: &str) -> Vec<Finding> {
    let lines: Vec<&str> = report.lines().collect();
    let mut evidence = Vec::new();
    let mut count = 0_usize;
    let mut packages = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.contains("FATAL EXCEPTION") {
            continue;
        }
        count += 1;
        let window = lookahead(&lines, index, 6);
        let package = window
            .iter()
            .find_map(|l| l.split_once("Process: ").map(|(_, rest)| rest))
            .and_then(|rest| rest.split(',').next())
            .map(str::trim);
        if let Some(package) = package {
            packages.push(package.to_string());
        }
        let exception = window.iter().find_map(|l| {
            l.split_once("AndroidRuntime: ")
                .map(|(_, rest)| rest)
                .filter(|rest| {
                    rest.starts_with("java.")
                        || rest.starts_with("kotlin.")
                        || rest.starts_with("android.")
                })
        });
        if evidence.len() < MAX_EVIDENCE_PER_FINDING {
            let mut excerpt = bounded(line);
            if let Some(package) = package {
                excerpt.push_str(&format!(" — {package}"));
            }
            if let Some(exception) = exception {
                excerpt.push_str(&format!(" — {}", bounded(exception)));
            }
            evidence.push(Evidence {
                source: source.into(),
                line_start: index + 1,
                line_end: index + 1,
                excerpt,
            });
        }
    }
    if evidence.is_empty() {
        return Vec::new();
    }
    let mut metadata = BTreeMap::new();
    metadata.insert("occurrences".into(), count.to_string());
    if !packages.is_empty() {
        metadata.insert("packages".into(), unique_sample(&packages, 8));
    }
    vec![Finding {
        schema_version: 1,
        id: "java_crash".into(),
        severity: "high".into(),
        title: "Uncaught Java exception".into(),
        summary: format!(
            "{count} fatal Java exception(s) were recorded via AndroidRuntime; each one terminated its process."
        ),
        evidence,
        metadata,
    }]
}

/// A tombstone file's filename is exactly `tombstone_<digits>` (the `.pb` sibling is protobuf,
/// not text, and is skipped).
pub fn is_text_tombstone(name: &str) -> bool {
    Path::new(name)
        .file_name()
        .and_then(|f| f.to_str())
        .and_then(|f| f.strip_prefix("tombstone_"))
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

pub fn parse_hex_addr(value: &str) -> Option<u64> {
    let trimmed = value.trim().trim_start_matches("0x");
    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(trimmed, 16).ok()
}

// https://source.android.com/docs/core/tests/debug/native-crash: classify a tombstone's crash
// subtype from its signal, cause line, and abort message.
pub fn classify_native_crash(
    signal: &str,
    fault_addr: &str,
    cause: Option<&str>,
    abort_message: Option<&str>,
    text: &str,
) -> String {
    if let Some(message) = abort_message {
        if message.starts_with("FORTIFY:") {
            return "fortify_failure".into();
        }
        if message.eq_ignore_ascii_case("stack corruption detected") {
            return "stack_corruption".into();
        }
        if message.to_ascii_lowercase().contains("file descriptor") {
            return "fdsan_error".into();
        }
    }
    if signal == "SIGSYS" {
        return "seccomp_violation".into();
    }
    if let Some(cause) = cause {
        if cause.contains("execute-only") {
            return "execute_only_memory_violation".into();
        }
        if cause.contains("seccomp") {
            return "seccomp_violation".into();
        }
    }
    if text.contains("__stack_chk_fail") {
        return "stack_corruption".into();
    }
    if text.contains("fdsan_error") {
        return "fdsan_error".into();
    }
    if signal == "SIGABRT" {
        return "abort".into();
    }
    if signal == "SIGSEGV" {
        if let Some(addr) = parse_hex_addr(fault_addr)
            && addr < 0x1000
        {
            return "null_pointer_dereference".into();
        }
        return "segmentation_fault".into();
    }
    format!("native_crash_{}", signal.to_ascii_lowercase())
}

pub struct NativeCrash {
    pub process: String,
    pub signal: String,
    pub crash_type: String,
    pub abort_message: Option<String>,
    pub line: usize,
    pub excerpt: String,
}

pub fn parse_tombstone(text: &str) -> Option<NativeCrash> {
    let lines: Vec<&str> = text.lines().collect();
    let process = lines.iter().find_map(|line| {
        let (_, rest) = line.split_once(">>> ")?;
        let (name, _) = rest.split_once(" <<<")?;
        Some(name.trim().to_string())
    })?;
    let (signal_index, signal_line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.starts_with("signal "))?;
    let signal = signal_line
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(name, _)| name)
        .unwrap_or("UNKNOWN");
    let fault_addr = signal_line
        .rsplit_once("fault addr ")
        .map(|(_, addr)| addr.trim())
        .unwrap_or("");
    let cause = lines
        .get(signal_index + 1)
        .filter(|line| line.starts_with("Cause:"))
        .copied();
    let abort_message = lines
        .iter()
        .find(|line| line.starts_with("Abort message:"))
        .map(|line| {
            line.trim_start_matches("Abort message:")
                .trim()
                .trim_matches('\'')
                .to_string()
        });
    let crash_type =
        classify_native_crash(signal, fault_addr, cause, abort_message.as_deref(), text);
    Some(NativeCrash {
        process,
        signal: signal.to_string(),
        crash_type,
        abort_message,
        line: signal_index + 1,
        excerpt: bounded(signal_line),
    })
}

// https://source.android.com/docs/core/tests/debug/native-crash: tombstones under
// `FS/data/tombstones/tombstone_NN` name the crashing process, the delivered signal, and (for
// aborts) a human-readable message; this classifies each into a crash subtype.
pub fn scan_native_crashes(
    archive: &mut ZipArchive<BufReader<File>>,
) -> Result<Vec<Finding>, Error> {
    let mut tombstone_names = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if !entry.is_dir() && is_text_tombstone(entry.name()) {
            tombstone_names.push(entry.name().to_owned());
        }
    }
    tombstone_names.sort();

    let mut evidence = Vec::new();
    let mut crash_types: BTreeMap<String, usize> = BTreeMap::new();
    for name in &tombstone_names {
        let entry = archive.by_name(name)?;
        let mut bytes = Vec::new();
        entry
            .take((MAX_TOMBSTONE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_TOMBSTONE_BYTES {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let Some(crash) = parse_tombstone(&text) else {
            continue;
        };
        *crash_types.entry(crash.crash_type.clone()).or_insert(0) += 1;
        if evidence.len() < MAX_EVIDENCE_PER_FINDING {
            let message = crash
                .abort_message
                .as_deref()
                .map(|message| format!(": {}", bounded(message)))
                .unwrap_or_default();
            evidence.push(Evidence {
                source: name.clone(),
                line_start: crash.line,
                line_end: crash.line,
                excerpt: format!(
                    "{} — {} (signal {}{message}) — {}",
                    crash.process, crash.crash_type, crash.signal, crash.excerpt
                ),
            });
        }
    }
    if evidence.is_empty() {
        return Ok(Vec::new());
    }
    let mut metadata = BTreeMap::new();
    metadata.insert("tombstone_count".into(), tombstone_names.len().to_string());
    for (crash_type, count) in &crash_types {
        metadata.insert(format!("crash_type.{crash_type}"), count.to_string());
    }
    Ok(vec![Finding {
        schema_version: 1,
        id: "native_crash".into(),
        severity: "high".into(),
        title: "Native crash tombstones".into(),
        summary: format!(
            "{} tombstone(s) were captured; each names the crashing process, signal, and a classified crash subtype (abort, null-pointer dereference, FORTIFY failure, stack corruption, seccomp violation, execute-only memory access, or fdsan error).",
            tombstone_names.len()
        ),
        evidence,
        metadata,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_fault_addresses() {
        assert_eq!(parse_hex_addr("0x5f2ced24a8"), Some(0x5f2ced24a8));
        assert_eq!(parse_hex_addr("0000000000000042"), Some(0x42));
        assert_eq!(parse_hex_addr("--------"), None);
        assert_eq!(parse_hex_addr(""), None);
    }

    #[test]
    fn classifies_native_crash_subtypes() {
        assert_eq!(
            classify_native_crash(
                "SIGABRT",
                "--------",
                None,
                Some("stack corruption detected"),
                ""
            ),
            "stack_corruption"
        );
        assert_eq!(
            classify_native_crash(
                "SIGABRT",
                "--------",
                None,
                Some("FORTIFY: memcpy prevented"),
                ""
            ),
            "fortify_failure"
        );
        assert_eq!(
            classify_native_crash("SIGSYS", "--------", None, None, ""),
            "seccomp_violation"
        );
        assert_eq!(
            classify_native_crash("SIGSEGV", "0x0000000000000000", None, None, ""),
            "null_pointer_dereference"
        );
        assert_eq!(
            classify_native_crash("SIGSEGV", "0x00000078abcdef00", None, None, ""),
            "segmentation_fault"
        );
        assert_eq!(
            classify_native_crash("SIGABRT", "--------", None, None, ""),
            "abort"
        );
    }

    #[test]
    fn parses_tombstone_signal_and_process() {
        let text = "pid: 9452, tid: 9483, name: bt_stack_manage  >>> com.google.android.bluetooth <<<\n\
                     signal 6 (SIGABRT), code -1 (SI_QUEUE), fault addr --------\n\
                     Abort message: 'boom'\n";
        let crash = parse_tombstone(text).expect("tombstone should parse");
        assert_eq!(crash.process, "com.google.android.bluetooth");
        assert_eq!(crash.signal, "SIGABRT");
        assert_eq!(crash.crash_type, "abort");
        assert_eq!(crash.abort_message.as_deref(), Some("boom"));
    }

    #[test]
    fn scans_java_crashes_with_package_and_exception() {
        let report = "09-24 00:27:01.751 1000 1116 1116 E AndroidRuntime: FATAL EXCEPTION: main\n\
                       09-24 00:27:01.751 1000 1116 1116 E AndroidRuntime: Process: com.example.app, PID: 1116\n\
                       09-24 00:27:01.751 1000 1116 1116 E AndroidRuntime: java.lang.IllegalArgumentException\n";
        let findings = scan_java_crashes("report.txt", report);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].metadata["occurrences"], "1");
        assert!(findings[0].evidence[0].excerpt.contains("com.example.app"));
        assert!(
            findings[0].evidence[0]
                .excerpt
                .contains("java.lang.IllegalArgumentException")
        );
    }

    #[test]
    fn is_text_tombstone_excludes_protobuf_sibling() {
        assert!(is_text_tombstone("FS/data/tombstones/tombstone_00"));
        assert!(!is_text_tombstone("FS/data/tombstones/tombstone_00.pb"));
        assert!(!is_text_tombstone("FS/data/tombstones/other"));
    }
}
