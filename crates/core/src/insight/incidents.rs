//! ANR channel separation and Java/native crash grouping.
use super::{
    Artifact, InsightError,
    common::*,
    report::{Line, Log, Report, entry_lines},
    system::zip_stamp,
};
use crate::{Archive, EntryMeta};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn side_provenance(
    r: &Report,
    entry: &EntryMeta,
    line: &Line,
    origin: &str,
    time: Option<Clock>,
    base: &str,
) -> Value {
    let mut p = r.provenance(
        usize::MAX,
        origin,
        "disk_artifact",
        time,
        base,
        if base == "tombstone_header" {
            "embedded_trace_stamp"
        } else {
            "file_mtime"
        },
    );
    p["archive_path"] = json!(entry.path);
    p["line_start"] = json!(line.no);
    p["line_end"] = json!(line.no);
    p
}
fn side_evidence(entry: &EntryMeta, line: &Line) -> Value {
    let mut e = evidence(&entry.path, line.no, line.offset, &line.text, line.bytes);
    e["entry_id"] = json!(entry.id.0);
    e
}
fn trace_processes(lines: &[Line], r: &Report) -> (Vec<Value>, Option<i64>, Option<i64>) {
    let mut list = Vec::new();
    let mut current: Option<Value> = None;
    let mut dumping = None;
    let mut uptime = None;
    for l in lines {
        let t = l.text.trim();
        if let Some(c) = captures(r"^-+ dumping pid: (\d+) at (\d+)", t) {
            dumping = number(&c[1]);
            uptime = number(&c[2]);
        }
        if let Some(c) = captures(r"^-+ pid (\d+) at (.+?) -+$", t) {
            if let Some(p) = current.take() {
                list.push(p);
            }
            let pid = number(&c[1]);
            current = Some(
                json!({"pid":pid,"cmdline":null,"subject_inference":{"basis":if dumping.is_some(){Some("dumping_pid_marker")}else{None},"subject_pid":dumping,"subject_inferred":true},"stamped_at":parse_clock(&c[2],r.offset).as_ref().map(stamp).or_else(||Some(c[2].clone())),"java_thread_count":null,"native_frame_count":0,"waiter_count":0}),
            );
            continue;
        }
        if let Some(p) = current.as_mut() {
            if let Some(v) = t.strip_prefix("Cmd line:") {
                p["cmdline"] = json!(normal(v));
            }
            if let Some(c) = captures(r"^DALVIK THREADS \((\d+)\)", t) {
                p["java_thread_count"] = json!(number(&c[1]));
            }
            if t.starts_with('#') || t.starts_with("native: #") {
                p["native_frame_count"] = json!(p["native_frame_count"].as_i64().unwrap_or(0) + 1);
            }
            if t.contains("- waiting to lock") {
                p["waiter_count"] = json!(p["waiter_count"].as_i64().unwrap_or(0) + 1);
            }
        }
    }
    if let Some(p) = current {
        list.push(p);
    }
    (list, dumping, uptime)
}
fn cross_boot(mut provenance: Value, uptime: Option<i64>, capture: &Value) -> Value {
    if let Some(ms) = uptime {
        provenance["cross_boot_reference"] = json!(ms);
        if capture["uptime_at_capture_seconds"]
            .as_i64()
            .is_some_and(|s| ms > 1000 * s.max(1))
        {
            provenance["freshness"] = json!("stale");
            provenance["freshness_basis"]["basis"] =
                json!("cross-boot reference exceeds independently derived capture uptime");
        }
    }
    provenance
}
fn anr_candidate(r: &Report, log: &Log) -> Option<Value> {
    let mut v = object(
        "incident_id process pid user_id user_id_basis occurred_at activity_component reason parent_component error_id frozen app_flags_hex",
    );
    if log.tag == "am_anr" {
        let fields: Vec<_> = log
            .message
            .trim()
            .strip_prefix('[')?
            .strip_suffix(']')?
            .splitn(5, ',')
            .map(str::trim)
            .collect();
        if fields.len() < 5 {
            return None;
        }
        v["pid"] = json!(number(fields[1]));
        v["process"] = json!(fields[2]);
        v["app_flags_hex"] = json!(fields[3]);
        v["reason"] = json!(fields[4]);
        if r.prop("ro.build.version.sdk")
            .and_then(|s| number(&s))
            .or_else(|| r.header("Android SDK version").and_then(|s| number(&s)))
            == Some(35)
        {
            v["user_id"] = json!(number(fields[0]));
            v["user_id_basis"] = json!("decoded_from_payload");
        }
    } else if log.tag == "ActivityManager" && log.priority == "E" {
        let tail = log.message.strip_prefix("ANR in ")?;
        v["process"] = json!(tail.split(" (").next().unwrap_or(tail));
        v["activity_component"] = json!(between(tail, " (", ")"));
        for next in r.logs.iter().filter(|l| {
            l.index > log.index && l.index <= log.index + 30 && l.tag == log.tag && l.pid == log.pid
        }) {
            let t = next.message.trim();
            if t.starts_with("ANR in ") {
                break;
            }
            for (prefix, key) in [
                ("PID:", "pid"),
                ("Reason:", "reason"),
                ("Parent:", "parent_component"),
                ("ErrorId:", "error_id"),
                ("Frozen:", "frozen"),
            ] {
                if let Some(value) = t.strip_prefix(prefix) {
                    v[key] = match key {
                        "pid" => json!(number(value)),
                        "frozen" => json!(value.trim().parse::<bool>().ok()),
                        _ => json!(normal(value)),
                    };
                }
            }
        }
    } else {
        return None;
    }
    if v["pid"].is_null() || v["process"].is_null() {
        return None;
    }
    v["incident_id"] = json!(format!(
        "anr-{}",
        hash(&format!(
            "{}|{}|{}|{}",
            log.tag, log.raw_time, v["pid"], v["process"]
        ))
    ));
    v["occurred_at"] = json!(log.time.as_ref().map(stamp));
    v["processes"] = json!([]);
    v["trace_files"] = json!([]);
    v["evidence_channels"] = json!([{"channel":if log.tag=="am_anr"{"event_log_am_anr"}else{"logcat_anr_tag"},"join_basis":"same_logcat_line"}]);
    v["provenance"] = r.log_provenance(
        log,
        if log.tag == "am_anr" {
            "event_log_am_anr"
        } else {
            "logcat_anr_tag"
        },
    );
    v["evidence"] = r.evidence(log.index);
    Some(v)
}
pub fn analyze_anr(
    archive: &Archive,
    r: &Report,
    capture: &Value,
) -> Result<Artifact, InsightError> {
    let mut a = Artifact::new(
        "anr",
        r,
        &[
            "SYSTEM LOG",
            "EVENT LOG",
            "ANR FILES",
            "VM TRACES AT LAST ANR",
        ],
        false,
    );
    let mut files = Vec::new();
    let mut listing: BTreeMap<String, (i64, String)> = BTreeMap::new();
    let mut total_blocks = None;
    for (_, l) in r.section_lines("ANR FILES") {
        let t = l.text.trim();
        if let Some(v) = t.strip_prefix("total ") {
            total_blocks = number(v);
        }
        if let Some(c) = captures(
            r"^[-rwx]+\s+\d+\s+\S+\s+\S+\s+(\d+)\s+(\d{4}-\d\d-\d\d \d\d:\d\d)\s+(anr_\S+)$",
            t,
        ) {
            listing.insert(c[3].clone(), (number(&c[1]).unwrap_or(0), c[2].clone()));
        }
    }
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.starts_with("FS/data/anr/anr_") && !e.is_dir)
    {
        let lines = entry_lines(archive, entry.id)?;
        let (processes, _, uptime) = trace_processes(&lines, r);
        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
        let created = name
            .strip_prefix("anr_")
            .and_then(|n| parse_clock(n, r.offset));
        let own_time = processes
            .iter()
            .filter_map(|p| p["stamped_at"].as_str().and_then(|s| parse_clock(s, None)))
            .min()
            .or(created);
        let evidence = lines.first().map(|l| side_evidence(entry, l));
        let provenance = lines.first().map(|l| {
            cross_boot(
                side_provenance(r, entry, l, "anr_file_listing", own_time, "file_name"),
                uptime,
                capture,
            )
        });
        let printed = listing.get(name);
        let mtime = printed.map(|(_, s)| s.clone());
        files.push(json!({"path":entry.path,"file_name":name,"size_bytes":printed.map(|(n,_)|*n).unwrap_or(entry.uncompressed_size as i64),"mtime_local":mtime,"created_at":created.as_ref().map(stamp),"is_newest":false,"mtime_ls_minute_differs_from_filename":printed.zip(created).map(|((_,m),t)|t.format("%Y-%m-%d %H:%M").to_string()!=*m),"provenance":provenance,"evidence":evidence,"processes":processes}));
    }
    let mut last_trace = object(
        "section_header source_path file_mtime_local trace_first_stamp trace_last_stamp uptime_ms_at_dump provenance",
    );
    last_trace["process_count"] = json!(0);
    last_trace["has_dumping_pid_marker"] = json!(false);
    last_trace["processes"] = json!([]);
    last_trace["lock_contention_markers"] = json!({"waiting_to_lock":0,"held_by_thread":0});
    if let Some(section) = r.find_section("VM TRACES AT LAST ANR") {
        let slice = &r.lines[section.start..=section.end];
        let (processes, dumping, uptime) = trace_processes(slice, r);
        let stamps: Vec<_> = processes
            .iter()
            .filter_map(|p| p["stamped_at"].as_str().and_then(|s| parse_clock(s, None)))
            .collect();
        let command = section.command.as_deref().unwrap_or("");
        let (source, mtime) = command
            .split_once(": ")
            .map(|(a, b)| (Some(a), Some(b)))
            .unwrap_or((None, None));
        let time = stamps
            .iter()
            .min()
            .copied()
            .or_else(|| mtime.and_then(|s| parse_clock(s, r.offset)));
        last_trace = json!({"section_header":r.lines[section.start].text,"source_path":source,"file_mtime_local":mtime,"trace_first_stamp":stamps.iter().min().map(stamp),"trace_last_stamp":stamps.iter().max().map(stamp),"process_count":processes.len(),"has_dumping_pid_marker":dumping.is_some(),"uptime_ms_at_dump":uptime,"processes":processes,"lock_contention_markers":{"waiting_to_lock":slice.iter().filter(|l|l.text.contains("- waiting to lock")).count(),"held_by_thread":slice.iter().filter(|l|l.text.contains("held by thread")).count()},"provenance":cross_boot(r.provenance(section.start,"vm_traces_at_last_anr","disk_artifact",time,"file_section_header_mtime","embedded_trace_stamp"),uptime,capture)});
        if let Some(path) = source {
            for file in &mut files {
                file["is_newest"] =
                    json!(format!("FS{path}") == file["path"].as_str().unwrap_or(""));
            }
        }
        a.finding(super::FindingInput { kind: "retained_anr_trace", severity: "info", confidence: "confirmed", process_key: None, attributes: json!({"has_dumping_pid_marker":dumping.is_some()}), metrics: json!({"processes":last_trace["process_count"],"uptime_ms_at_dump":uptime,"retained_files":files.len()}), evidence: r.evidence(section.start), provenance: last_trace["provenance"].clone() });
    }
    let mut current: Vec<Value> = Vec::new();
    let mut historic = Vec::new();
    let mut seen = BTreeSet::new();
    for log in &r.logs {
        let Some(candidate) = anr_candidate(r, log) else {
            continue;
        };
        let key = (
            candidate["process"].to_string(),
            candidate["pid"].to_string(),
            log.raw_time.clone(),
        );
        if !seen.insert(key) {
            continue;
        }
        if r.within(log.time) {
            current.push(candidate);
        } else {
            historic.push(candidate);
        }
    }
    // Link independent channels only as inferred correlation, never as proof of
    // incident identity. Keep same-channel records distinct.
    let mut incidents: Vec<Value> = Vec::new();
    for candidate in current {
        let time = candidate["occurred_at"]
            .as_str()
            .and_then(|s| parse_clock(s, None));
        let duplicate = incidents.iter().position(|old| {
            old["process"] == candidate["process"]
                && old["pid"] == candidate["pid"]
                && old["evidence_channels"][0]["channel"]
                    != candidate["evidence_channels"][0]["channel"]
                && old["occurred_at"]
                    .as_str()
                    .and_then(|s| parse_clock(s, None))
                    .zip(time)
                    .is_some_and(|(a, b)| (a - b).num_milliseconds().abs() <= 10)
        });
        if let Some(i) = duplicate {
            if let Some(channels) = incidents[i]["evidence_channels"].as_array_mut() {
                let mut channel = candidate["evidence_channels"][0].clone();
                channel["join_basis"] = json!("heuristic");
                channels.push(channel);
            }
            incidents[i]["confidence"] = json!("inferred");
        } else {
            incidents.push(candidate);
        }
    }
    for item in incidents.iter().chain(historic.iter()) {
        let process = item["process"].as_str().unwrap_or("?");
        let pid = item["pid"].as_i64().unwrap_or(0);
        let key = format!(
            "{process}#unknown@{}",
            item["incident_id"].as_str().unwrap_or("record")
        );
        a.finding(super::FindingInput {
            kind: if r.within(
                item["occurred_at"]
                    .as_str()
                    .and_then(|s| parse_clock(s, None)),
            ) {
                "anr_incident"
            } else {
                "historical_anr_report"
            },
            severity: "high",
            confidence: if item["confidence"] == "inferred" {
                "inferred"
            } else if item["evidence_channels"][0]["channel"] == "event_log_am_anr" {
                "reported"
            } else {
                "confirmed"
            },
            process_key: Some(key),
            attributes: json!({"process":process,"pid":pid}),
            metrics: json!({"incident_records":1}),
            evidence: item["evidence"].clone(),
            provenance: item["provenance"].clone(),
        });
    }
    for (i, l) in r
        .lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.text.trim() == "<no ANR has occurred since boot>")
    {
        a.negative.push(json!({"scope":r.title(i),"statement":l.text.trim(),"source":{"archive_path":r.path,"section_title":r.title(i),"section_ordinal":r.section(i).map(|s|s.ordinal),"line_start":l.no,"line_end":l.no},"authority":"platform_assertion"}));
    }
    let live = r
        .logs
        .iter()
        .filter(|l| l.tag == "ActivityManager" && l.message.starts_with("ANR in "))
        .count();
    let event = r.logs.iter().filter(|l| l.tag == "am_anr").count();
    a.count(r, "anr_in_hits", "ANR in ", live);
    a.count(r, "am_anr_hits", "am_anr:", event);
    if live == 0 {
        a.absence(r, "anr_in_hits", "ANR in ");
    }
    if event == 0 {
        a.absence(r, "am_anr_hits", "am_anr:");
    }
    let times: Vec<_> = files
        .iter()
        .filter_map(|f| f["created_at"].as_str().and_then(|s| parse_clock(s, None)))
        .collect();
    let trace_files = json!({"count":if listing.is_empty(){None}else{Some(listing.len())},"archive_entries":files.len(),"listing_total_kib_as_printed":total_blocks,"total_bytes":files.iter().filter_map(|f|f["size_bytes"].as_u64()).sum::<u64>(),"oldest_created_at":times.iter().min().map(stamp),"newest_created_at":times.iter().max().map(stamp),"span_seconds":times.iter().min().zip(times.iter().max()).map(|(a,b)|(*b-*a).num_milliseconds() as f64/1000.),"all_outside_log_window":if times.len()==files.len()&&!times.is_empty(){Some(times.iter().all(|t|!r.within(Some(*t))))}else{None},"files":files});
    let mut channels = Vec::new();
    for (name, kind, title, key) in [
        (
            "logcat_anr_tag",
            "logcat_tag",
            "SYSTEM LOG",
            Some("anr_in_hits"),
        ),
        ("anr_file_listing", "zip_entry_listing", "ANR FILES", None),
        (
            "vm_traces_at_last_anr",
            "zip_inlined_section",
            "VM TRACES AT LAST ANR",
            None,
        ),
        (
            "dumpsys_activity_last_anr",
            "dumpsys_section",
            "SERVICE activity",
            None,
        ),
        (
            "dumpsys_windowmanager_last_anr",
            "dumpsys_section",
            "SERVICE window",
            None,
        ),
        ("dropbox_anr", "dumpsys_section", "DROPBOX ANR", None),
        (
            "event_log_am_anr",
            "event_log_tag",
            "EVENT LOG",
            Some("am_anr_hits"),
        ),
    ] {
        let section = r.find_section(title);
        channels.push(json!({"channel":name,"emitter_kind":kind,"present":section.is_some_and(|s|s.status=="ok"),"counts_ref":key,"provenance":section.map(|s|r.snapshot(s.start,name))}));
    }
    a.set("channels", json!(channels));
    a.set("anr_incidents", json!(incidents));
    a.set("historical_reports", json!(historic));
    a.set("trace_files", trace_files);
    a.set("last_anr_trace", last_trace);
    a.gaps.push(gap("trace_files.files[].created_at","UNVERIFIABLE","Filename timestamps do not encode a timezone; current-day explicit offset is used only as a documented device-clock assumption, while in-file explicit timestamps govern provenance."));
    Ok(a)
}

