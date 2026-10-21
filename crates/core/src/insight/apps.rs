//! Package/process inventory and conservatively joined lifecycle evidence.
use super::{
    Artifact,
    common::*,
    report::{Log, Report},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn package(name: &str, hash: &str) -> Value {
    let mut p = object(
        "app_id shared_uid_name version_name version_code min_sdk target_sdk code_path resource_path apk_signing_version apex_module_name package_source package_source_name installer_package_name initiating_package_name last_update_time time_stamp",
    );
    p["package_name"] = json!(name);
    p["identity_hash"] = json!(hash);
    for key in [
        "flags",
        "private_flags",
        "users",
        "declared_permissions",
        "install_permissions",
    ] {
        p[key] = json!([]);
    }
    p
}
fn package_user(id: i64, text: &str) -> Value {
    let mut u = object(
        "installed data_dir first_install_time first_install_time_raw enabled stopped not_launched hidden suspended quarantined ce_data_inode de_data_inode install_reason install_reason_name",
    );
    u["user_id"] = json!(id);
    for (key, raw) in [
        ("installed", "installed="),
        ("stopped", "stopped="),
        ("not_launched", "notLaunched="),
        ("hidden", "hidden="),
        ("suspended", "suspended="),
        ("quarantined", "quarantined="),
    ] {
        u[key] = json!(token_bool(text, raw));
    }
    for (key, raw) in [
        ("enabled", "enabled="),
        ("ce_data_inode", "ceDataInode="),
        ("de_data_inode", "deDataInode="),
        ("install_reason", "installReason="),
    ] {
        u[key] = json!(token_num(text, raw));
    }
    u
}
fn package_tier(p: &mut Value) {
    let path = p["code_path"].as_str().unwrap_or("").to_string();
    let prefix = [
        "/data/app",
        "/system_ext",
        "/system",
        "/product",
        "/vendor",
        "/apex",
    ]
    .into_iter()
    .find(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
    .unwrap_or("other");
    let shared = p["shared_uid_name"].as_str().map(str::to_string);
    let tier = if shared.as_deref() == Some("android.uid.system") {
        "shared_system"
    } else {
        match prefix {
            "/system" | "/system_ext" => "system",
            "/product" => "product",
            "/vendor" => "vendor",
            "/apex" => "apex",
            _ => "third_party",
        }
    };
    p["code_path_prefix"] = if path.is_empty() {
        Value::Null
    } else {
        json!(prefix)
    };
    p["tier"] = if path.is_empty() && shared.is_none() {
        Value::Null
    } else {
        json!(tier)
    };
    p["tier_basis"] = json!([{"field":"code_path_prefix","value":p["code_path_prefix"]},{"field":"shared_uid_name","value":p["shared_uid_name"]}]);
    if let Some(users) = p["users"].as_array_mut() {
        for u in users {
            if u["installed"] == false {
                for key in [
                    "data_dir",
                    "first_install_time",
                    "ce_data_inode",
                    "de_data_inode",
                ] {
                    u[key] = Value::Null;
                }
            }
        }
    }
    let source = p["package_source"].as_i64();
    p["package_source_name"] = json!(source.and_then(|n| {
        [
            "UNSPECIFIED",
            "OTHER",
            "STORE",
            "LOCAL_FILE",
            "DOWNLOADED_FILE",
        ]
        .get(n as usize)
        .copied()
    }));
}
fn packages(r: &Report) -> (Vec<Value>, Vec<Value>) {
    let mut packages = Vec::new();
    let mut shared = Vec::new();
    let mut current: Option<Value> = None;
    let mut permission_mode = "";
    let push = |p: Option<Value>, all: &mut Vec<Value>| {
        if let Some(mut p) = p {
            package_tier(&mut p);
            if !all
                .iter()
                .any(|old| old["package_name"] == p["package_name"])
            {
                all.push(p);
            }
        }
    };
    for (i, l) in r.section_lines("SERVICE package") {
        let t = l.text.trim();
        if let Some(c) = captures(r"^Package \[([^\]]+)\] \(([^)]+)\):", t) {
            push(current.take(), &mut packages);
            let mut p = package(&c[1], &c[2]);
            p["provenance"] = r.snapshot(i, "dumpsys_package");
            current = Some(p);
            permission_mode = "";
            continue;
        }
        if let Some(c) = captures(r"^SharedUser \[([^\]]+)\] \(([^)]+)\):", t) {
            push(current.take(), &mut packages);
            shared.push(json!({"name":c[1],"identity_hash":c[2],"app_id":null,"packages":[],"provenance":r.snapshot(i,"dumpsys_package")}));
            continue;
        }
        if current.is_none()
            && let Some(shared_user) = shared.last_mut()
            && let Some(id) = t.strip_prefix("userId=").and_then(number)
        {
            shared_user["app_id"] = json!(id);
        }
        let Some(p) = current.as_mut() else {
            continue;
        };
        if t.contains("installed=")
            && let Some(c) = captures(r"^User (\d+):", t)
        {
            if let Some(users) = p["users"].as_array_mut() {
                users.push(package_user(number(&c[1]).unwrap_or(0), t));
            }
            permission_mode = "";
            continue;
        }
        for (key, raw) in [
            ("code_path", "codePath="),
            ("resource_path", "resourcePath="),
            ("version_name", "versionName="),
            ("installer_package_name", "installerPackageName="),
            ("initiating_package_name", "initiatingPackageName="),
            ("apex_module_name", "apexModuleName="),
            ("last_update_time", "lastUpdateTime="),
            ("time_stamp", "timeStamp="),
        ] {
            if let Some(tail) = t.strip_prefix(raw) {
                p[key] = json!(normal(tail));
            }
        }
        for (key, raw) in [
            ("app_id", "appId="),
            ("app_id", "userId="),
            ("version_code", "versionCode="),
            ("min_sdk", "minSdk="),
            ("target_sdk", "targetSdk="),
            ("apk_signing_version", "apkSigningVersion="),
            ("package_source", "packageSource="),
        ] {
            if let Some(n) = token_num(t, raw) {
                p[key] = json!(n);
            }
        }
        if let Some(c) = captures(r"sharedUser=.*? ([^ /}]+)(?:/\d+)?\}", t) {
            p["shared_uid_name"] = json!(c[1]);
        }
        for (key, raw) in [("flags", "flags=["), ("private_flags", "privateFlags=[")] {
            if let Some(v) = between(t, raw, "]") {
                p[key] = json!(words(v));
            }
        }
        if t == "declared permissions:" {
            permission_mode = "declared_permissions";
        } else if t == "install permissions:" {
            permission_mode = "install_permissions";
        } else if t.ends_with(':') && !t.contains("permission") {
            permission_mode = "";
        }
        if !permission_mode.is_empty()
            && t.contains(':')
            && !t.ends_with(':')
            && (!t.contains("granted=") || t.contains("granted=true"))
            && let Some(list) = p[permission_mode].as_array_mut()
        {
            list.push(json!(t.split(':').next().unwrap_or(t)));
        }
        if let Some(u) = p["users"].as_array_mut().and_then(|u| u.last_mut()) {
            for (key, raw) in [
                ("data_dir", "dataDir="),
                ("first_install_time_raw", "firstInstallTime="),
                ("install_reason_name", "installReasonName="),
            ] {
                if let Some(v) = t.strip_prefix(raw) {
                    u[key] = json!(normal(v));
                    if key == "first_install_time_raw" {
                        u["first_install_time"] = json!(normal(v));
                    }
                }
            }
        }
    }
    push(current, &mut packages);
    for user in &mut shared {
        user["packages"] = json!(
            packages
                .iter()
                .filter(|p| p["shared_uid_name"] == user["name"])
                .map(|p| p["package_name"].clone())
                .collect::<Vec<_>>()
        );
    }
    (packages, shared)
}
fn process_template() -> Value {
    let mut p = object(
        "process_key start_seq record_kind record_identity_hash pid process_name runtime_key attribution_key user_id uid uid_formatted app_id isolated_uid start_uptime_ago start_elapsed_time_ago start_wall_clock start_wall_clock_evidence last_activity_time_ago code_path data_dir instruction_set required_abi class_name compat oom_adj_name cur_proc_state set_proc_state capability",
    );
    for key in [
        "gids",
        "packages",
        "package_dependencies",
        "services",
        "providers",
        "receivers",
        "connections",
        "lifecycle_refs",
    ] {
        p[key] = json!([]);
    }
    p["persistent"] = json!(false);
    p["removed"] = json!(false);
    p["start_seq_conflict"] = json!(false);
    p["oom_adj"] = object("max cur_raw set_raw cur set");
    p["lru"] =
        object("index oom_adj_name capability flags fixed service top_activity client cch_empty");
    p
}
fn processes(r: &Report) -> Vec<Value> {
    let mut list = Vec::new();
    let mut current: Option<Value> = None;
    let flush = |value: Option<Value>, list: &mut Vec<Value>| {
        if let Some(value) = value
            && !list.iter().any(|p| {
                p["record_identity_hash"] == value["record_identity_hash"]
                    && p["start_seq"] == value["start_seq"]
                    && p["pid"] == value["pid"]
            })
        {
            list.push(value);
        }
    };
    for (i, l) in r.section_lines("SERVICE activity") {
        let t = l.text.trim();
        if let Some(c) = captures(
            r"\*(APP|PERS)\* UID (\d+) ProcessRecord\{([^ ]+) (\d+):([^/}]+)/([^}]+)\}",
            t,
        ) {
            flush(current.take(), &mut list);
            let mut p = process_template();
            p["record_kind"] = json!(if c[1] == "PERS" { "persistent" } else { "app" });
            p["record_identity_hash"] = json!(c[3]);
            p["pid"] = json!(number(&c[4]));
            p["process_name"] = json!(c[5]);
            p["uid_formatted"] = json!(c[6]);
            let uid = number(&c[2]);
            p["uid"] = json!(uid);
            p["user_id"] = json!(uid.map(|n| n / 100000));
            p["app_id"] = json!(uid.map(|n| n % 100000));
            p["persistent"] = json!(c[1] == "PERS");
            p["provenance"] = r.snapshot(i, "dumpsys_activity_processes");
            current = Some(p);
            continue;
        }
        let Some(p) = current.as_mut() else {
            continue;
        };
        for (key, raw) in [
            ("start_seq", "startSeq="),
            ("isolated_uid", "isolatedUid="),
            ("cur_proc_state", "curProcState="),
            ("set_proc_state", "setProcState="),
        ] {
            if let Some(n) = token_num(t, raw) {
                p[key] = json!(n);
            }
        }
        for (key, raw) in [
            ("code_path", "codePath="),
            ("data_dir", "dataDir="),
            ("instruction_set", "instructionSet="),
            ("required_abi", "requiredAbi="),
            ("class_name", "className="),
            ("start_uptime_ago", "startUpTime="),
            ("start_elapsed_time_ago", "startElapsedTime="),
            ("last_activity_time_ago", "lastActivityTime="),
        ] {
            if let Some(v) = token(t, raw) {
                p[key] = json!(normal(v));
            }
        }
        for (key, raw) in [
            ("packages", "packageList={"),
            ("package_dependencies", "packageDependencies={"),
        ] {
            if let Some(v) = between(t, raw, "}") {
                p[key] = json!(words(v));
            }
        }
        if let Some(v) = between(t, "gids={", "}") {
            p["gids"] = json!(v.split(',').filter_map(number).collect::<Vec<_>>());
        }
        if let Some(v) = between(t, "compat={", "}") {
            p["compat"] = json!(format!("{{{v}}}"));
        }
        if token_bool(t, "persistent=") == Some(true) {
            p["persistent"] = json!(true);
        }
        if token_bool(t, "removed=") == Some(true) {
            p["removed"] = json!(true);
        }
        for (key, raw) in [
            ("services", "ServiceRecord{"),
            ("providers", "ContentProviderRecord{"),
            ("receivers", "ReceiverList{"),
            ("connections", "ConnectionRecord{"),
        ] {
            if t.contains(raw)
                && let Some(array) = p[key].as_array_mut()
            {
                array.push(json!(t));
            }
        }
        if let Some(c) = captures(
            r"max=(-?\d+) curRaw=(-?\d+) setRaw=(-?\d+) cur=(-?\d+) set=(-?\d+)",
            t,
        ) {
            for (j, key) in ["max", "cur_raw", "set_raw", "cur", "set"]
                .iter()
                .enumerate()
            {
                p["oom_adj"][*key] = json!(number(&c[j + 1]));
            }
        }
    }
    flush(current, &mut list);
    let mut seqs: BTreeMap<i64, usize> = BTreeMap::new();
    for p in &list {
        if let Some(seq) = p["start_seq"].as_i64() {
            *seqs.entry(seq).or_default() += 1;
        }
    }
    for (i, p) in list.iter_mut().enumerate() {
        let name = p["process_name"].as_str().unwrap_or("?").to_string();
        let uid = p["uid"].as_i64().unwrap_or(0);
        let user = uid / 100000;
        p["process_key"] = json!(format!("{name}#{uid}@{}", i + 1));
        p["runtime_key"] = json!(format!("{user}:{name}"));
        p["attribution_key"] = p["packages"]
            .as_array()
            .and_then(|v| v.first())
            .and_then(Value::as_str)
            .map(|name| json!(format!("{name}/{user}")))
            .unwrap_or(Value::Null);
        p["start_seq_conflict"] = json!(
            p["start_seq"]
                .as_i64()
                .is_some_and(|seq| seqs.get(&seq).is_some_and(|n| *n > 1))
        );
    }
    attach_lru(r, &mut list);
    list
}
fn payload(log: &Log) -> Option<Vec<String>> {
    Some(
        log.message
            .trim()
            .strip_prefix('[')?
            .strip_suffix(']')?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
    )
}
fn lifecycle(r: &Report) -> Vec<Value> {
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    for log in &r.logs {
        if !["am_proc_start", "am_proc_bound", "am_proc_died", "am_kill"]
            .contains(&log.tag.as_str())
        {
            continue;
        }
        let Some(fields) = payload(log) else {
            continue;
        };
        if fields.len() < 3 {
            continue;
        }
        let key = (log.tag.clone(), log.raw_time.clone(), fields.clone());
        if !seen.insert(key) {
            continue;
        }
        let is_start = log.tag == "am_proc_start";
        let name = fields.get(if is_start { 3 } else { 2 }).cloned();
        events.push(json!({"kind":log.tag,"at":log.raw_time,"uid_column_raw":log.uid_raw,"prefix_pid":log.pid,"pid":fields.get(1).and_then(|v|number(v)),"tid":log.tid,"user_id":fields.first().and_then(|v|number(v)),"start_uid":if is_start{fields.get(2).and_then(|v|number(v))}else{None},"process_name":name,"hosting_type":if is_start{fields.get(4)}else{None},"hosting_component":if is_start{fields.get(5)}else{None},"oom_adj":if !is_start{fields.get(3).and_then(|v|number(v))}else{None},"reason":if log.tag=="am_kill"{fields.get(4)}else{None},"rss_kib":if log.tag=="am_kill"{fields.get(5).and_then(|v|number(v))}else{None},"process_key":null,"unresolved_reason":"no_process_record","provenance":r.log_provenance(log,"am_proc_event_log")}));
    }
    events
}
fn channel(r: &Report, name: &str, titles: &[&str], logs: &[&str]) -> Value {
    let blocks:Vec<_>=r.sections.iter().filter(|s|titles.iter().any(|t|s.title.contains(t))).map(|s|json!({"index":s.ordinal,"command":s.command,"line_start":r.lines[s.start].no,"line_end":r.lines[s.end].no})).collect();
    let times: Vec<_> = r
        .logs
        .iter()
        .filter(|l| logs.contains(&l.tag.as_str()))
        .filter_map(|l| l.time)
        .collect();
    let first = times.iter().min().copied();
    let last = times.iter().max().copied();
    let duration = first
        .zip(last)
        .map(|(a, b)| (b - a).num_milliseconds() as f64 / 1000.);
    let full = r
        .first
        .zip(r.last)
        .map(|(a, b)| (b - a).num_milliseconds() as f64 / 1000.);
    let coverage = ratio(duration, full);
    let present = !blocks.is_empty();
    json!({"name":name,"present":present,"first_at":first.as_ref().map(stamp),"last_at":last.as_ref().map(stamp),"duration_seconds":duration,"covers_log_window":coverage.map(|v|v>=1.),"coverage_ratio":coverage,"incomplete_reason":if name=="am_proc_event_log"&&coverage.is_some_and(|v|v<1.){Some("event_log_buffer_retention")}else{None},"snapshot_at":r.find_section(titles.first().copied().unwrap_or("-")).and_then(|s|r.section_clock(s.start)).as_ref().map(stamp),"blocks":blocks,"carries":logs})
}
pub fn analyze(r: &Report, capture: &Value) -> Artifact {
    let mut a = Artifact::new(
        "apps",
        r,
        &[
            "SERVICE package",
            "SERVICE activity",
            "EVENT LOG",
            "SERVICE usagestats",
        ],
        false,
    );
    let (packages, shared) = packages(r);
    let mut processes = processes(r);
    let mut events = lifecycle(r);
    // A join requires an observed start, matching UID/name/PID, and a bounded
    // snapshot interval. A recycled PID or missing start never invents identity.
    for i in 0..events.len() {
        let event = &events[i];
        let candidates: Vec<_> = processes
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p["pid"] == event["pid"]
                    && p["process_name"] == event["process_name"]
                    && p["user_id"] == event["user_id"]
            })
            .map(|(j, _)| j)
            .collect();
        if candidates.len() != 1 {
            continue;
        }
        let j = candidates[0];
        let Some(t) = event["provenance"]["captured_at"]
            .as_str()
            .and_then(|s| parse_clock(s, None))
        else {
            continue;
        };
        let Some(snapshot) = processes[j]["provenance"]["captured_at"]
            .as_str()
            .and_then(|s| parse_clock(s, None))
        else {
            events[i]["unresolved_reason"] = json!("outside_snapshot_range");
            continue;
        };
        let start = events
            .iter()
            .filter(|e| {
                e["kind"] == "am_proc_start"
                    && e["pid"] == event["pid"]
                    && e["process_name"] == event["process_name"]
                    && e["start_uid"] == processes[j]["uid"]
            })
            .filter_map(|e| {
                e["provenance"]["captured_at"]
                    .as_str()
                    .and_then(|s| parse_clock(s, None))
            })
            .filter(|s| *s <= snapshot)
            .max();
        let ended = events.iter().any(|e| {
            e["kind"] == "am_proc_died"
                && e["pid"] == event["pid"]
                && e["provenance"]["captured_at"]
                    .as_str()
                    .and_then(|s| parse_clock(s, None))
                    .is_some_and(|d| start.is_some_and(|s| d >= s) && d < snapshot)
        });
        if start.is_some_and(|s| t >= s && t <= snapshot) && !ended {
            events[i]["process_key"] = processes[j]["process_key"].clone();
            events[i]["unresolved_reason"] = Value::Null;
            processes[j]["start_wall_clock"] = json!(start.as_ref().map(stamp));
            if let Some(start_event) = events.iter().find(|e| {
                e["kind"] == "am_proc_start"
                    && e["pid"] == processes[j]["pid"]
                    && e["provenance"]["captured_at"] == processes[j]["start_wall_clock"]
            }) && let Some(line) = start_event["provenance"]["line_start"].as_u64()
            {
                processes[j]["start_wall_clock_evidence"] =
                    r.evidence(line.saturating_sub(1) as usize);
            }
            if let Some(refs) = processes[j]["lifecycle_refs"].as_array_mut() {
                refs.push(json!({"event_kind":events[i]["kind"],"at":events[i]["at"],"event_seq_index":i}));
            }
        }
    }
    let mut index = json!({"package_to_process_keys":{},"process_name_to_packages":{},"uid_to_process_keys":{},"unmapped_process_names":{"count":0,"examples":[]}});
    let mut unmapped = Vec::new();
    for p in &processes {
        let name = p["process_name"].as_str().unwrap_or("");
        index["process_name_to_packages"][name] = p["packages"].clone();
        let u = p["uid"].to_string();
        if index["uid_to_process_keys"][&u].is_null() {
            index["uid_to_process_keys"][&u] = json!([]);
        }
        if let Some(v) = index["uid_to_process_keys"][&u].as_array_mut() {
            v.push(p["process_key"].clone());
        }
        if let Some(pkgs) = p["packages"].as_array() {
            if pkgs.is_empty() {
                unmapped.push(name.to_string());
            }
            for pkg in pkgs.iter().filter_map(Value::as_str) {
                if index["package_to_process_keys"][pkg].is_null() {
                    index["package_to_process_keys"][pkg] = json!([]);
                }
                if let Some(v) = index["package_to_process_keys"][pkg].as_array_mut() {
                    v.push(p["process_key"].clone());
                }
            }
        }
    }
    index["unmapped_process_names"] =
        json!({"count":unmapped.len(),"examples":unmapped.iter().take(10).collect::<Vec<_>>()});
    let mut activity = Vec::new();
    let mut range = String::new();
    let mut user = None;
    let mut seen = BTreeSet::new();
    for (i, l) in r.section_lines("usagestats") {
        let t = l.text.trim();
        if t.contains("timeRange=") || t.starts_with("time range") {
            range = t.to_string();
        }
        if let Some(c) = captures(r"(?:User |user=)(\d+)", t) {
            user = number(&c[1]);
        }
        if let Some(c) = captures(r#"^time="?([0-9-]+ [0-9:]+)"?.*?type=([A-Z_]+)(.*)$"#, t) {
            let pkg = token(&c[3], "package=");
            let cls = token(&c[3], "class=");
            let key = (
                c[1].clone(),
                c[2].clone(),
                pkg.map(str::to_string),
                cls.map(str::to_string),
                user,
            );
            if seen.insert(key) {
                activity.push(json!({"at":c[1],"type":c[2],"package_name":pkg,"class_name":cls,"range_raw":range,"user_id":user,"range_window_seconds":null,"provenance":r.snapshot(i,"usagestats")}));
            }
        }
    }
    let channels = vec![
        channel(r, "dumpsys_package", &["SERVICE package"], &[]),
        channel(r, "dumpsys_activity_processes", &["SERVICE activity"], &[]),
        channel(
            r,
            "am_proc_event_log",
            &["EVENT LOG"],
            &["am_proc_start", "am_proc_bound", "am_proc_died", "am_kill"],
        ),
        channel(r, "usagestats", &["usagestats"], &[]),
        channel(r, "logcat_main", &["SYSTEM LOG"], &[]),
    ];
    let event_channel = &channels[2];
    let names: BTreeSet<_> = processes
        .iter()
        .filter_map(|p| p["process_name"].as_str())
        .collect();
    let user_ids: BTreeSet<_> = packages
        .iter()
        .filter_map(|p| p["users"].as_array())
        .flatten()
        .filter_map(|u| u["user_id"].as_i64())
        .collect();
    let mut counts = json!({"packages":packages.len(),"process_records":processes.len(),"distinct_process_names":names.len(),"packages_with_at_least_one_process":packages.iter().filter(|p|p["package_name"].as_str().is_some_and(|n|index["package_to_process_keys"].get(n).is_some())).count(),"users":user_ids.len(),"shared_uids":shared.len(),"package_user_state_blocks":packages.iter().filter_map(|p|p["users"].as_array()).map(Vec::len).sum::<usize>(),"lru":{"total":null,"non_activity":null,"non_service":null},"car_service_process_names":names.iter().filter(|n|n.starts_with("com.android.car")).count(),"car_service_packages":packages.iter().filter(|p|p["package_name"].as_str().is_some_and(|n|n.starts_with("com.android.car"))).count(),"lifecycle_channel_window":{"first_event_at":event_channel["first_at"],"last_event_at":event_channel["last_at"],"duration_seconds":event_channel["duration_seconds"],"covers_full_log_window":event_channel["covers_log_window"],"coverage_ratio":event_channel["coverage_ratio"],"section_titles":r.sections.iter().filter(|s|s.title=="EVENT LOG").map(|s|s.title.clone()).collect::<Vec<_>>(),"sections_scanned":event_channel["blocks"]},"sources_available":channels.iter().filter(|c|c["present"]==true).map(|c|c["name"].clone()).collect::<Vec<_>>(),"sources_absent":channels.iter().filter(|c|c["present"]==false).map(|c|c["name"].clone()).collect::<Vec<_>>()});
    counts["lru"] = lru_counts(r);
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for e in &events {
        if let Some(reason) = e["unresolved_reason"].as_str() {
            *unresolved.entry(reason.into()).or_default() += 1;
        }
    }
    let starts: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "am_proc_start")
        .collect();
    let distinct: BTreeSet<_> = starts.iter().filter_map(|e| e["pid"].as_i64()).collect();
    let manager: BTreeSet<_> = r
        .logs
        .iter()
        .filter(|l| {
            l.message.contains("crashed too many times") || l.message.contains("crashed quickly")
        })
        .map(|l| (&l.raw_time, &l.message))
        .collect();
    let mut attribution = json!({"kill_sources":{"am_kill_events":events.iter().filter(|e|e["kind"]=="am_kill").count(),"ams_crash_loop_kill_lines":manager.len(),"app_errors_am_process_crashed_too_much":r.lines.iter().filter(|l|l.text.contains("am_process_crashed_too_much")).count()},"kill_source_coverage":{},"pid_reuse":{"worst_process":null,"pid_first":starts.first().map(|s|s["pid"].clone()),"pid_last":starts.last().map(|s|s["pid"].clone()),"distinct_pid_count":distinct.len(),"pid_max":r.prop("kernel.pid_max").and_then(|s|number(&s)),"window_first":event_channel["first_at"],"window_last":event_channel["last_at"],"window_seconds":event_channel["duration_seconds"],"start_proc_events":starts.len()},"unresolved":unresolved.into_iter().map(|(reason,count)|json!({"reason":reason,"count":count})).collect::<Vec<_>>()});
    for key in [
        "am_kill_events",
        "ams_crash_loop_kill_lines",
        "app_errors_am_process_crashed_too_much",
    ] {
        attribution["kill_source_coverage"][key] = json!({"channel":if key=="am_kill_events"{"am_proc_event_log"}else{"logcat_main"},"window_start":event_channel["first_at"],"window_end":event_channel["last_at"],"coverage_ratio":event_channel["coverage_ratio"],"covers_log_window":event_channel["covers_log_window"]});
    }
    for tag in ["am_proc_start", "am_proc_bound", "am_proc_died", "am_kill"] {
        let n = events.iter().filter(|e| e["kind"] == tag).count();
        a.count(r, &format!("{tag}_hits"), &format_pattern(tag), n);
        if let Some(block) = a.counts.get_mut(&format!("{tag}_hits")) {
            merge(
                &mut block["coverage"],
                json!({"window_start":event_channel["first_at"],"window_end":event_channel["last_at"],"archive_window":r.window(),"coverage_ratio":event_channel["coverage_ratio"]}),
            );
        }
    }
    if let Some(section) = r.find_section("SERVICE package") {
        a.finding(super::FindingInput { kind: "package_inventory", severity: "info", confidence: "confirmed", process_key: None, attributes: json!({}), metrics: json!({"packages":packages.len(),"process_records":processes.len(),"shared_uids":shared.len()}), evidence: r.evidence(section.start), provenance: r.snapshot(section.start,"dumpsys_package") });
    }
    if !events.is_empty() {
        let index = events[0]["provenance"]["line_start"]
            .as_u64()
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        a.finding(super::FindingInput { kind: "process_churn", severity: "low", confidence: "confirmed", process_key: None, attributes: json!({}), metrics: json!({"retained_lifecycle_events":events.len(),"window_seconds":event_channel["duration_seconds"],"coverage_ratio":event_channel["coverage_ratio"],"cold_boot":capture["is_cold_boot"]}), evidence: r.evidence(index), provenance: events[0]["provenance"].clone() });
    }
    a.notes.push("WINDOW_LOCAL_IDENTITY: Process keys use observed-name/UID/ordinal fallback; events without demonstrated lifetime boundaries remain unjoined.".into());
    a.gaps.push(gap("inventory.processes[].start_wall_clock","PARSE_INDETERMINATE","Only observed start/snapshot intervals are joined; missing starts or recycled PIDs leave identity unresolved."));
    a.set("channels", json!(channels));
    a.set("inventory",json!({"counts":counts,"packages":packages,"processes":processes,"shared_uids":shared,"index":index}));
    a.set("lifecycle_events", json!(events));
    a.set("activity_events", json!(activity));
    a.set("attribution", attribution);
    a
}
fn format_pattern(tag: &str) -> String {
    format!("{tag}:")
}

