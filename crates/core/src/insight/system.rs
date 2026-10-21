//! Device facts, source inventory, and the canonical capture context.
use super::{
    Artifact, InsightError,
    common::*,
    report::{Report, before},
};
use crate::{Archive, EntryMeta};
use chrono::{Duration, NaiveDate, TimeZone};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn zip_stamp(entry: &EntryMeta, r: &Report) -> Option<Clock> {
    let m = entry.modified?;
    let y = 1980 + i32::from(m.dos_date >> 9);
    let mo = u32::from((m.dos_date >> 5) & 15);
    let d = u32::from(m.dos_date & 31);
    let date = NaiveDate::from_ymd_opt(y, mo, d)?.and_hms_opt(
        u32::from(m.dos_time >> 11),
        u32::from((m.dos_time >> 5) & 63),
        u32::from(m.dos_time & 31) * 2,
    )?;
    r.offset?.from_local_datetime(&date).single()
}
fn capture(r: &Report) -> Value {
    let mut boot_bounds: Option<(Clock, Clock)> = None;
    let mut inputs = Vec::new();
    for (i, l) in r.lines.iter().enumerate() {
        if let Some(c) = captures(r"^Uptime: (\d+) Realtime: (\d+)", l.text.trim())
            && let Some(ms) = number(&c[1])
            && let Some(end) = r.section_clock(i)
        {
            let Some(duration) = r.section(i).and_then(|s| s.duration) else {
                continue;
            };
            let Some(lower) = before(end, duration).checked_sub_signed(Duration::milliseconds(ms))
            else {
                continue;
            };
            let Some(upper) = end
                .checked_add_signed(Duration::seconds(1))
                .and_then(|t| t.checked_sub_signed(Duration::milliseconds(ms)))
            else {
                continue;
            };
            boot_bounds =
                Some(boot_bounds.map_or((lower, upper), |(lo, hi)| (lo.max(lower), hi.min(upper))));
            inputs.push(json!({"line":l.no,"kind":"printed_uptime","wall":stamp(&end),"uptime_ms":ms,"section_duration_seconds":duration}));
        }
    }
    let mut uptime = None;
    let mut bounds: Option<[f64; 2]> = None;
    let mut boot = None;
    let mut boot_interval: Option<[String; 2]> = None;
    if let Some(cap) = r.capture_time
        && let Some((lo, hi)) = boot_bounds
        && lo <= hi
    {
        let lower = (cap - hi).num_milliseconds() as f64 / 1000.;
        let upper = (cap + Duration::seconds(1) - lo).num_milliseconds() as f64 / 1000.;
        bounds = Some([round(lower.max(0.), 3), round(upper.max(0.), 3)]);
        uptime = Some(((lower + upper) / 2.).max(0.).floor() as i64);
        let mid = lo + (hi - lo) / 2;
        boot = Some(mid);
        boot_interval = Some([stamp(&lo), stamp(&hi)]);
    }
    if bounds.is_none()
        && let Some(raw) = r.header("Uptime")
        && let Some(c) = captures(
            r"up (\d+) weeks?, (\d+) days?, (\d+) hours?, (\d+) minutes?",
            &raw,
        )
    {
        let total = [604800i64, 86400, 3600, 60]
            .into_iter()
            .zip(c.iter().skip(1))
            .try_fold(0i64, |sum, (factor, value)| {
                sum.checked_add(number(value)?.checked_mul(factor)?)
            });
        if let Some(total) = total {
            bounds = Some([total as f64, total as f64 + 60.]);
        }
    }
    let cold = bounds.and_then(|[a, b]| {
        if b <= 300. {
            Some(true)
        } else if a >= 300. {
            Some(false)
        } else {
            None
        }
    });
    let mut window = r.window();
    window["first_line_minus_boot_seconds"] = json!(
        r.first
            .zip(boot)
            .map(|(a, b)| (a - b).num_milliseconds() as f64 / 1000.)
    );
    let firstboot = r.prop("ro.runtime.firstboot").and_then(|s| number(&s));
    let corroboration=firstboot.and_then(|ms|chrono::DateTime::from_timestamp_millis(ms).map(|t|t.fixed_offset())).map(|t|json!({"ro_runtime_firstboot":firstboot,"implied_at_local":stamp(&t),"agrees_with_derived":boot.map(|b|(b-t).num_seconds().abs()<=2),"delta_seconds":boot.map(|b|(b-t).num_milliseconds() as f64/1000.)}));
    json!({"captured_at":r.capture_time.as_ref().map(stamp),"captured_at_time_base":r.capture_time.map(|_|"dumpstate_header"),"timezone_basis":{"offset":r.offset.map(|o|o.to_string()),"source_line":r.offset_line.map(|i|r.lines[i].no),"rule":"Use an explicit-offset timestamp on the same local date as the banner; retain an unknown clock if none exists"},"uptime_at_capture_seconds":uptime,"uptime":{"header_raw":r.header("Uptime"),"section_raw":r.section_lines("UPTIME").find(|(_,l)|l.text.contains(" up ")).map(|(_,l)|l.text.clone()),"raw_precision":r.header("Uptime").map(|_|"minutes"),"seconds_at_start":uptime,"bounds_seconds":bounds,"derivation":"For each printed uptime pair, boot lies in [section_end-duration-uptime, section_end+1s-uptime]; intersect valid brackets, then capture uptime = banner - bracket midpoint. Minute-only headers retain bounds without inventing exact seconds.","derivation_inputs":inputs},"log_window":window,"boot":{"at":boot.as_ref().map(stamp),"bounds":boot_interval,"derivation":"See capture.uptime.derivation and derivation_inputs","epoch_corroboration":corroboration},"is_cold_boot":cold})
}
fn fact_tree(facts: &mut Value, path: &str, v: &Value, evidence: &Value, observed: &str) {
    match v {
        Value::Object(map) => {
            for (k, v) in map {
                if !["provenance", "evidence"].contains(&k.as_str()) {
                    fact_tree(facts, &format!("{path}.{k}"), v, evidence, observed)
                }
            }
        }
        Value::Array(array) => {
            for (i, v) in array.iter().enumerate() {
                fact_tree(facts, &format!("{path}[{i}]"), v, evidence, observed)
            }
        }
        Value::Null => {}
        _ => {
            facts[path] = json!({"observed_at":observed,"evidence":evidence,"stale_relative_to_capture_seconds":null});
        }
    }
}
fn default_evidence(r: &Report) -> Value {
    if r.lines.is_empty() {
        json!({"archive_path":"-","kind":"record","locator":{"line_start":null,"line_end":null,"byte_offset":null,"byte_length":null},"excerpt":"","excerpt_bytes":0,"excerpt_truncated":false,"excerpt_complete":false})
    } else {
        r.evidence(0)
    }
}
fn property(r: &Report, prop: &str, header: Option<&str>) -> Option<String> {
    r.prop(prop).or_else(|| header.and_then(|h| r.header(h)))
}
fn users(r: &Report) -> Value {
    let mut list: Vec<Value> = Vec::new();
    let mut current = None;
    for (i, l) in r.section_lines("SERVICE user") {
        let text = l.text.trim();
        if let Some(t) = text.strip_prefix("Current user:") {
            current = number(t);
        }
        if let Some(c) = captures(
            r"^UserInfo\{(\d+):([^:]*):([0-9a-fA-F]+)\}.*?(?:serialNo=(\d+))?",
            text,
        ) {
            let mut user = object(
                "id name flags_hex parent_id type state serial_no created_ago start_time_ago last_logged_in_ago unlock_time_ago last_entered_foreground_ago",
            );
            user["id"] = json!(number(&c[1]));
            user["name"] = json!(normal(&c[2]));
            user["flags_hex"] = json!(i64::from_str_radix(&c[3], 16).ok());
            user["flags"] = json!([]);
            user["is_primary"] = json!(false);
            user["serial_no"] = json!(token_num(text, "serialNo="));
            user["provenance"] = r.snapshot(i, "dumpsys_user");
            list.push(user);
        } else if let Some(u) = list.last_mut() {
            for (prefix, key) in [
                ("Type:", "type"),
                ("State:", "state"),
                ("Created:", "created_ago"),
                ("Start time:", "start_time_ago"),
                ("Last logged in:", "last_logged_in_ago"),
                ("Unlock time:", "unlock_time_ago"),
                ("Last entered foreground:", "last_entered_foreground_ago"),
            ] {
                if let Some(v) = text.strip_prefix(prefix) {
                    u[key] = json!(normal(v.trim().trim_end_matches(" ago")));
                }
            }
            if let Some(v) = text.strip_prefix("Flags:") {
                let flags = words(v);
                u["is_primary"] = json!(flags.iter().any(|f| f == "PRIMARY"));
                u["flags"] = json!(flags);
            }
            if let Some(v) = text.strip_prefix("Profile group id:") {
                u["parent_id"] = json!(number(v));
            }
        }
    }
    let mut ids = BTreeSet::new();
    let mut install = 0;
    let mut shared = 0;
    let mut shared_users = 0;
    let mut other_users = 0;
    let mut in_shared = false;
    for (_, l) in r.section_lines("SERVICE package") {
        let t = l.text.trim();
        if t.starts_with("SharedUser [") {
            shared += 1;
            in_shared = true;
        }
        if t.starts_with("Package [") {
            in_shared = false;
        }
        if let Some(c) = captures(r"^User (\d+):", t) {
            ids.insert(number(&c[1]).unwrap_or(0));
            if t.contains("installed=") {
                install += 1;
            } else if in_shared {
                shared_users += 1;
            } else {
                other_users += 1;
            }
        }
    }
    json!({"current_user_id":current,"list":list,"cross_check":{"package_dump_user_ids":ids,"package_install_state_blocks":install,"shared_user_blocks":shared,"shared_user_state_blocks":shared_users,"other_user_header_blocks":other_users}})
}
pub fn analyze(archive: &Archive, r: &Report) -> Result<(Artifact, Value, Value), InsightError> {
    let mut a = Artifact::new(
        "system",
        r,
        &[
            "SYSTEM PROPERTIES",
            "UPTIME",
            "SERVICE user",
            "HARDWARE HALS",
        ],
        true,
    );
    let cap = capture(r);
    let fingerprint = property(r, "ro.build.fingerprint", Some("Build fingerprint"));
    let fp: Vec<_> = fingerprint
        .as_deref()
        .unwrap_or("")
        .split(['/', ':'])
        .collect();
    let build_type = r
        .prop("ro.build.type")
        .or_else(|| fp.get(6).and_then(|s| normal(s)));
    let mut build = object(
        "display_id fingerprint type is_user_build tags incremental module_metadata_version security_patch build_date_utc bootloader radio network boot_image_fingerprint",
    );
    build["display_id"] = json!(property(r, "ro.build.display.id", Some("Build")));
    build["fingerprint"] = json!(fingerprint);
    build["type"] = json!(build_type);
    build["is_user_build"] = json!(build_type.as_deref().map(|t| t == "user"));
    build["tags"] = json!(
        r.prop("ro.build.tags")
            .or_else(|| fp.get(7).and_then(|s| normal(s)))
    );
    build["incremental"] = json!(
        r.prop("ro.build.version.incremental")
            .or_else(|| fp.get(5).and_then(|s| normal(s)))
    );
    build["version"] = json!({"release":r.prop("ro.build.version.release").or_else(||fp.get(3).and_then(|s|normal(s))),"sdk":property(r,"ro.build.version.sdk",Some("Android SDK version")).and_then(|s|number(&s))});
    build["sdk_extensions"] = json!(
        r.header("SDK extensions")
            .map(|v| v
                .trim_matches(['[', ']'])
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>())
            .unwrap_or_default()
    );
    build["module_metadata_version"] =
        json!(r.header("Module Metadata version").and_then(|s| number(&s)));
    for (key, prop) in [
        ("security_patch", "ro.build.version.security_patch"),
        ("build_date_utc", "ro.build.date"),
        ("boot_image_fingerprint", "ro.bootimage.build.fingerprint"),
    ] {
        build[key] = json!(r.prop(prop));
    }
    for (key, prop, header) in [
        ("bootloader", "ro.bootloader", "Bootloader"),
        ("radio", "gsm.version.baseband", "Radio"),
        ("network", "gsm.operator.alpha", "Network"),
    ] {
        build[key] = json!(property(r, prop, Some(header)));
    }
    let mut product = object(
        "name device brand manufacturer model first_api_level characteristics cpu_abi cpu_abilist",
    );
    for (key, prop) in [
        ("name", "ro.product.name"),
        ("device", "ro.product.device"),
        ("brand", "ro.product.brand"),
        ("manufacturer", "ro.product.manufacturer"),
        ("model", "ro.product.model"),
        ("cpu_abi", "ro.product.cpu.abi"),
        ("cpu_abilist", "ro.product.cpu.abilist"),
    ] {
        product[key] = json!(r.prop(prop));
    }
    product["first_api_level"] = json!(
        r.prop("ro.product.first_api_level")
            .and_then(|v| number(&v))
    );
    product["characteristics"] = json!(
        r.prop("ro.build.characteristics")
            .map(|s| s.split(',').map(str::to_string).collect::<Vec<_>>())
            .unwrap_or_default()
    );
    let mut bootconfig = json!({});
    for (_, l) in r.lines.iter().enumerate().take(100) {
        if let Some(c) = captures(r#"^androidboot\.([^ ]+) = "(.*)"$"#, &l.text)
            && [
                "boot_devices",
                "hardware",
                "hardware.vulkan",
                "qemu",
                "qemu.avd_name",
                "qemu.skin",
                "qemu.vsync",
                "veritymode",
                "vbmeta.digest",
                "vbmeta.hash_alg",
                "vbmeta.size",
                "logcat",
            ]
            .contains(&c[1].as_str())
        {
            bootconfig[&c[1]] = json!(c[2]);
        }
    }
    let mut basis = Vec::new();
    let mut matched = false;
    let mut all = true;
    for (prop, target, rule) in [
        ("ro.hardware", "ranchu", "R1"),
        ("ro.kernel.qemu", "1", "R2"),
        ("ro.boot.qemu", "1", "R3"),
    ] {
        let value = r.prop(prop);
        all &= value.is_some();
        let yes = value.as_deref() == Some(target);
        matched |= yes;
        if value.is_some() {
            basis.push(
                json!({"property":prop,"value":value,"matched_rule":if yes{Some(rule)}else{None}}),
            );
        }
    }
    let cmdline = r.header("Command line");
    let hardware = bootconfig["hardware"].as_str().map(str::to_string);
    let device = json!({"is_emulator":if matched{Some(true)}else if all{Some(false)}else{None},"is_emulator_basis":if matched||all{basis}else{Vec::new()},"avd_name":r.prop("ro.boot.qemu.avd_name"),"serial_no":r.prop("ro.boot.serialno"),"boot_hardware":hardware,"bootconfig_excerpt":bootconfig,"cmdline_has_kvm_arm_protected":cmdline.as_ref().map(|s|s.contains("kvm-arm.mode=protected"))});
    let kernel_line = r.header("Kernel");
    let kernel = json!({"version_line":kernel_line,"version_release":kernel_line.as_deref().and_then(|s|s.split_whitespace().nth(if s.starts_with("Linux version"){2}else{0})),"cmdline":cmdline,"config_archived":archive.entries().iter().any(|e|e.path=="FS/proc/config.gz")});
    let mut mounts = BTreeSet::new();
    let mut anr_files = Vec::new();
    let mut fs_paths = Vec::new();
    let mut proto = Vec::new();
    let mut lshal = Vec::new();
    let mut windows = None;
    let mut board = None;
    for e in archive.entries() {
        if e.path.starts_with("FS/") {
            fs_paths.push(e.path.clone());
        }
        if let Some(c) = captures(r"^FS/proc/(\d+)/mountinfo$", &e.path) {
            mounts.insert(number(&c[1]).unwrap_or(0));
        }
        if e.path.starts_with("FS/data/anr/anr_") {
            anr_files.push(json!({"path":e.path,"mtime":zip_stamp(e,r).as_ref().map(stamp),"bytes":e.uncompressed_size}));
        }
        if e.path.starts_with("proto/") {
            proto.push(json!({"path":e.path,"bytes":e.uncompressed_size,"status":if e.uncompressed_size==0{"empty"}else{"ok"}}));
        }
        if e.path.starts_with("lshal-debug/") {
            lshal.push(e.path.clone());
        }
        if e.path.ends_with("visible_windows.zip") {
            windows = Some(e);
        }
        if e.path == "dumpstate_board.txt" {
            board = Some(e);
        }
    }
    let storage = json!({"mountinfo_pids":mounts,"anr_files_archived":anr_files,"archived_trees":fs_paths.iter().filter_map(|p|p.strip_prefix("FS/")?.split('/').next()).collect::<BTreeSet<_>>(),"tombstone_zip_entries":archive.entries().iter().filter(|e|e.path.starts_with("FS/data/tombstones/")).count()});
    let inventory = json!({"proto":proto,"fs_paths":fs_paths,"lshal_debug":lshal,"visible_windows":{"present":windows.is_some(),"entries":[]},"dumpstate_board":{"present":board.is_some(),"bytes":board.map(|e|e.uncompressed_size),"has_board_binary":archive.entries().iter().any(|e|e.path=="dumpstate_board.bin")}});
    if windows.is_some() {
        a.gaps.push(gap(
            "archive_entries.visible_windows.entries",
            "REGION_EXCLUDED",
            "Nested archives are not recursively opened under the archive access contract.",
        ));
    }
    let mut hal_rows = Vec::new();
    let mut has_server = false;
    for (i, l) in r.section_lines("HARDWARE HALS") {
        let text = l.text.trim();
        if text.starts_with("Interface") {
            has_server = text.contains("Server");
            continue;
        }
        let columns: Vec<_> = text.split_whitespace().collect();
        if columns.len() < 3 || !columns[0].contains('@') || !columns[0].contains("::") {
            continue;
        }
        let hash_column = columns
            .iter()
            .position(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()));
        let threads = columns.iter().find_map(|v| {
            let (a, b) = v.split_once('/')?;
            Some((number(a)?, number(b)?))
        });
        let released = hash_column
            .and_then(|j| j.checked_sub(1))
            .and_then(|j| match columns[j] {
                "Y" => Some(true),
                "N" => Some(false),
                _ => None,
            });
        hal_rows.push(json!({"interface_instance":columns[0],"transport":columns[1],"server_pid":if has_server{number(columns[2])}else{None},"arch":columns.get(if has_server{4}else{2}),"threads_used":threads.map(|(a,_)|a),"threads_total":threads.map(|(_,b)|b),"released":released,"interface_hash":hash_column.map(|j|columns[j]),"provenance":r.snapshot(i,"lshal")}));
    }
    let mut displays = Vec::new();
    for (i, l) in r.section_lines("SurfaceFlinger") {
        if let Some(c) = captures(
            r"Display (\d+) \(HWC display (\d+)\): port=(\d+) pnpId=(\S+) displayName=(.*)$",
            l.text.trim(),
        ) {
            displays.push(json!({"display_id":number(&c[1]),"hwc_display":number(&c[2]),"port":number(&c[3]),"pnp_id":c[4],"display_name":c[5],"evidence":r.evidence(i)}));
        }
    }
    let user_data = users(r);
    let mut facts = json!({});
    for (key, value, observed) in [
        ("build", build.clone(), "boot"),
        ("product", product.clone(), "boot"),
        ("device", device.clone(), "boot"),
        ("kernel", kernel.clone(), "boot"),
        ("users", user_data.clone(), "capture"),
    ] {
        let evidence = r
            .find_section(if key == "users" {
                "SERVICE user"
            } else {
                "SYSTEM PROPERTIES"
            })
            .map(|s| r.section_evidence(s))
            .unwrap_or_else(|| default_evidence(r));
        fact_tree(&mut facts, key, &value, &evidence, observed);
        a.set(key, value);
    }
    a.set(
        "hal",
        json!({"hidl_services":hal_rows,"hidl_debug_entries":lshal}),
    );
    a.set("displays", json!(displays));
    a.set("storage", storage);
    a.set("archive_entries", inventory);
    for key in ["hal", "displays", "storage", "archive_entries"] {
        let title = match key {
            "hal" => "HARDWARE HALS",
            "displays" => "SurfaceFlinger",
            _ => "-",
        };
        let evidence = r
            .find_section(title)
            .map(|s| r.section_evidence(s))
            .unwrap_or_else(|| default_evidence(r));
        if let Some(value) = a.data.get(key) {
            fact_tree(
                &mut facts,
                key,
                value,
                &evidence,
                if ["storage", "archive_entries"].contains(&key) {
                    "archive-entry"
                } else {
                    "capture"
                },
            );
        }
    }
    refine_fact_evidence(r, archive, &mut facts, &cap);
    a.set("fact_evidence", facts);
    for prop in &r.property_conflicts {
        a.gaps.push(gap(
            &format!("properties.{prop}"),
            "AMBIGUOUS_SOURCE",
            "Duplicate property observations disagree; no value was selected.",
        ));
    }
    if cap["uptime_at_capture_seconds"].is_null() {
        a.gaps.push(gap("capture.uptime_at_capture_seconds","PARSE_INDETERMINATE","Minute-granular uptime or missing clock anchors do not determine seconds-level uptime."));
    }
    a.gaps.push(gap(
        "capture.log_window.truncated",
        "NOT_PRINTED",
        "The report provides no authoritative log-buffer truncation flag.",
    ));
    let source_device = json!({"product_name":product["name"],"build_id":r.prop("ro.build.id").or_else(||fp.get(4).and_then(|s|normal(s))),"build_display_id":build["display_id"],"build_fingerprint":build["fingerprint"],"android_sdk_version":build["version"]["sdk"],"build_type":build["type"],"device":fp.get(2).and_then(|s|normal(s)),"hardware":hardware,"bootloader":build["bootloader"],"radio":build["radio"]});
    if r.identity.format_version.is_none()
        && !r
            .identity
            .notes
            .contains(&crate::ArchiveNote::ArchiveFormat0)
    {
        a.gaps.push(gap(
            "source.format_version",
            "PARSE_INDETERMINATE",
            "The version marker is invalid, oversized or unreadable; format 0 is not inferred.",
        ));
    }
    if !r.lines.is_empty() {
        let mut attributes =
            json!({"format_version_marker_status": r.identity.markers.version.status.code()});
        if let Some(version) = r.identity.format_version.as_deref().or_else(|| {
            r.identity
                .notes
                .contains(&crate::ArchiveNote::ArchiveFormat0)
                .then_some("0")
        }) {
            attributes["format_version"] = json!(version);
        }
        a.finding(super::FindingInput { kind: "archive_identity", severity: "info", confidence: "confirmed", process_key: None, attributes, metrics: json!({"zip_entries":archive.entry_count(),"stored_bytes":r.identity.size_bytes}), evidence: r.evidence(0), provenance: r.snapshot(0, "archive_header") });
    }
    if cap["is_cold_boot"] == true
        && let Some(i) = r.headers.get("Uptime").map(|(_, i)| *i)
    {
        a.finding(super::FindingInput {
            kind: "cold_boot_context",
            severity: "low",
            confidence: "inferred",
            process_key: None,
            attributes: json!({}),
            metrics: json!({"uptime_bounds_seconds":cap["uptime"]["bounds_seconds"]}),
            evidence: r.evidence(i),
            provenance: r.snapshot(i, "uptime_header"),
        });
    }
    for (key, pattern) in [
        ("tombstone_hits", "tombstone"),
        ("signal_hits", "signal "),
        ("watchdog_hits", "watchdog"),
    ] {
        a.count(r, key, pattern, 0);
    }
    Ok((a, cap, source_device))
}

