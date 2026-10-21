//! Memory pressure, snapshot ownership, CPU windows, and contention channels.
use super::{Artifact, common::*, report::Report};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn kib(token: &str) -> Option<f64> {
    let s = token.trim().replace(',', "");
    let split = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let n = s[..split].parse::<f64>().ok()?;
    Some(
        n * match s[split..].trim_end_matches('B') {
            "G" => 1048576.,
            "M" => 1024.,
            "K" | "k" | "" => 1.,
            _ => return None,
        },
    )
}
fn psi(r: &Report, kind: &str) -> Value {
    let mut value = object("some full provenance");
    let mut found = false;
    for (i, l) in r.lines.iter().enumerate() {
        let sec = r.section(i);
        let relevant = sec.is_some_and(|s| {
            s.command
                .as_deref()
                .is_some_and(|s| s.contains(&format!("/pressure/{kind}")))
                || s.title.to_lowercase().contains(&format!("{kind} pressure"))
        });
        if !relevant {
            continue;
        }
        let t = l.text.trim();
        if t.starts_with("some ") || t.starts_with("full ") {
            let name = t.split_whitespace().next().unwrap_or("some");
            value[name] = json!({"avg10":token(t,"avg10=").and_then(decimal),"avg60":token(t,"avg60=").and_then(decimal),"avg300":token(t,"avg300=").and_then(decimal),"total_us":token_num(t,"total=")});
            value["provenance"] = r.provenance(
                i,
                &format!("psi_{kind}"),
                "command_output",
                None,
                "undetermined",
                "none",
            );
            found = true;
        }
    }
    if found { value } else { Value::Null }
}
fn vmstat(r: &Report, key: &str) -> Option<i64> {
    r.lines
        .iter()
        .enumerate()
        .filter(|(i, _)| r.title(*i).contains("VM STAT"))
        .find_map(|(_, l)| {
            l.text
                .trim()
                .strip_prefix(&format!("{key} "))
                .and_then(number)
        })
}
fn memory_process() -> Value {
    object(
        "pid process_name user_id pss_total_kib pss_private_dirty_kib pss_shared_clean_kib rss_total_kib swap_pss_kib swap_uss_kib zswap_kib oom_adj heap_size_kib heap_alloc_kib heap_free_kib source_channel provenance",
    )
}
fn iotop(r: &Report) -> Value {
    let Some(section) = r.find_section("IOTOP") else {
        return Value::Null;
    };
    let mut maxima = [None::<f64>; 4];
    let mut rows = 0;
    for l in &r.lines[section.start..=section.end] {
        let tokens: Vec<_> = l
            .text
            .split_whitespace()
            .filter(|s| s.ends_with('%'))
            .filter_map(decimal)
            .collect();
        if tokens.len() >= 4 {
            rows += 1;
            for (i, p) in tokens.iter().take(4).enumerate() {
                maxima[i] = Some(maxima[i].map_or(*p, |old| old.max(*p)));
            }
        }
    }
    json!({"window_ms":null,"row_granularity":"thread","sort_column":null,"max_io_delay_pct":maxima[0],"max_swap_delay_pct":maxima[1],"max_sched_delay_pct":maxima[2],"max_mem_delay_pct":maxima[3],"rows":rows,"resolution_floor_note":"0.00% is below the two-decimal print resolution; it does not prove no I/O or no wait.","provenance":r.provenance(section.start,"iotop","command_output",None,"undetermined","none")})
}
fn procstats(r: &Report) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    let mut current: Option<Value> = None;
    for (i, l) in r.section_lines("procstats") {
        let t = l.text.trim();
        let name = if t == "CURRENT STATS:" {
            Some("CURRENT")
        } else if t == "AGGREGATED OVER LAST 24 HOURS:" {
            Some("AGGREGATED_24H")
        } else if t == "AGGREGATED OVER LAST 3 HOURS:" {
            Some("AGGREGATED_3H")
        } else {
            None
        };
        if let Some(name) = name {
            if let Some(b) = current.take() {
                blocks.push(b);
            }
            current = Some(
                json!({"sub_section":name,"scope_label_verbatim":t,"period_start_clock":null,"period_total_uptime":null,"period_total_elapsed":null,"period_complete":null,"period_flags":[],"aggregated_over_n":null,"backing_state_file":null,"sys_mem_usage":[],"checkin_period":null,"provenance":r.provenance(i,"procstats",if name=="CURRENT"{"background_sampler"}else{"persisted_aggregate"},None,"undetermined","none")}),
            );
        }
        if t.starts_with("period,") {
            let parts: Vec<_> = t.split(',').collect();
            if parts.len() >= 5 {
                let start = number(parts[2]);
                let end = number(parts[3]);
                let checkin = json!({"start_clock":parts[1],"start_realtime_ms":start,"end_realtime_ms":end,"complete":parts[4]=="complete","window_ms":start.zip(end).map(|(a,b)|b-a)});
                if let Some(b) = current.as_mut() {
                    b["checkin_period"] = checkin;
                }
            }
        }
        let Some(b) = current.as_mut() else {
            continue;
        };
        if let Some(v) = t.strip_prefix("Start time:") {
            let time = parse_clock(v, r.offset);
            b["period_start_clock"] = json!(time.as_ref().map(stamp).or_else(|| normal(v)));
            b["provenance"] = r.provenance(
                i,
                "procstats",
                if b["sub_section"] == "CURRENT" {
                    "background_sampler"
                } else {
                    "persisted_aggregate"
                },
                time,
                "dumpsys_section_snapshot",
                "dumpsys_Timestamp",
            );
        }
        for (prefix, key) in [
            ("Total uptime:", "period_total_uptime"),
            ("Total elapsed time:", "period_total_elapsed"),
            ("mFile=", "backing_state_file"),
        ] {
            if let Some(v) = t.strip_prefix(prefix) {
                b[key] = json!(normal(v));
            }
        }
        if let Some(v) = t.strip_prefix("Aggregated over:") {
            b["aggregated_over_n"] = json!(number(v));
        }
        if t.starts_with("Total elapsed time:") {
            let flags: Vec<_> = ["shutdown", "sysprops", "complete", "partial"]
                .into_iter()
                .filter(|f| t.contains(&format!("({f})")))
                .collect();
            b["period_complete"] = json!(flags.contains(&"complete"));
            b["period_flags"] = json!(flags);
        }
        if let Some(c) = captures(r"^([^:]+): (\d+) samples:", t)
            && let Some(rows) = b["sys_mem_usage"].as_array_mut()
        {
            rows.push(json!({"label":c[1],"sample_count":number(&c[2]),"cached_min_kib":null,"cached_avg_kib":null,"cached_max_kib":null,"free_min_kib":null,"free_avg_kib":null,"free_max_kib":null,"zram_min_kib":null,"zram_avg_kib":null,"zram_max_kib":null,"kernel_min_kib":null,"kernel_avg_kib":null,"kernel_max_kib":null,"native_min_kib":null,"native_avg_kib":null,"native_max_kib":null}));
        }
        if let Some(c) = captures(
            r"^(Cached|Free|ZRam|Kernel|Native): ([0-9.]+[KMG]B) min, ([0-9.]+[KMG]B) avg, ([0-9.]+[KMG]B) max",
            t,
        ) && let Some(row) = b["sys_mem_usage"]
            .as_array_mut()
            .and_then(|rows| rows.last_mut())
        {
            let prefix = c[1].to_lowercase();
            for (j, suffix) in ["min", "avg", "max"].iter().enumerate() {
                row[format!("{prefix}_{suffix}_kib")] = json!(kib(&c[j + 2]));
            }
        }
        if let Some(c) = captures(
            r"^([A-Za-z][A-Za-z /-]*):\s+([0-9.]+[KMG])\s*/\s*([0-9.]+[KMG])\s*/\s*([0-9.]+[KMG])",
            t,
        ) && let Some(array) = b["sys_mem_usage"].as_array_mut()
        {
            array.push(json!({"label":c[1],"sample_count":null,"min_kib":kib(&c[2]),"avg_kib":kib(&c[3]),"max_kib":kib(&c[4])}));
        }
    }
    if let Some(b) = current {
        blocks.push(b);
    }
    blocks
}
fn lmk_kills(r: &Report) -> Vec<Value> {
    let mut kills = Vec::new();
    let mut seen = BTreeSet::new();
    for log in r.logs.iter().filter(|l| l.tag == "lmkd") {
        if let Some(c) = captures(
            r"Kill '([^']+)' \((\d+)\), uid (\d+), oom_score_adj (-?\d+) to free (\d+)kB",
            &log.message,
        ) && seen.insert((log.raw_time.clone(), c[2].clone(), c[1].clone()))
        {
            kills.push(json!({"timestamp":log.time.as_ref().map(stamp),"process_name":c[1],"pid":number(&c[2]),"uid":number(&c[3]),"oom_score_adj":number(&c[4]),"rss_kib":number(&c[5]),"swap_kib":null,"reason_code":null,"reason_text":log.message.split_once("reason: ").map(|(_,s)|excerpt(s).0),"killinfo_event":null,"provenance":r.log_provenance(log,"lmkd_kill"),"evidence":r.evidence(log.index)}));
        }
    }
    kills
}
fn memory(r: &Report) -> (Artifact, Value) {
    let mut a = Artifact::new(
        "memory",
        r,
        &[
            "MEMORY INFO",
            "PROCRANK",
            "SERVICE meminfo",
            "VM STAT",
            "procstats",
        ],
        false,
    );
    let mut system = object(
        "total_ram_kib memory_level free_ram_kib free_ratio used_ram_kib used_pss_kib kernel_used_kib total_pss_kib total_pss_kib_derivation lost_ram_kib tuning gpu_kib dma_buf provenance",
    );
    system["free_ram_components_kib"] = object("cached_pss cached_kernel free");
    let mut swap = object(
        "swap_total_kib swap_used_kib swap_used_pct zram_physical_kib zram_physical_pct_of_ram compression_ratio zram_block_io provenance",
    );
    swap["compression_ratio_derived"] = json!(true);
    let mut processes = Vec::new();
    let mut categories = Vec::new();
    let mut mode = "";
    let mut table: Vec<String> = Vec::new();
    let mut snapshots: Vec<Value> = Vec::new();
    let mut rawswap: BTreeMap<usize, (Option<i64>, Option<i64>, usize)> = BTreeMap::new();
    let mut scanning = BTreeSet::new();
    for (i, l) in r.lines.iter().enumerate() {
        let t = l.text.trim();
        let title = r.title(i);
        if title.contains("MEMORY INFO") {
            if let Some(v) = t.strip_prefix("SwapTotal:")
                && let Some(s) = l.section
            {
                rawswap.entry(s).or_insert((None, None, i)).0 =
                    number(v.split_whitespace().next().unwrap_or(""));
            }
            if let Some(v) = t.strip_prefix("SwapFree:")
                && let Some(s) = l.section
            {
                rawswap.entry(s).or_insert((None, None, i)).1 =
                    number(v.split_whitespace().next().unwrap_or(""));
            }
        }
        if title.contains("meminfo") {
            if t == "Total PSS by process:" {
                mode = "process";
            } else if t == "Total PSS by category:" {
                mode = "category";
            } else if t.starts_with("Total ") && t.ends_with(':') {
                mode = "other";
            }
            if let Some(c) = captures(
                r"^([0-9,]+)K: (.*?) \(pid (\d+).*?\)(?:.*?([0-9,]+)K in swap)?",
                t,
            ) && mode == "process"
            {
                let mut p = memory_process();
                p["pid"] = json!(number(&c[3]));
                p["process_name"] = json!(c[2]);
                p["pss_total_kib"] = json!(number(&c[1]));
                p["swap_pss_kib"] = json!(number(&c[4]));
                p["source_channel"] = json!("dumpsys_meminfo");
                p["provenance"] = r.snapshot(i, "dumpsys_meminfo");
                processes.push(p);
                if t.contains("in swap")
                    && let Some(s) = l.section
                {
                    scanning.insert(s);
                }
            } else if mode == "category"
                && let Some(c) = captures(
                    r"^([0-9,]+)K: ([^(]+?)(?:\s+\(\s*([0-9,]+)K in swap\))?$",
                    t,
                )
            {
                categories.push(json!({"category":c[2].trim(),"pss_total_kib":number(&c[1]),"swap_pss_kib":number(&c[3]),"depth":l.text.len()-l.text.trim_start().len(),"parent_category":null,"provenance":r.snapshot(i,"dumpsys_meminfo")}));
            }
            for (prefix, key) in [
                ("Total RAM:", "total_ram_kib"),
                ("Free RAM:", "free_ram_kib"),
                ("Used RAM:", "used_ram_kib"),
                ("Lost RAM:", "lost_ram_kib"),
                ("GPU:", "gpu_kib"),
            ] {
                if let Some(v) = t.strip_prefix(prefix) {
                    system[key] = json!(v.split_whitespace().next().and_then(number));
                    system["provenance"] = r.snapshot(i, "dumpsys_meminfo");
                }
            }
            if let Some(c) = captures(r"\(status ([^)]+)\)", t) {
                system["memory_level"] = json!(c[1]);
            }
            if let Some(c) = captures(
                r"([0-9,]+)K cached pss \+\s*([0-9,]+)K cached kernel \+\s*([0-9,]+)K free",
                t,
            ) {
                system["free_ram_components_kib"] = json!({"cached_pss":number(&c[1]),"cached_kernel":number(&c[2]),"free":number(&c[3])});
            }
            if let Some(c) = captures(r"([0-9,]+)K used pss \+\s*([0-9,]+)K kernel", t) {
                system["used_pss_kib"] = json!(number(&c[1]));
                system["kernel_used_kib"] = json!(number(&c[2]));
            }
            if let Some(c) = captures(
                r"^Tuning: (\d+) \(large (\d+)\), oom\s+([0-9,]+)K, restore limit\s+([0-9,]+)K(.*)$",
                t,
            ) {
                system["tuning"] = json!({"memory_class_mib":number(&c[1]),"large_memory_class_mib":number(&c[2]),"oom_adj_score_kib":number(&c[3]),"restore_limit_kib":number(&c[4]),"low_ram":c[5].contains("low-ram"),"high_end_gfx":c[5].contains("high-end-gfx")});
            }
            if let Some(c) = captures(
                r"^DMA-BUF:\s+([0-9,]+)K \(\s*([0-9,]+)K mapped \+\s*([0-9,]+)K unmapped\)",
                t,
            ) {
                system["dma_buf"] = json!({"total_kib":number(&c[1]),"mapped_kib":number(&c[2]),"unmapped_kib":number(&c[3])});
            }
        } else {
            mode = "";
        }
        if t.starts_with("PID ") && (t.contains("Pss") || t.contains("PSS")) {
            table = t.split_whitespace().map(str::to_string).collect();
            if t.to_lowercase().contains("swap")
                && let Some(s) = l.section
            {
                scanning.insert(s);
            }
        }
        if (title.contains("PROCRANK")
            || title.contains("PROCDUMP")
            || title.contains("PROCESSES MEMORY"))
            && !table.is_empty()
        {
            let fields: Vec<_> = t.split_whitespace().collect();
            if fields.len() >= table.len() && fields.first().and_then(|s| number(s)).is_some() {
                let mut p = memory_process();
                p["pid"] = json!(fields.first().and_then(|s| number(s)));
                p["process_name"] = json!(fields.last());
                for (column, key) in [
                    ("Pss", "pss_total_kib"),
                    ("Rss", "rss_total_kib"),
                    ("PSwap", "swap_pss_kib"),
                    ("USwap", "swap_uss_kib"),
                    ("ZSwap", "zswap_kib"),
                    ("oom", "oom_adj"),
                ] {
                    if let Some(j) = table.iter().position(|s| s.eq_ignore_ascii_case(column)) {
                        p[key] = json!(fields.get(j).and_then(|s| number(s)));
                    }
                }
                p["source_channel"] = json!(if title.contains("PROCRANK") {
                    "procrank"
                } else {
                    "bugreport_procdump"
                });
                p["zswap_estimated"] = json!(true);
                p["provenance"] = r.provenance(
                    i,
                    "procrank",
                    "point_in_time_snapshot",
                    None,
                    "undetermined",
                    "none",
                );
                processes.push(p);
            }
        }
        if let Some(c) = captures(
            r"^ZRAM:\s*([0-9,]+)K physical used for\s*([0-9,]+)K in swap \(([0-9,]+)K total swap\)",
            t,
        ) {
            let channel = if title.contains("meminfo") {
                "dumpsys_meminfo"
            } else if title.contains("PROCRANK") {
                "procrank"
            } else {
                "bugreport_procdump"
            };
            let provenance = if channel == "dumpsys_meminfo" {
                r.snapshot(i, channel)
            } else {
                r.provenance(
                    i,
                    channel,
                    "point_in_time_snapshot",
                    None,
                    "undetermined",
                    "none",
                )
            };
            let node = json!({"channel":channel,"line_start":l.no,"swap_total_kib":number(&c[3]),"swap_used_kib":number(&c[2]),"swap_used_kib_derived":false,"zram_physical_kib":number(&c[1]),"section_duration_ms":r.section(i).and_then(|s|s.duration).map(|s|(s*1000.).round() as i64),"provenance":provenance});
            snapshots.push(node);
            if channel == "dumpsys_meminfo" {
                swap["swap_total_kib"] = json!(number(&c[3]));
                swap["swap_used_kib"] = json!(number(&c[2]));
                swap["zram_physical_kib"] = json!(number(&c[1]));
                swap["provenance"] = r.snapshot(i, channel);
            }
        }
        if let Some(c) = captures(
            r"^Swap:\s*([0-9.,]+[KMG]?) total,\s*([0-9.,]+[KMG]?) used",
            t,
        ) {
            snapshots.push(json!({"channel":"top","line_start":l.no,"swap_total_kib":kib(&c[1]),"swap_used_kib":kib(&c[2]),"swap_used_kib_derived":false,"zram_physical_kib":null,"section_duration_ms":r.section(i).and_then(|s|s.duration).map(|s|s*1000.),"provenance":r.provenance(i,"top","point_in_time_snapshot",None,"undetermined","none")}));
        }
    }
    for (_, (total, free, i)) in rawswap {
        snapshots.push(json!({"channel":"proc_meminfo","line_start":r.lines[i].no,"swap_total_kib":total,"swap_used_kib":total.zip(free).and_then(|(a,b)|a.checked_sub(b)),"swap_used_kib_derived":true,"zram_physical_kib":null,"section_duration_ms":r.section(i).and_then(|s|s.duration).map(|s|s*1000.),"provenance":r.provenance(i,"proc_meminfo","point_in_time_snapshot",None,"undetermined","none")}));
    }
    snapshots.sort_by_key(|v| v["line_start"].as_u64().unwrap_or(0));
    for i in 1..snapshots.len() {
        let a = snapshots[i - 1]["line_start"].clone();
        let b = snapshots[i]["line_start"].clone();
        let delta = snapshots[i]["swap_used_kib"]
            .as_f64()
            .zip(snapshots[i - 1]["swap_used_kib"].as_f64())
            .map(|(a, b)| a - b);
        snapshots[i]["delta_from_previous"] =
            json!({"from_line_start":a,"to_line_start":b,"swap_used_delta_kib":delta});
    }
    system["free_ratio"] = json!(ratio(
        system["free_ram_kib"].as_f64(),
        system["total_ram_kib"].as_f64()
    ));
    system["total_pss_kib"] = json!(
        system["used_pss_kib"]
            .as_i64()
            .zip(system["free_ram_components_kib"]["cached_pss"].as_i64())
            .and_then(|(a, b)| a.checked_add(b))
    );
    system["total_pss_kib_derivation"] = json!("used_pss_kib + free_ram_components_kib.cached_pss");
    swap["swap_used_pct"] = json!(
        ratio(
            swap["swap_used_kib"].as_f64(),
            swap["swap_total_kib"].as_f64()
        )
        .map(|v| v * 100.)
    );
    swap["compression_ratio"] = json!(ratio(
        swap["swap_used_kib"].as_f64(),
        swap["zram_physical_kib"].as_f64()
    ));
    swap["zram_physical_pct_of_ram"] = json!(
        ratio(
            swap["zram_physical_kib"].as_f64(),
            system["total_ram_kib"].as_f64()
        )
        .map(|v| v * 100.)
    );
    swap["snapshots"] = json!(snapshots);
    let scan_sections:Vec<_>=scanning.iter().map(|i|{let s=&r.sections[*i];json!({"name":s.title,"line_start":r.lines[s.start].no,"line_end":r.lines[s.end].no,"duration_ms":s.duration.map(|v|v*1000.),"provenance":r.snapshot(s.start,"page_scanning")})}).collect();
    let sum: f64 = scan_sections
        .iter()
        .filter_map(|s| s["duration_ms"].as_f64())
        .sum();
    let growth = if snapshots.len() >= 2 {
        snapshots
            .last()
            .and_then(|n| n["swap_used_kib"].as_f64())
            .zip(snapshots.first().and_then(|n| n["swap_used_kib"].as_f64()))
            .map(|(b, a)| b - a)
    } else {
        None
    };
    let collection = json!({"page_scanning_sections":scan_sections,"page_scanning_total_ms":sum,"swap_timeline":snapshots,"swap_growth_kib":growth,"page_scanning_ms_in_interval":null});
    let pressure = json!({"psi_memory":psi(r,"memory"),"psi_io":psi(r,"io"),"vmstat_oom_kill":vmstat(r,"oom_kill"),"vmstat_pswpin":vmstat(r,"pswpin"),"vmstat_pswpout":vmstat(r,"pswpout")});
    let kills = lmk_kills(r);
    let io = iotop(r);
    let event_count = |tag: &str| {
        r.logs
            .iter()
            .filter(|l| l.tag == tag)
            .map(|l| (&l.raw_time, &l.message))
            .collect::<BTreeSet<_>>()
            .len()
    };
    let kill_evidence = json!({"lmkd_alive":r.logs.iter().any(|l|l.tag=="lmkd").then_some(true),"lmkd_using_psi":r.logs.iter().any(|l|l.tag=="lmkd"&&l.message.to_lowercase().contains("psi")).then_some(true),"vmstat_oom_kill":vmstat(r,"oom_kill"),"am_kill_count":event_count("am_kill"),"am_low_memory_count":event_count("am_low_memory"),"am_low_memory_value":r.logs.iter().find(|l|l.tag=="am_low_memory").map(|l|l.message.clone()),"am_low_memory_meaning":"This retained event denotes the platform's low-priority-background-process condition; it is not itself an OOM or a kill notification.","am_meminfo_count":event_count("am_meminfo"),"am_pss_count":event_count("am_pss")});
    if let Some(s) = r.find_section("SERVICE meminfo") {
        // A present section can lack recognized metrics; retain its locator without inventing data.
        let provenance = if system["provenance"].is_null() {
            r.snapshot(s.start, "dumpsys_meminfo")
        } else {
            system["provenance"].clone()
        };
        a.finding(super::FindingInput { kind: "memory_snapshot", severity: if system["memory_level"].as_str().is_some_and(|s|["critical","low"].contains(&s)){"high"}else{"info"}, confidence: "reported", process_key: None, attributes: json!({}), metrics: json!({"memory_level":system["memory_level"],"free_ratio":system["free_ratio"],"free_ram_kib":system["free_ram_kib"],"total_ram_kib":system["total_ram_kib"]}), evidence: r.evidence(s.start), provenance });
    }
    if let Some(n) = vmstat(r, "oom_kill")
        && n > 0
        && let Some((i, _)) = r
            .lines
            .iter()
            .enumerate()
            .find(|(_, l)| l.text.starts_with("oom_kill "))
    {
        a.finding(super::FindingInput {
            kind: "oom_counter",
            severity: "critical",
            confidence: "confirmed",
            process_key: None,
            attributes: json!({}),
            metrics: json!({"oom_kill_since_boot":n}),
            evidence: r.evidence(i),
            provenance: r.provenance(i, "vmstat", "command_output", None, "undetermined", "none"),
        });
    }
    for (j, k) in kills.iter().enumerate() {
        a.finding(super::FindingInput {
            kind: "lmkd_kill",
            severity: if k["oom_score_adj"].as_i64().is_some_and(|v| v <= 0) {
                "high"
            } else {
                "medium"
            },
            confidence: "reported",
            process_key: Some(format!(
                "{}#{}@{}",
                k["process_name"].as_str().unwrap_or("?"),
                k["uid"],
                j + 1
            )),
            attributes: json!({"process":k["process_name"],"pid":k["pid"]}),
            metrics: json!({"rss_kib":k["rss_kib"],"oom_score_adj":k["oom_score_adj"]}),
            evidence: k["evidence"].clone(),
            provenance: k["provenance"].clone(),
        });
    }
    for tag in ["am_kill", "am_low_memory", "am_meminfo", "am_pss"] {
        a.count(
            r,
            &format!("{tag}_hits"),
            &format!("{tag}:"),
            event_count(tag),
        );
    }
    a.count(r, "lmkd_kill_hits", "Kill '", kills.len());
    if kills.is_empty() {
        a.absence(r, "lmkd_kill_hits", "Kill '");
    }
    let stats = procstats(r);
    for b in &stats {
        if b["provenance"]["freshness"] == "stale" {
            let line = b["provenance"]["line_start"]
                .as_u64()
                .unwrap_or(1)
                .saturating_sub(1) as usize;
            a.finding(super::FindingInput {
                kind: "historical_memory_aggregate",
                severity: "info",
                confidence: "confirmed",
                process_key: None,
                attributes: json!({"sub_section":b["sub_section"]}),
                metrics: json!({"aggregated_over_n":b["aggregated_over_n"]}),
                evidence: r.evidence(line),
                provenance: b["provenance"].clone(),
            });
        }
    }
    let mut channels = Vec::new();
    for (name, title) in [
        ("proc_meminfo", "MEMORY INFO"),
        ("dumpsys_meminfo", "SERVICE meminfo"),
        ("procrank", "PROCRANK"),
        ("procstats", "procstats"),
        ("vmstat", "VM STAT"),
        ("iotop", "IOTOP"),
    ] {
        channels.push(json!({"channel":name,"present":r.find_section(title).is_some(),"provenance":r.find_section(title).map(|s|r.snapshot(s.start,name))}));
    }
    a.set("channels", json!(channels));
    a.set("system", system);
    a.set("swap", swap);
    a.set("processes", json!(processes));
    a.set("categories", json!(categories));
    a.set("kernel_memory",json!({"slab":r.section_lines("MEMORY INFO").find_map(|(_,l)|l.text.strip_prefix("Slab:").and_then(|v|v.split_whitespace().next()).and_then(number)),"pagetables":r.section_lines("MEMORY INFO").find_map(|(_,l)|l.text.strip_prefix("PageTables:").and_then(|v|v.split_whitespace().next()).and_then(number)),"stacks":r.section_lines("MEMORY INFO").find_map(|(_,l)|l.text.strip_prefix("KernelStack:").and_then(|v|v.split_whitespace().next()).and_then(number)),"cma":r.section_lines("MEMORY INFO").find_map(|(_,l)|l.text.strip_prefix("CmaTotal:").and_then(|v|v.split_whitespace().next()).and_then(number))}));
    for key in ["slab", "pagetables", "stacks", "cma"] {
        let old = a.data["kernel_memory"][key].clone();
        if let Some(kernel) = a.data.get_mut("kernel_memory") {
            kernel[key] = if old.is_null() {
                Value::Null
            } else {
                json!({"total_kib":old,"source":"/proc/meminfo","provenance":r.find_section("MEMORY INFO").map(|s|r.provenance(s.start,"proc_meminfo","command_output",None,"undetermined","none"))})
            };
        }
    }
    a.set("procstats", json!(stats));
    a.set("pressure_indicators", pressure);
    a.set("kills", json!(kills));
    a.set("kill_evidence", kill_evidence);
    a.set(
        "io",
        json!({"iotop":io,"storaged":null,"zram_block_io":null}),
    );
    a.gaps.push(gap("collection.page_scanning_ms_in_interval","PARSE_INDETERMINATE","Some swap readings carry no execution clock; line order cannot establish temporal overlap or collector causation."));
    let mut block_io = Vec::new();
    for (i, l) in r.lines.iter().enumerate() {
        if r.section(i)
            .and_then(|s| s.command.as_deref())
            .is_some_and(|c| c.contains("/sys/block/zram") && c.contains("/stat"))
        {
            let fields = l
                .text
                .split_whitespace()
                .map(number)
                .collect::<Option<Vec<_>>>();
            if let Some(raw) = fields.filter(|v| v.len() >= 11) {
                block_io.push(json!({"device_source":r.section(i).and_then(|s|s.command.as_deref()),"raw_counters":raw,"cumulative_since_boot":true,"sector_size_bytes":null,"provenance":r.provenance(i,"zram_block_io","command_output",None,"undetermined","none")}));
            }
        }
    }
    if !block_io.is_empty() {
        let v = json!({"devices":block_io});
        if let Some(swap) = a.data.get_mut("swap") {
            swap["zram_block_io"] = v.clone();
        }
        if let Some(io) = a.data.get_mut("io") {
            io["zram_block_io"] = v;
        }
    }
    let mut storage_rows = Vec::new();
    for (i, l) in r.section_lines("storaged") {
        let values = l
            .text
            .split_whitespace()
            .map(number)
            .collect::<Option<Vec<_>>>();
        if let Some(row) = values.filter(|v| v.len() > 2) {
            storage_rows.push(json!({"raw_counters":row,"provenance":r.snapshot(i,"storaged")}));
        }
    }
    if !storage_rows.is_empty()
        && let Some(io) = a.data.get_mut("io")
    {
        io["storaged"] = json!({"rows":storage_rows,"measurement_kind":"cumulative_since_process_start","window":null});
    }
    (a, collection)
}