fn attach_lru(r: &Report, processes: &mut [Value]) {
    for (_, line) in r.section_lines("SERVICE activity") {
        let Some(c) = captures(
            r"^.*?#(\d+):\s+(\S+)\s+(.*?)\s+([A-Z-]{7})\s+t:\s+\d+\s+(\d+):([^/]+)/([^ ]+)(.*)$",
            line.text.trim(),
        ) else {
            continue;
        };
        let pid = number(&c[5]);
        let name = &c[6];
        let observed_uid = uid(&c[7]);
        for process in processes.iter_mut().filter(|p| {
            p["pid"].as_i64() == pid
                && p["process_name"].as_str() == Some(name.as_str())
                && p["uid"].as_i64() == observed_uid
        }) {
            process["oom_adj_name"] = json!(c[2]);
            process["capability"] = json!(c[4]);
            process["lru"] = json!({"index":number(&c[1]),"oom_adj_name":c[2],"capability":c[4],"flags":words(&c[3]),"fixed":c[8].contains("(fixed)"),"service":c[8].contains("(service)"),"top_activity":c[8].contains("(top-activity)"),"client":c[8].contains("(client)"),"cch_empty":c[8].contains("(empty)")});
        }
    }
}
fn lru_counts(r: &Report) -> Value {
    r.section_lines("SERVICE activity").find_map(|(_,l)|captures(r"Process LRU list .*?, (\d+) total, non-act at (\d+), non-svc at (\d+)",&l.text)).map(|c|json!({"total":number(&c[1]),"non_activity":number(&c[2]),"non_service":number(&c[3])})).unwrap_or_else(||object("total non_activity non_service"))
}