fn refine_fact_evidence(r: &Report, archive: &Archive, facts: &mut Value, capture: &Value) {
    for (field, property, header) in [
        ("build.display_id", "ro.build.display.id", "Build"),
        (
            "build.fingerprint",
            "ro.build.fingerprint",
            "Build fingerprint",
        ),
        ("build.type", "ro.build.type", "Build fingerprint"),
        ("build.is_user_build", "ro.build.type", "Build fingerprint"),
        ("build.tags", "ro.build.tags", "Build fingerprint"),
        (
            "build.incremental",
            "ro.build.version.incremental",
            "Build fingerprint",
        ),
        (
            "build.version.release",
            "ro.build.version.release",
            "Build fingerprint",
        ),
        (
            "build.version.sdk",
            "ro.build.version.sdk",
            "Android SDK version",
        ),
        (
            "build.module_metadata_version",
            "-",
            "Module Metadata version",
        ),
        ("build.bootloader", "ro.bootloader", "Bootloader"),
        ("build.radio", "gsm.version.baseband", "Radio"),
        ("build.network", "gsm.operator.alpha", "Network"),
        (
            "build.security_patch",
            "ro.build.version.security_patch",
            "-",
        ),
        ("build.build_date_utc", "ro.build.date", "-"),
        (
            "build.boot_image_fingerprint",
            "ro.bootimage.build.fingerprint",
            "-",
        ),
        ("kernel.version_line", "-", "Kernel"),
        ("kernel.version_release", "-", "Kernel"),
        ("kernel.cmdline", "-", "Command line"),
        ("device.cmdline_has_kvm_arm_protected", "-", "Command line"),
        ("device.avd_name", "ro.boot.qemu.avd_name", "-"),
        ("device.serial_no", "ro.boot.serialno", "-"),
    ] {
        if let Some(index) = r
            .properties
            .get(property)
            .map(|(_, i)| *i)
            .or_else(|| r.headers.get(header).map(|(_, i)| *i))
            && let Some(fact) = facts.get_mut(field)
        {
            fact["evidence"] = r.evidence(index);
        }
    }
    for key in [
        "name",
        "device",
        "brand",
        "manufacturer",
        "model",
        "first_api_level",
        "cpu_abi",
        "cpu_abilist",
    ] {
        let property = match key {
            "cpu_abi" => "ro.product.cpu.abi".into(),
            "cpu_abilist" => "ro.product.cpu.abilist".into(),
            _ => format!("ro.product.{key}"),
        };
        if let Some((_, index)) = r.properties.get(&property)
            && let Some(fact) = facts.get_mut(format!("product.{key}"))
        {
            fact["evidence"] = r.evidence(*index);
        }
    }
    if let Some((_, index)) = r.headers.get("== dumpstate") {
        fact_tree(facts, "capture", capture, &r.evidence(*index), "capture");
    }
    if let Some(map) = facts.as_object_mut() {
        for (path, fact) in map.iter_mut() {
            if let Some(key) = path.strip_prefix("device.bootconfig_excerpt.")
                && let Some((i, _)) = r
                    .lines
                    .iter()
                    .enumerate()
                    .take(100)
                    .find(|(_, l)| l.text.starts_with(&format!("androidboot.{key} =")))
            {
                fact["evidence"] = r.evidence(i);
            }
            if path == "device.boot_hardware"
                && let Some((i, _)) = r
                    .lines
                    .iter()
                    .enumerate()
                    .take(100)
                    .find(|(_, l)| l.text.starts_with("androidboot.hardware ="))
            {
                fact["evidence"] = r.evidence(i);
            }
            if path.starts_with("capture.log_window.") {
                fact["observed_at"] = json!("interval");
            }
            if path.starts_with("capture.uptime.")
                || path.starts_with("capture.boot.")
                || path == "capture.uptime_at_capture_seconds"
                || path == "capture.is_cold_boot"
            {
                let inputs: Vec<_> = capture["uptime"]["derivation_inputs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v["line"].as_u64())
                    .filter_map(|n| r.lines.iter().position(|l| l.no == n))
                    .map(|i| r.evidence(i))
                    .collect();
                if let Some(first) = inputs.first() {
                    fact["evidence"] = first.clone();
                    fact["corroborating_evidence"] =
                        json!(inputs.iter().skip(1).collect::<Vec<_>>());
                }
            }
            if path.starts_with("storage.")
                || path.starts_with("archive_entries.")
                || path == "kernel.config_archived"
            {
                let entry = archive.entries().iter().find(|e| path.contains(&e.path));
                fact["observed_at"] = json!("archive-entry");
                fact["evidence"] = json!({"archive_path":entry.map(|e|e.path.as_str()).unwrap_or("-"),"entry_id":entry.map(|e|e.id.0),"kind":if entry.is_some(){"file_entry"}else{"record"},"locator":{"line_start":null,"line_end":null,"byte_offset":null,"byte_length":entry.map(|e|e.uncompressed_size)},"excerpt":"","excerpt_bytes":0,"excerpt_truncated":false,"excerpt_complete":false});
            }
        }
    }
}