fn cpu_sample(r: &Report, index: usize, channel: &str, time: Option<Clock>) -> Value {
    let mut v = object(
        "total_pct pct_user pct_kernel pct_iowait pct_irq pct_softirq pct_nice pct_steal online_cpus configured_cpus threads_total running sleeping stopped zombie age_at_print_ms",
    );
    v["channel"] = json!(channel);
    v["line_start"] = json!(r.lines[index].no);
    v["window"] = object("start_utc end_utc duration_ms");
    v["window_source"] = json!("unknown");
    v["null_reasons"] = json!({});
    v["provenance"] = r.provenance(
        index,
        channel,
        if channel == "dumpsys_cpuinfo" {
            "background_sampler"
        } else {
            "command_output"
        },
        time,
        "dumpsys_section_snapshot",
        "dumpsys_Timestamp",
    );
    v
}
fn cpu(r: &Report, capture: &Value, apps: &Artifact, io: &Value) -> Artifact {
    let mut a = Artifact::new(
        "cpu",
        r,
        &[
            "SERVICE cpuinfo",
            "CPU INFO",
            "cpu_monitor",
            "SYSTEM LOG",
            "EVENT LOG",
        ],
        false,
    );
    let mut samples: Vec<Value> = Vec::new();
    let mut processes = Vec::new();
    let mut current = None;
    let mut cores = Vec::new();
    let mut cumulative = false;
    let mut read_uptime = None;
    let mut online = BTreeMap::new();
    let mut load_sources = Vec::new();
    for (i, l) in r.lines.iter().enumerate() {
        let t = l.text.trim();
        let title = r.title(i);
        if title.contains("cpu_monitor") {
            if let Some(n) = token_num(t, "mLastReadUptimeMillis = ") {
                read_uptime = Some(n);
            }
            if let Some(c) = captures(r"cpuCore = (\d+).*?isOnline = (Yes|No)", t) {
                online.insert(number(&c[1]).unwrap_or(0), c[2] == "Yes");
            }
            if t == "Latest cumulative CPU usage stats by CPU core:" {
                cumulative = true;
            } else if cumulative
                && let Some(c) = captures(r"^CPU core id = (\d+), CpuUsageStats\{(.*)\}", t)
            {
                let mut core = object(
                    "core_id is_online user_time_ms nice_time_ms system_time_ms idle_time_ms iowait_time_ms irq_time_ms softirq_time_ms steal_time_ms guest_time_ms guest_nice_time_ms busy_pct_since_boot busy_pct_derivation read_at_uptime_ms provenance",
                );
                let id = number(&c[1]);
                core["core_id"] = json!(id);
                core["is_online"] = json!(id.and_then(|id| online.get(&id).copied()));
                for (key, raw) in [
                    ("user_time_ms", "userTimeMillis = "),
                    ("nice_time_ms", "niceTimeMillis = "),
                    ("system_time_ms", "systemTimeMillis = "),
                    ("idle_time_ms", "idleTimeMillis = "),
                    ("iowait_time_ms", "iowaitTimeMillis = "),
                    ("irq_time_ms", "irqTimeMillis = "),
                    ("softirq_time_ms", "softirqTimeMillis = "),
                    ("steal_time_ms", "stealTimeMillis = "),
                    ("guest_time_ms", "guestTimeMillis = "),
                    ("guest_nice_time_ms", "guestNiceTimeMillis = "),
                ] {
                    core[key] = json!(token_num(&c[2], raw));
                }
                let busy = [
                    "user_time_ms",
                    "nice_time_ms",
                    "system_time_ms",
                    "irq_time_ms",
                    "softirq_time_ms",
                ]
                .iter()
                .map(|k| core[*k].as_i64())
                .collect::<Option<Vec<_>>>()
                .and_then(|v| v.into_iter().try_fold(0i64, |sum, n| sum.checked_add(n)));
                let total = busy
                    .zip(core["idle_time_ms"].as_i64())
                    .zip(core["iowait_time_ms"].as_i64())
                    .zip(core["steal_time_ms"].as_i64())
                    .and_then(|(((a, b), c), d)| a.checked_add(b)?.checked_add(c)?.checked_add(d));
                core["busy_pct_since_boot"] =
                    json!(ratio(busy.map(|v| v as f64), total.map(|v| v as f64)).map(|v| v * 100.));
                core["busy_pct_derivation"] = json!(
                    "100*(user+nice+system+irq+softirq)/(user+nice+system+idle+iowait+irq+softirq+steal); guest counters already included in user/nice"
                );
                core["read_at_uptime_ms"] = json!(read_uptime);
                core["provenance"] = r.provenance(
                    i,
                    "cpu_monitor",
                    "command_output",
                    None,
                    "undetermined",
                    "none",
                );
                cores.push(core);
            } else if cumulative && !t.starts_with("CPU core id") && !t.is_empty() {
                cumulative = false;
            }
        } else {
            cumulative = false;
        }
        if title.contains("cpuinfo") {
            if let Some(c) = captures(
                r"^CPU usage from (-?\d+)ms to (-?\d+)ms ago \((.+) to (.+)\):",
                t,
            ) {
                let start = parse_clock(&c[3], r.offset);
                let end = parse_clock(&c[4], r.offset);
                let mut s = cpu_sample(r, i, "dumpsys_cpuinfo", end);
                s["window"] = json!({"start_utc":start.as_ref().map(stamp),"end_utc":end.as_ref().map(stamp),"duration_ms":start.zip(end).map(|(a,b)|(b-a).num_milliseconds())});
                s["window_source"] = json!("section_body");
                s["age_at_print_ms"] = json!({"start":number(&c[1]),"end":number(&c[2])});
                if end.zip(r.capture_time).is_some_and(|(e, c)| e < c) {
                    s["provenance"]["collector_perturbed"] = json!(false);
                    s["provenance"]["collector_perturbed_basis"] =
                        json!("section_measured_before_dump");
                }
                samples.push(s);
                current = Some(samples.len() - 1);
            }
            if let Some(c) = captures(r"^([0-9.]+)% TOTAL:(.*)$", t)
                && let Some(j) = current
            {
                samples[j]["total_pct"] = json!(decimal(&c[1]));
                for (raw, key) in [
                    ("user", "pct_user"),
                    ("kernel", "pct_kernel"),
                    ("iowait", "pct_iowait"),
                    ("irq", "pct_irq"),
                    ("softirq", "pct_softirq"),
                ] {
                    let pattern = match raw {
                        "user" => r"([0-9.]+)% user",
                        "kernel" => r"([0-9.]+)% kernel",
                        "iowait" => r"([0-9.]+)% iowait",
                        "irq" => r"([0-9.]+)% irq",
                        _ => r"([0-9.]+)% softirq",
                    };
                    samples[j][key] = json!(
                        captures(pattern, &c[2])
                            .and_then(|v| decimal(&v[1]))
                            .unwrap_or(0.)
                    );
                }
            }
            if let Some(c) = captures(
                r"^([0-9.]+)% (\d+)/([^:]+):\s*([0-9.]+)% user \+ ([0-9.]+)% kernel(.*)$",
                t,
            ) && let Some(j) = current
            {
                processes.push(json!({"pid":number(&c[2]),"label":c[3],"pct_total":decimal(&c[1]),"pct_user":decimal(&c[4]),"pct_kernel":decimal(&c[5]),"minor_faults":captures(r"(\d+) minor",&c[6]).and_then(|v|number(&v[1])),"major_faults":captures(r"(\d+) major",&c[6]).and_then(|v|number(&v[1])),"is_thread_row":l.text.len()-l.text.trim_start().len()>2,"window_ref":format!("cpu.json#/utilization/{j}/window"),"provenance":samples[j]["provenance"]}));
            }
        }
        if let Some(c) = captures(r"^(\d+)%cpu\s+(.*)$", t) {
            let mut s = cpu_sample(r, i, "top", None);
            s["configured_cpus"] = json!(number(&c[1]).map(|n| n / 100));
            for (raw, key) in [
                ("user", "pct_user"),
                ("nice", "pct_nice"),
                ("sys", "pct_kernel"),
                ("iow", "pct_iowait"),
                ("irq", "pct_irq"),
                ("sirq", "pct_softirq"),
                ("host", "pct_steal"),
            ] {
                let pattern = match raw {
                    "user" => r"([0-9.]+)%user",
                    "nice" => r"([0-9.]+)%nice",
                    "sys" => r"([0-9.]+)%sys",
                    "iow" => r"([0-9.]+)%iow",
                    "irq" => r"([0-9.]+)%irq",
                    "sirq" => r"([0-9.]+)%sirq",
                    _ => r"([0-9.]+)%host",
                };
                s[key] = json!(captures(pattern, &c[2]).and_then(|v| decimal(&v[1])));
            }
            s["null_reasons"]["total_pct"] = json!(
                "Top's aggregate buckets use 100 percent per configured CPU, not the cpuinfo denominator; no merged busy percentage is published."
            );
            samples.push(s);
            current = None;
        }
        if let Some(c) = captures(
            r"^Threads: (\d+) total,\s*(\d+) running,\s*(\d+) sleeping,\s*(\d+) stopped,\s*(\d+) zombie",
            t,
        ) {
            let mut pending = json!({});
            for (j, k) in ["threads_total", "running", "sleeping", "stopped", "zombie"]
                .iter()
                .enumerate()
            {
                pending[*k] = json!(number(&c[j + 1]));
            }
            if let Some(s) = samples.last_mut().filter(|s| s["channel"] == "top") {
                merge(s, pending);
            } else {
                a.data.insert("top_threads_pending".into(), pending);
            }
        }
        if t.contains("load average:") || t.starts_with("Load:") {
            let suffix = t
                .split_once("load average:")
                .or_else(|| t.split_once("Load:"))
                .map(|(_, s)| s)
                .unwrap_or("");
            let vals: Vec<_> = suffix.split([',', ' ']).filter_map(decimal).collect();
            if vals.len() >= 3 {
                load_sources.push(json!({"source":if i<100{"bugreport_header"}else if title=="UPTIME"{"uptime_section"}else{"dumpsys_cpuinfo"},"load_1m":vals[0],"load_5m":vals[1],"load_15m":vals[2],"uptime_ms_at_read":capture["uptime_at_capture_seconds"].as_i64().map(|v|v*1000),"uptime_qualifier":if capture["uptime_at_capture_seconds"].is_null(){"unresolved"}else{"derived_capture_context_not_simultaneous_sample"},"line_start":l.no}));
            }
        }
    }
    if let Some(pending) = a.data.remove("top_threads_pending") {
        for sample in samples.iter_mut().filter(|s| s["channel"] == "top") {
            merge(sample, pending.clone());
        }
    }
    let observed_online = if online.is_empty() {
        None
    } else {
        Some(online.values().filter(|v| **v).count())
    };
    for s in &mut samples {
        s["online_cpus"] = json!(observed_online);
        for key in [
            "total_pct",
            "pct_user",
            "pct_kernel",
            "pct_iowait",
            "pct_irq",
            "pct_softirq",
            "pct_nice",
            "pct_steal",
        ] {
            if s[key].is_null() && s["null_reasons"][key].is_null() {
                s["null_reasons"][key] =
                    json!(if s["channel"] == "dumpsys_cpuinfo" && key == "pct_nice" {
                        "Nice time is combined with user in this channel"
                    } else if s["channel"] == "dumpsys_cpuinfo" && key == "pct_steal" {
                        "Steal is not emitted by this aggregate"
                    } else {
                        "The measurement does not print this bucket"
                    });
            }
        }
    }
    let mut monitor = Vec::new();
    let mut sampled = Vec::new();
    let mut seen = BTreeSet::new();
    let mut mutex = Vec::new();
    for log in &r.logs {
        let key = (&log.raw_time, log.pid, log.tid, &log.tag, &log.message);
        if !seen.insert(key) {
            continue;
        }
        if log.message.starts_with("Long monitor contention")
            && let Some(c) = captures(
                r"with owner (.*?) \((\d+)\) at (.*) waiters=(\d+) in (.*?) for ([0-9.]+)(ms|s)$",
                &log.message,
            )
        {
            let value = decimal(&c[6]);
            let wait = value.map(|v| if c[7] == "s" { v * 1000. } else { v });
            let location = c[3].rsplit_once(")(");
            let method = location
                .map(|(m, _)| format!("{m})"))
                .unwrap_or(c[3].clone());
            let source = location.map(|(_, s)| s.trim_end_matches(')'));
            let file_line = source.and_then(|s| s.rsplit_once(':'));
            monitor.push(json!({"timestamp":log.time.as_ref().map(stamp),"process":log.tag,"pid":log.pid,"tid":log.tid,"owner_name":c[1],"owner_tid":number(&c[2]),"owner_method":method,"owner_file":file_line.map(|(f,_)|f),"owner_line":file_line.and_then(|(_,l)|number(l)),"waiters":number(&c[4]),"waiter_method":c[5],"wait_ms":wait,"wait_verbatim":format!("{}{}",c[6],c[7]),"provenance":r.log_provenance(log,"art_java_monitor"),"evidence":r.evidence(log.index)}));
        }
        if log.tag == "dvm_lock_sample"
            && let Some(payload) = log
                .message
                .trim()
                .strip_prefix('[')
                .and_then(|v| v.strip_suffix(']'))
        {
            let f: Vec<_> = payload.split(',').collect();
            if f.len() >= 11 {
                sampled.push(json!({"timestamp":log.time.as_ref().map(stamp),"process":f[0],"is_sensitive":number(f[1]),"thread_name":f[2],"wait_ms":number(f[3]),"waiter_file":f[4],"waiter_line":number(f[5]),"waiter_method":f[6],"owner_file":if f[7]=="-"{f[4]}else{f[7]},"owner_line":number(f[8]),"owner_method":f[9],"sample_percent":number(f[10]),"provenance":r.log_provenance(log,"dvm_lock_sample")}));
            }
        }
        if log.message.contains("contended for") && log.message.contains("mutex") {
            mutex.push(r.evidence(log.index));
        }
    }
    for (i, event) in monitor.iter().enumerate() {
        let wait = event["wait_ms"].as_f64().unwrap_or(0.);
        a.finding(super::FindingInput {
            kind: "monitor_contention",
            severity: if wait >= 1000. { "medium" } else { "low" },
            confidence: "confirmed",
            process_key: Some(format!(
                "{}#unresolved@{}",
                event["process"].as_str().unwrap_or("?"),
                i + 1
            )),
            attributes: json!({"process":event["process"],"owner_method":event["owner_method"]}),
            metrics: json!({"wait_ms":wait,"waiters":event["waiters"]}),
            evidence: event["evidence"].clone(),
            provenance: event["provenance"].clone(),
        });
    }
    let waits: Vec<_> = monitor
        .iter()
        .filter_map(|e| e["wait_ms"].as_f64())
        .collect();
    let ss = sampled
        .iter()
        .filter(|v| v["process"] == "system_server")
        .count();
    let contention = json!({"art_java_monitor":{"event_count":monitor.len(),"channel_present":!monitor.is_empty(),"threshold_ms":null,"wait_ms_max":waits.iter().copied().reduce(f64::max),"wait_ms_min":waits.iter().copied().reduce(f64::min),"events_in_system_server":monitor.iter().filter(|v|v["process"]=="system_server").count(),"events":monitor},"art_runtime_mutex":{"channel_present":!mutex.is_empty(),"event_count":mutex.len(),"format":null,"short_format":null,"zero_reason":if mutex.is_empty(){Some("No structurally matched retained mutex records; enablement is unknown")}else{None},"evidence":mutex},"art_rate_limiting":{"java_monitor_is_sampled":null,"java_monitor_sampling_rule":null,"java_monitor_sample_pct_observed":sampled.iter().filter_map(|v|v["sample_percent"].as_i64()).collect::<BTreeSet<_>>(),"java_monitor_threshold_source":null,"runtime_mutex_is_rate_limited":null,"runtime_mutex_rate_limit":null,"counting_caveat":"Retained monitor messages and dvm_lock_sample rows are separate sampled channels, not a complete contention-event count."},"dvm_lock_sample":{"sampled_event_count":sampled.len(),"in_system_server":ss,"system_server_ratio":ratio(Some(ss as f64),Some(sampled.len() as f64)),"field_order_source":"android-15.0.0_r17: monitor_android.cc; OEM layouts require independent confirmation","example":sampled.first(),"events":sampled},"provenance":r.provenance(0,"contention","live_log",None,"undetermined","none")});
    for sample in &samples {
        if let Some(pct) = sample["total_pct"].as_f64() {
            let idx = sample["line_start"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
            a.finding(super::FindingInput { kind: "cpu_utilization", severity: if pct >= 80. { "medium" } else { "info" }, confidence: "confirmed", process_key: None, attributes: json!({"channel":sample["channel"]}), metrics: json!({"total_pct":pct,"window":sample["window"],"pct_irq":sample["pct_irq"]}), evidence: r.evidence(idx), provenance: sample["provenance"].clone() });
        }
    }
    let lifecycle = apps.data.get("lifecycle_events").and_then(Value::as_array);
    let event_count = |tag: &str| {
        lifecycle
            .map(|v| v.iter().filter(|e| e["kind"] == tag).count())
            .unwrap_or(0)
    };
    let event_window = &apps.data["inventory"]["counts"]["lifecycle_channel_window"];
    let duration = event_window["duration_seconds"].as_f64();
    let rate_valid = duration.is_some_and(|v| v >= 120.);
    let churn = json!({"am_proc_start_count":event_count("am_proc_start"),"am_proc_died_count":event_count("am_proc_died"),"am_proc_bound_count":event_count("am_proc_bound"),"am_uid_running_count":r.logs.iter().filter(|l|l.tag=="am_uid_running").count(),"am_uid_active_count":r.logs.iter().filter(|l|l.tag=="am_uid_active").count(),"window":{"first":event_window["first_event_at"],"last":event_window["last_event_at"],"duration_seconds":duration},"rate_per_s":if rate_valid{ratio(Some(event_count("am_proc_start") as f64),duration)}else{None},"rate_valid":rate_valid,"rate_suppressed_reason":if rate_valid{None}else{Some("observation_window_shorter_than_120_seconds")}});
    let graphics = graphics(r);
    let power = power(r, capture);
    for (key, pattern, matched) in [
        (
            "long_monitor_contention_hits",
            "Long monitor contention",
            waits.len(),
        ),
        (
            "dvm_lock_sample_hits",
            "dvm_lock_sample:",
            contention["dvm_lock_sample"]["sampled_event_count"]
                .as_u64()
                .unwrap_or(0) as usize,
        ),
    ] {
        a.count(r, key, pattern, matched);
    }
    a.gaps.push(gap(
        "contention.art_java_monitor.threshold_ms",
        "UNVERIFIABLE",
        "Build type does not establish the loaded ART library's compile-time threshold.",
    ));
    a.set("channels",json!([{"channel":"dumpsys_cpuinfo","present":r.find_section("cpuinfo").is_some()},{"channel":"top","present":r.find_section("CPU INFO").is_some()},{"channel":"cpu_monitor","present":r.find_section("cpu_monitor").is_some()}]));
    let clean: Vec<_> = samples
        .iter()
        .filter(|s| s["provenance"]["collector_perturbed"] == false)
        .map(|s| s["channel"].clone())
        .collect();
    a.set(
        "collection",
        json!({"cpu_clean_channels":clean,"cpu_perturbed_channels":[],"mechanism":null}),
    );
    a.set("utilization", json!(samples));
    a.set("per_cpu_core", json!(cores));
    a.set("processes", json!(processes));
    a.set("contention", contention);
    a.set("load",json!({"sources":load_sources,"uptime_ms_at_read":capture["uptime_at_capture_seconds"].as_i64().map(|n|n*1000),"online_cpus":observed_online,"provenance":r.provenance(0,"load","command_output",None,"undetermined","none")}));
    a.set(
        "stall_signals",
        json!({"psi_cpu":psi(r,"cpu"),"psi_io":psi(r,"io"),"iotop":io,"proc_churn":churn}),
    );
    a.set("graphics", graphics);
    a.set("power_history", power);
    a
}
fn graphics(r: &Report) -> Value {
    let mut records = Vec::new();
    let mut current: Option<Value> = None;
    for (i, l) in r
        .lines
        .iter()
        .enumerate()
        .filter(|(i, _)| r.title(*i).contains("gfxinfo"))
    {
        let t = l.text.trim();
        if let Some(c) = captures(r"^\*\* Graphics info for pid (\d+) \[([^\]]+)\]", t) {
            if let Some(v) = current.take() {
                records.push(v);
            }
            current = Some(
                json!({"pid":number(&c[1]),"process_name":c[2],"stats_since_ns":null,"total_frames":null,"janky_frames":null,"janky_pct":null,"p50_ms":null,"p90_ms":null,"p95_ms":null,"p99_ms":null,"provenance":r.snapshot(i,"gfxinfo")}),
            );
        }
        if let Some(v) = current.as_mut() {
            for (prefix, key) in [
                ("Stats since:", "stats_since_ns"),
                ("Total frames rendered:", "total_frames"),
                ("Janky frames:", "janky_frames"),
                ("50th percentile:", "p50_ms"),
                ("90th percentile:", "p90_ms"),
                ("95th percentile:", "p95_ms"),
                ("99th percentile:", "p99_ms"),
            ] {
                if let Some(raw) = t.strip_prefix(prefix) {
                    v[key] = json!(
                        raw.split_whitespace()
                            .next()
                            .and_then(|n| number(n.trim_end_matches("ms")))
                    );
                }
            }
            if t.starts_with("Janky frames:")
                && let Some(c) = captures(r"\(([0-9.]+)%\)", t)
            {
                v["janky_pct"] = json!(decimal(&c[1]));
            }
        }
    }
    if let Some(v) = current {
        records.push(v);
    }
    if records.is_empty() {
        Value::Null
    } else {
        json!({"per_process":records,"window_note":"Each process has an independent stats_since_ns; these frame windows are not the CPU sampling window."})
    }
}
fn power(r: &Report, capture: &Value) -> Value {
    let mut v = object(
        "batterystats_reset_time batterystats_duration_s powerstats_earliest_data_time quarantine_reason",
    );
    let mut found = false;
    for (i, l) in r.lines.iter().enumerate() {
        let t = l.text.trim();
        if r.title(i).contains("batterystats") && t.starts_with("Start clock time:") {
            v["batterystats_reset_time"] = json!(t.trim_start_matches("Start clock time:").trim());
            found = true;
        }
        if r.title(i).contains("powerstats") && t.starts_with("Earliest data time") {
            v["powerstats_earliest_data_time"] = json!(t.split_once(':').map(|(_, v)| v.trim()));
            found = true;
        }
    }
    if found {
        v["quarantine_reason"] = json!(if capture["boot"]["at"].is_null() {
            "No common observation interval with CPU utilization is established"
        } else {
            "Persisted history retains its own start clock and is not joined to the current CPU sample"
        });
        v
    } else {
        Value::Null
    }
}
pub fn analyze(r: &Report, capture: &Value, apps: &Artifact) -> (Artifact, Artifact, Value) {
    let (memory, collection) = memory(r);
    let io = memory.data["io"]["iotop"].clone();
    let cpu = cpu(r, capture, apps, &io);
    (memory, cpu, collection)
}