#[derive(Debug, Clone)]
struct Crash {
    data: Value,
    evidence: Value,
    provenance: Value,
    frames: Vec<String>,
    time: Option<Clock>,
    raw_hits: usize,
}
fn normalize_frame(frame: &str) -> String {
    if let Some(c) = captures(r"^(.*?):\d+(\).*)$", frame) {
        format!("{}{}", c[1], c[2])
    } else if let Some(c) = captures(r"^(.*)\+\d+\)$", frame) {
        format!("{})", c[1])
    } else {
        frame.to_string()
    }
}
fn java_crashes(r: &Report) -> Vec<Crash> {
    let mut crashes = Vec::new();
    for (pos, log) in r.logs.iter().enumerate() {
        if log.tag != "AndroidRuntime" || !log.message.starts_with("FATAL EXCEPTION:") {
            continue;
        }
        let mut process = None;
        let mut pid = None;
        let mut exception = None;
        let mut message = None;
        let mut frames = Vec::new();
        for next in r.logs.iter().skip(pos + 1).take(100) {
            if next.index > log.index + 150 {
                break;
            }
            if next.tag != log.tag || next.pid != log.pid {
                continue;
            }
            if next.message.starts_with("FATAL EXCEPTION:") {
                break;
            }
            if let Some(c) = captures(r"^Process: (.*), PID: (\d+)$", &next.message) {
                process = normal(&c[1]);
                pid = number(&c[2]);
            } else if next.message.trim_start().starts_with("at ") {
                if frames.len() < 5 {
                    frames.push(next.message.trim().trim_start_matches("at ").to_string());
                }
            } else if exception.is_none()
                && let Some(c) = captures(
                    r"^([\w.$]+(?:Exception|Error|Throwable))(?:: (.*))?$",
                    next.message.trim(),
                )
            {
                exception = Some(c[1].clone());
                message = normal(&c[2]);
            }
        }
        let (Some(process), Some(pid), Some(exception)) = (process, pid, exception) else {
            continue;
        };
        let key = format!(
            "{process}|{pid}|{}|{exception}|{}",
            log.raw_time,
            frames
                .first()
                .map(|s| normalize_frame(s))
                .unwrap_or_default()
        );
        if let Some(old) = crashes
            .iter_mut()
            .find(|c: &&mut Crash| c.data["dedup_key"] == key)
        {
            old.raw_hits += 1;
            continue;
        }
        let data = json!({"incident_id":format!("java-{}",hash(&key)),"group_id":null,"language":"java","package":process.split(':').next(),"process":process,"pid":pid,"prefix_pid":log.pid,"prefix_uid":uid(&log.uid_raw),"thread_name":log.message.strip_prefix("FATAL EXCEPTION:").map(str::trim),"exception_class":exception,"exception_message":message,"top_frames":frames,"signal_number":null,"signal_name":null,"sigcode_name":null,"fault_addr":null,"abort_message":null,"occurred_at":log.time.as_ref().map(stamp),"process_uptime_seconds":null,"flags_hex":null,"crash_handler":null,"evidence_channels":[{"channel":"crash_buffer","join_basis":"same_logcat_line"}],"dedup_key":key});
        crashes.push(Crash {
            data,
            evidence: r.evidence(log.index),
            provenance: r.log_provenance(log, "crash_buffer"),
            frames,
            time: log.time,
            raw_hits: 1,
        });
    }
    crashes
}
fn native_crashes(archive: &Archive, r: &Report) -> Result<Vec<Crash>, InsightError> {
    let mut result = Vec::new();
    for entry in archive
        .entries()
        .iter()
        .filter(|e| captures(r"^FS/data/tombstones/tombstone_\d+$", &e.path).is_some())
    {
        let lines = entry_lines(archive, entry.id)?;
        if !lines
            .iter()
            .take(4)
            .any(|l| l.text.starts_with("*** *** ***"))
        {
            continue;
        }
        let mut data = object(
            "incident_id group_id package process pid prefix_pid prefix_uid thread_name exception_class exception_message signal_number signal_name sigcode_name fault_addr abort_message occurred_at process_uptime_seconds flags_hex crash_handler",
        );
        data["language"] = json!("native");
        let mut time = None;
        let mut frames = Vec::new();
        let mut signal_line = 0usize;
        for (i, l) in lines.iter().enumerate() {
            let t = l.text.trim();
            if let Some(v) = t.strip_prefix("Timestamp:") {
                time = parse_clock(v, None);
                data["occurred_at"] = json!(time.as_ref().map(stamp));
            }
            if let Some(v) = t.strip_prefix("Cmdline:") {
                data["process"] = json!(normal(v));
                data["package"] = json!(v.trim().split(':').next());
            }
            if let Some(c) = captures(r"^pid: (\d+), tid: (\d+), name: (.*?)\s+>>> (.*?) <<<$", t) {
                data["pid"] = json!(number(&c[1]));
                data["thread_name"] = json!(c[3]);
                data["process"] = json!(c[4]);
                data["package"] = json!(c[4].split(':').next());
            }
            if let Some(c) = captures(
                r"^signal (\d+) \(([^)]+)\), code .*?\(([^)]+)\), fault addr (.*)$",
                t,
            ) {
                data["signal_number"] = json!(number(&c[1]));
                data["signal_name"] = json!(c[2]);
                data["sigcode_name"] = json!(c[3]);
                data["fault_addr"] = json!(normal(&c[4]));
                signal_line = i;
            }
            if let Some(v) = t.strip_prefix("Abort message:") {
                data["abort_message"] = json!(excerpt(v.trim().trim_matches('\'')).0);
            }
            if let Some(c) = captures(r"^Process uptime: (\d+)s", t) {
                data["process_uptime_seconds"] = json!(number(&c[1]));
            }
            if t.starts_with('#') && t.contains(" pc ") && frames.len() < 12 {
                let parts: Vec<_> = t.splitn(4, ' ').filter(|s| !s.is_empty()).collect();
                let frame = if let Some(c) =
                    captures(r"^#\d+\s+pc\s+[0-9a-f]+\s+(.*?)(?: \(BuildId:.*)?$", t)
                {
                    c[1].clone()
                } else {
                    parts.last().copied().unwrap_or(t).to_string()
                };
                frames.push(frame);
            }
        }
        if data["signal_number"].is_null() || data["process"].is_null() {
            continue;
        }
        let key = format!(
            "{}|{}|{}|{}",
            entry.path, data["process"], data["pid"], data["occurred_at"]
        );
        data["incident_id"] = json!(format!("native-{}", hash(&key)));
        data["top_frames"] = json!(frames);
        data["evidence_channels"] =
            json!([{"channel":"tombstone_files","join_basis":"same_logcat_line"}]);
        let Some(line) = lines.get(signal_line) else {
            continue;
        };
        result.push(Crash {
            data,
            evidence: side_evidence(entry, line),
            provenance: side_provenance(
                r,
                entry,
                line,
                "tombstone_files",
                time.or_else(|| zip_stamp(entry, r)),
                if time.is_some() {
                    "tombstone_header"
                } else {
                    "zip_entry_mtime"
                },
            ),
            frames,
            time,
            raw_hits: 1,
        });
    }
    Ok(result)
}
fn group_crashes(
    r: &Report,
    mut crashes: Vec<Crash>,
    a: &mut Artifact,
    thresholds: &super::CrashThresholds,
) -> (Vec<Value>, Vec<Value>) {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, c) in crashes.iter().enumerate() {
        let top = if c.data["language"] == "native" {
            c.frames
                .iter()
                .find(|f| !f.contains("/libc.so") && !f.contains("/libbase.so"))
                .or_else(|| c.frames.first())
        } else {
            c.frames.first()
        }
        .map(|s| normalize_frame(s))
        .unwrap_or_default();
        let signature = format!(
            "{}|{}|{}|{}",
            c.data["language"], c.data["exception_class"], c.data["signal_number"], top
        );
        groups.entry(signature).or_default().push(i);
    }
    // When distinct abort-message hashes share termination frames, select the
    // first differing normalized frame as the subgroup discriminator. The free
    // text/hash itself never becomes a grouping key.
    let mut refined = BTreeMap::new();
    for (signature, ids) in groups {
        let hashes: BTreeSet<_> = ids
            .iter()
            .filter_map(|i| crashes[*i].data["abort_message"].as_str())
            .map(hash)
            .collect();
        if crashes[ids[0]].data["language"] == "native" && hashes.len() > 1 {
            let depth = (0..ids
                .iter()
                .map(|i| crashes[*i].frames.len())
                .min()
                .unwrap_or(0))
                .find(|depth| {
                    ids.iter()
                        .map(|i| normalize_frame(&crashes[*i].frames[*depth]))
                        .collect::<BTreeSet<_>>()
                        .len()
                        > 1
                });
            if let Some(depth) = depth {
                for i in ids {
                    let c = &crashes[i];
                    let key = format!(
                        "native|null|{}|{}",
                        c.data["signal_number"],
                        normalize_frame(&c.frames[depth])
                    );
                    refined.entry(key).or_insert_with(Vec::new).push(i);
                }
                continue;
            }
        }
        refined
            .entry(signature)
            .or_insert_with(Vec::new)
            .extend(ids);
    }
    // Historical records must not inflate current repeat/rate/escalation rules.
    let mut cohorts: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (signature, ids) in refined {
        for i in ids {
            let freshness = crashes[i].provenance["freshness"]
                .as_str()
                .unwrap_or("undated");
            let cohort = if matches!(freshness, "now" | "in_window") {
                "current"
            } else {
                freshness
            };
            cohorts
                .entry((signature.clone(), cohort.into()))
                .or_default()
                .push(i);
        }
    }
    let mut output = Vec::new();
    for ((signature, cohort), ids) in cohorts {
        let first = &crashes[ids[0]];
        let gid = format!("group-{}", hash(&format!("{signature}|{cohort}")));
        let language = first.data["language"].as_str().unwrap_or("java");
        let times: Vec<_> = ids.iter().filter_map(|i| crashes[*i].time).collect();
        let mut sorted = times.clone();
        sorted.sort();
        let start = sorted.first().copied();
        let end = sorted.last().copied();
        let duration = start
            .zip(end)
            .map(|(a, b)| (b - a).num_milliseconds() as f64 / 1000.);
        let rate = ratio(Some(ids.len() as f64), duration);
        let mut gaps: Vec<_> = sorted
            .windows(2)
            .map(|w| (w[1] - w[0]).num_milliseconds())
            .collect();
        gaps.sort();
        let max_gap = gaps.last().copied();
        let p50 = gaps.get(gaps.len() / 2).copied();
        let pkgs: BTreeSet<_> = ids
            .iter()
            .filter_map(|i| crashes[*i].data["package"].as_str())
            .map(str::to_string)
            .collect();
        let processes: BTreeSet<_> = ids
            .iter()
            .filter_map(|i| crashes[*i].data["process"].as_str())
            .map(str::to_string)
            .collect();
        let same_cohort = |log: &&crate::insight::report::Log| {
            let age = log
                .time
                .zip(r.capture_time)
                .map(|(t, c)| (c - t).num_seconds().max(0));
            match cohort.as_str() {
                "current" => age.is_some_and(|n| n < 3600),
                "stale" => age.is_some_and(|n| n >= 3600),
                _ => age.is_none(),
            }
        };
        let manager: BTreeSet<_> = r
            .logs
            .iter()
            .filter(same_cohort)
            .filter(|l| {
                processes.iter().any(|p| l.message.contains(p))
                    && (l.message.contains("crashed too many times")
                        || l.message.contains("crashed quickly")
                        || l.message.contains("over process crash limit"))
            })
            .map(|l| (&l.raw_time, &l.message))
            .collect();
        let rescue: BTreeSet<_> = r
            .logs
            .iter()
            .filter(same_cohort)
            .filter(|l| {
                pkgs.iter().any(|p| l.message.contains(p))
                    && (l.tag.contains("RescueParty") || l.tag.contains("PackageWatchdog"))
                    && l.message.to_lowercase().contains("fail")
            })
            .map(|l| (&l.raw_time, &l.message))
            .collect();
        let continuity = thresholds.inter_gap_max_ms.unwrap_or_else(|| {
            r.lines
                .iter()
                .find_map(|l| token_num(&l.text, "min_crash_interval="))
                .unwrap_or(120000)
        });
        let mut met = Vec::new();
        if ids.len() >= thresholds.repeat_count {
            met.push("repeat_count");
        }
        if rate.is_some_and(|v| v >= thresholds.rate_per_second) {
            met.push("rate_per_second");
        }
        if duration.is_some_and(|v| v <= thresholds.window_seconds) {
            met.push("window_seconds");
        }
        if max_gap.is_some_and(|v| v <= continuity) {
            met.push("inter_gap_max_ms");
        }
        if manager.len() >= thresholds.escalation || rescue.len() >= thresholds.escalation {
            met.push("escalation");
        }
        let stale = ids
            .iter()
            .all(|i| crashes[*i].provenance["freshness"] == "stale");
        let class = if stale && ids.len() > 1 {
            "stale_recurring"
        } else if met.len() == 5 {
            "crash_loop"
        } else if ids.len() > 1 {
            "recurring"
        } else {
            "isolated"
        };
        let aborts: BTreeSet<_> = ids
            .iter()
            .filter_map(|i| crashes[*i].data["abort_message"].as_str())
            .map(hash)
            .collect();
        let raw: usize = ids.iter().map(|i| crashes[*i].raw_hits).sum();
        let pids: BTreeSet<_> = ids
            .iter()
            .filter_map(|i| crashes[*i].data["pid"].as_i64())
            .collect();
        let dup_pids: Vec<_> = ids
            .iter()
            .filter(|i| crashes[**i].raw_hits > 1)
            .filter_map(|i| crashes[*i].data["pid"].as_i64())
            .collect();
        let g = json!({"group_id":gid,"language":language,"signature":{"exception_class":first.data["exception_class"],"signal_number":first.data["signal_number"],"signal_name":first.data["signal_name"],"sigcode_name":first.data["sigcode_name"],"top_frame":signature.rsplit('|').next(),"abort_message_hash":if aborts.len()==1{aborts.first().cloned()}else{None},"abort_message_varies":aborts.len()>1},"packages":pkgs,"package_count":pkgs.len(),"repeat_count":ids.len(),"raw_line_hits":raw,"duplicate_hits":raw-ids.len(),"duplicate_detection":{"method":"same_process_pid_timestamp_exception_and_normalized_frame","duplicate_pids":dup_pids},"first_seen":start.as_ref().map(stamp),"last_seen":end.as_ref().map(stamp),"window_seconds":duration,"rate_per_second":rate,"inter_gap_p50_ms":p50,"inter_gap_max_ms":max_gap,"pid_span":{"first":if stale{None}else{ids.first().and_then(|i|crashes[*i].data["pid"].as_i64())},"last":if stale{None}else{ids.last().and_then(|i|crashes[*i].data["pid"].as_i64())},"distinct_count":pids.len(),"pid_max":null},"classification":class,"classification_thresholds":{"repeat_count":thresholds.repeat_count,"rate_per_second":thresholds.rate_per_second,"window_seconds":thresholds.window_seconds,"inter_gap_max_ms":continuity,"escalation":thresholds.escalation},"classification_met":met,"manager_kill_count":manager.len(),"manager_kill_reason":manager.iter().find_map(|(_,m)|if m.contains("crashed quickly"){Some("crashed quickly")}else if m.contains("over process crash limit"){Some("over process crash limit")}else{None}),"rescueparty_failure_count":rescue.len(),"rescueparty_disabled":r.logs.iter().find(|l|l.tag.contains("RescueParty")&&l.message.contains("disabled")).map(|_|true),"corroboration":[],"provenance":first.provenance});
        let process = first.data["process"].as_str().unwrap_or("?");
        let key = format!("{process}#unknown@{}", gid);
        a.finding(super::FindingInput { kind: if class=="crash_loop"{"crash_loop"}else{"crash_group"}, severity: if class=="crash_loop"{"critical"}else{"high"}, confidence: "confirmed", process_key: Some(key), attributes: json!({"language":language,"classification":class,"process":process,"group_id":gid}), metrics: json!({"repeat_count":ids.len(),"rate_per_second":rate,"window_seconds":duration,"manager_kill_count":manager.len(),"rescueparty_failure_count":rescue.len(),"inter_gap_max_ms":max_gap}), evidence: first.evidence.clone(), provenance: first.provenance.clone() });
        for i in ids {
            crashes[i].data["group_id"] = json!(gid);
        }
        output.push(g);
    }
    let incidents = crashes
        .into_iter()
        .map(|mut c| {
            if let Some(map) = c.data.as_object_mut() {
                map.remove("dedup_key");
            }
            c.data["provenance"] = c.provenance;
            c.data["evidence"] = c.evidence;
            c.data
        })
        .collect();
    (output, incidents)
}
pub fn analyze_crashes(
    archive: &Archive,
    r: &Report,
    _capture: &Value,
    _apps: &Artifact,
    thresholds: &super::CrashThresholds,
) -> Result<Artifact, InsightError> {
    let mut a = Artifact::new("crashes", r, &["SYSTEM LOG", "EVENT LOG", "DROPBOX"], false);
    let java = java_crashes(r);
    let native = native_crashes(archive, r)?;
    let java_len = java.len();
    let native_len = native.len();
    let (java_groups, java_incidents) = group_crashes(r, java, &mut a, thresholds);
    let (native_groups, native_incidents) = group_crashes(r, native, &mut a, thresholds);
    a.count(r, "fatal_exception_hits", "FATAL EXCEPTION", java_len);
    a.count(
        r,
        "am_crash_hits",
        "am_crash:",
        r.logs.iter().filter(|l| l.tag == "am_crash").count(),
    );
    if java_len == 0 {
        a.absence(r, "fatal_exception_hits", "FATAL EXCEPTION");
    }
    let channels:Vec<_>=[("crash_buffer","logcat_tag","SYSTEM LOG"),("am_crash_event_log","event_log_tag","EVENT LOG"),("dropbox_crash","dumpsys_section","DROPBOX"),("tombstone_files","zip_entry_listing","-"),("am_proc_died","event_log_tag","EVENT LOG"),("manager_kill_lines","logcat_tag","SYSTEM LOG"),("rescueparty_lines","logcat_tag","SYSTEM LOG")].into_iter().map(|(name,kind,title)|json!({"channel":name,"emitter_kind":kind,"present":if name=="tombstone_files"{native_len>0}else{r.find_section(title).is_some()},"window_start":if name=="tombstone_files"{Value::Null}else{json!(r.first.as_ref().map(stamp))},"window_end":if name=="tombstone_files"{Value::Null}else{json!(r.last.as_ref().map(stamp))},"coverage_ratio":null})).collect();
    let mut reports = Vec::new();
    let mut seen = BTreeSet::new();
    for log in r.logs.iter().filter(|log| log.tag == "am_crash") {
        let Some(payload) = log
            .message
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
        else {
            continue;
        };
        let fields: Vec<_> = payload.splitn(8, ',').map(str::trim).collect();
        if fields.len() < 6 || !seen.insert((log.raw_time.clone(), payload.to_string())) {
            continue;
        }
        let pid = number(fields[1]);
        let process = fields[2];
        let provenance = r.log_provenance(log, "am_crash_event_log");
        reports.push(json!({"process":process,"pid":pid,"user_id":number(fields[0]),"flags_hex":fields[3],"exception_class":fields[4],"exception_message":normal(fields[5]),"occurred_at":log.time.as_ref().map(stamp),"evidence":r.evidence(log.index),"provenance":provenance}));
        // Secondary event rows are observations, never added to the primary
        // crash-buffer incident count. A sole event report stays reported.
        if java_len == 0 {
            a.finding(super::FindingInput {
                kind: "reported_crash_event",
                severity: "high",
                confidence: "reported",
                process_key: Some(format!("{process}#unresolved@{}", reports.len())),
                attributes: json!({"process":process}),
                metrics: json!({"retained_event_rows":1}),
                evidence: r.evidence(log.index),
                provenance: provenance.clone(),
            });
        }
    }
    a.set("event_log_reports", json!(reports));
    a.set("channels", json!(channels));
    a.set("count_authority",json!({"channel":"crash_buffer","rule":"Count validated, de-duplicated retained Java crash-buffer records; other channels do not replace this count and it is not a device-wide lifetime total."}));
    a.set("java_crash_groups", json!(java_groups));
    a.set("java_crash_incidents", json!(java_incidents));
    a.set("native_crash_groups", json!(native_groups));
    a.set("native_crash_incidents", json!(native_incidents));
    a.gaps.push(gap(
        "crash_groups[].pid_span.pid_max",
        "NOT_PRINTED",
        "No /proc/sys/kernel/pid_max observation was found in this report.",
    ));
    Ok(a)
}
