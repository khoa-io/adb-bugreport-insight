//! Executable artifact invariants, checked before files can be published.
use super::{InsightError, common::parse_clock};
use serde_json::Value;
use std::collections::BTreeSet;

fn require(condition: bool, message: impl Into<String>) -> Result<(), InsightError> {
    if condition {
        Ok(())
    } else {
        Err(InsightError::Invalid(message.into()))
    }
}
fn fields(v: &Value, keys: &str, context: &str) -> Result<(), InsightError> {
    for key in keys.split_whitespace() {
        require(v.get(key).is_some(), format!("{context}: missing {key}"))?;
    }
    Ok(())
}
fn excerpts(v: &Value) -> Result<(), InsightError> {
    match v {
        Value::Object(map) => {
            if let Some(text) = map.get("excerpt").and_then(Value::as_str) {
                fields(
                    v,
                    "excerpt_bytes excerpt_truncated excerpt_complete",
                    "evidence",
                )?;
                require(text.len() <= 512, "evidence excerpt exceeds 512 bytes")?;
                require(
                    v["excerpt_bytes"].as_u64() == Some(text.len() as u64),
                    "excerpt byte count disagrees",
                )?;
                require(
                    !text.contains('\0') || v["excerpt_complete"] == false,
                    "NUL excerpt claimed complete",
                )?;
            }
            for value in map.values() {
                excerpts(value)?;
            }
        }
        Value::Array(array) => {
            for value in array {
                excerpts(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn artifacts(values: &[Value]) -> Result<(), InsightError> {
    require(values.len() == 6, "six JSON artifacts required")?;
    let mut ids = BTreeSet::new();
    for (artifact, name) in values
        .iter()
        .zip(["system", "apps", "anr", "crashes", "memory", "cpu"])
    {
        require(artifact.is_object(), "artifact root must be object")?;
        require(
            artifact["source"] == values[0]["source"]
                && artifact["generator"] == values[0]["generator"],
            "artifacts must share source identity and generator context",
        )?;
        require(artifact["artifact"] == name, "artifact order/name mismatch")?;
        fields(
            artifact,
            "schema_version artifact generator source scope collection counts severity_summary freshness_summary negative_evidence evidence_gaps findings notes",
            name,
        )?;
        if name == "system" {
            fields(
                artifact,
                "capture build product device kernel users displays hal storage archive_entries fact_evidence",
                name,
            )?;
            require(artifact.get("capture_ref").is_none(), "system owns capture")?;
        } else {
            fields(artifact, "capture_ref", name)?;
            require(
                artifact["capture_ref"]["source"] == "system.json#/capture",
                "invalid capture reference",
            )?;
            require(
                artifact.get("capture").is_none(),
                "only system owns capture",
            )?;
        }
        fields(
            artifact,
            match name {
                "apps" => "channels inventory lifecycle_events activity_events attribution",
                "anr" => "channels anr_incidents trace_files last_anr_trace",
                "crashes" => {
                    "channels count_authority java_crash_groups java_crash_incidents native_crash_groups native_crash_incidents"
                }
                "memory" => {
                    "channels system swap processes categories kernel_memory procstats pressure_indicators kills kill_evidence io"
                }
                "cpu" => {
                    "channels utilization per_cpu_core processes contention load stall_signals graphics power_history"
                }
                _ => "",
            },
            name,
        )?;
        let sha = artifact["source"]["archive_sha256"].as_str().unwrap_or("");
        require(
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "stored archive SHA-256 is required",
        )?;
        fields(
            &artifact["source"],
            "archive_name archive_size_bytes archive_sha256 format_version main_entry main_entry_size_bytes main_entry_line_count zip_entry_count device",
            "source",
        )?;
        for key in ["run_started_at", "run_finished_at"] {
            require(
                artifact["generator"][key]
                    .as_str()
                    .and_then(|s| parse_clock(s, None))
                    .is_some(),
                format!("generator.{key} must be RFC3339"),
            )?;
        }
        let findings = artifact["findings"]
            .as_array()
            .ok_or_else(|| InsightError::Invalid("findings must be array".into()))?;
        for f in findings {
            fields(
                f,
                "id kind severity unconstrained_severity confidence process_key event_time attributes metrics evidence corroborating_evidence provenance",
                "finding",
            )?;
            require(
                ids.insert(f["id"].as_str().unwrap_or("").to_string()),
                "finding ids must be unique",
            )?;
            require(
                ["critical", "high", "medium", "low", "info"]
                    .contains(&f["severity"].as_str().unwrap_or("")),
                "invalid severity",
            )?;
            require(
                ["confirmed", "inferred", "reported"]
                    .contains(&f["confidence"].as_str().unwrap_or("")),
                "invalid confidence",
            )?;
            let attributes = f["attributes"]
                .as_object()
                .ok_or_else(|| InsightError::Invalid("attributes must be object".into()))?;
            require(
                attributes
                    .values()
                    .all(|v| v.is_string() || v.is_number() || v.is_boolean()),
                "attributes must contain scalar values only",
            )?;
            if matches!(f["severity"].as_str(), Some("critical" | "high")) {
                require(
                    f["metrics"].as_object().is_some_and(|m| !m.is_empty()),
                    "high severity requires metrics",
                )?;
            }
            let p = &f["provenance"];
            fields(
                p,
                "evidence_origin archive_path section_title section_ordinal line_start line_end captured_at captured_at_time_base evidence_time_source age_seconds_at_capture capture_kind freshness freshness_basis cross_boot_reference collector_perturbed collector_perturbed_basis",
                &format!("{name}: {} provenance", f["kind"]),
            )?;
            match p["freshness"].as_str() {
                Some("stale" | "undated") => require(
                    !matches!(f["severity"].as_str(), Some("high" | "critical")),
                    "stale/undated severity exceeds medium",
                )?,
                Some("now" | "in_window") => {
                    require(
                        p["captured_at"]
                            .as_str()
                            .and_then(|s| parse_clock(s, None))
                            .is_some(),
                        "current finding needs clock",
                    )?;
                    require(
                        p["age_seconds_at_capture"]
                            .as_i64()
                            .is_some_and(|n| (0..3600).contains(&n)),
                        "current finding needs bounded age",
                    )?;
                    require(
                        artifact["scope"]["truncated"] == false,
                        "truncated scope cannot claim current findings",
                    )?;
                }
                _ => return Err(InsightError::Invalid("invalid freshness".into())),
            }
            if p["freshness"] == "undated" {
                require(
                    p["captured_at"].is_null() && p["age_seconds_at_capture"].is_null(),
                    "undated clock must be null",
                )?;
            }
            fields(
                &f["evidence"],
                "archive_path kind locator excerpt excerpt_truncated excerpt_bytes excerpt_complete",
                "evidence",
            )?;
        }
        for (field, keys) in [
            ("severity_summary", "critical high medium low info"),
            ("freshness_summary", "now in_window stale undated"),
        ] {
            let mut total = 0u64;
            for key in keys.split_whitespace() {
                let expected = findings
                    .iter()
                    .filter(|f| {
                        if field == "severity_summary" {
                            f["severity"] == key
                        } else {
                            f["provenance"]["freshness"] == key
                        }
                    })
                    .count();
                require(
                    artifact[field][key].as_u64() == Some(expected as u64),
                    format!("{field}.{key} mismatch"),
                )?;
                total += expected as u64;
            }
            require(
                total == findings.len() as u64,
                "summary does not total findings",
            )?;
        }
        if let Some(counts) = artifact["counts"].as_object() {
            for (key, c) in counts {
                require(
                    ["_hits", "_records", "_entries"]
                        .iter()
                        .any(|s| key.ends_with(s)),
                    "invalid count key suffix",
                )?;
                fields(
                    c,
                    "pattern pattern_kind case_sensitive raw_count matched_evidence suppressed_noise not_emitted_by_platform coverage regions",
                    "count block",
                )?;
                let raw = c["raw_count"].as_u64().unwrap_or(0);
                let sum = [
                    "matched_evidence",
                    "suppressed_noise",
                    "not_emitted_by_platform",
                ]
                .iter()
                .map(|k| c[*k].as_u64().unwrap_or(0))
                .sum::<u64>();
                require(raw == sum, "count partition does not close")?;
                if c["suppressed_noise"].as_u64().unwrap_or(0) > 0 {
                    require(
                        c["suppressed_noise_note"].is_string(),
                        "suppression requires explanation",
                    )?;
                }
            }
        }
        require(
            !findings.is_empty()
                || artifact["negative_evidence"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                || artifact["notes"].as_array().is_some_and(|v| !v.is_empty()),
            "empty findings need explanation",
        )?;
        for gap in artifact["evidence_gaps"].as_array().into_iter().flatten() {
            require(
                [
                    "NOT_PRINTED",
                    "SECTION_ABSENT",
                    "SECTION_FAILED",
                    "REGION_EXCLUDED",
                    "PARSE_INDETERMINATE",
                    "AMBIGUOUS_SOURCE",
                    "UNVERIFIABLE",
                ]
                .contains(&gap["reason"].as_str().unwrap_or("")),
                "invalid evidence gap reason",
            )?;
        }
        domain_fields(artifact)?;
        excerpts(artifact)?;
    }
    Ok(())
}

fn array_fields(value: &Value, keys: &str, context: &str) -> Result<(), InsightError> {
    let rows = value
        .as_array()
        .ok_or_else(|| InsightError::Invalid(format!("{context}: expected array")))?;
    for row in rows {
        fields(row, keys, context)?;
    }
    Ok(())
}
fn domain_fields(a: &Value) -> Result<(), InsightError> {
    match a["artifact"].as_str().unwrap_or("") {
        "system" => {
            fields(
                &a["capture"],
                "captured_at captured_at_time_base uptime_at_capture_seconds log_window boot is_cold_boot",
                "capture",
            )?;
            fields(
                &a["capture"]["log_window"],
                "sections first_line_at last_line_at duration_seconds is_contiguous first_line_minus_boot_seconds truncated",
                "log window",
            )?;
            fields(
                &a["capture"]["boot"],
                "at derivation epoch_corroboration",
                "boot",
            )?;
            fields(
                &a["build"],
                "display_id fingerprint type is_user_build tags incremental version sdk_extensions module_metadata_version security_patch build_date_utc bootloader radio network boot_image_fingerprint",
                "build",
            )?;
            fields(
                &a["product"],
                "name device brand manufacturer model first_api_level characteristics cpu_abi cpu_abilist",
                "product",
            )?;
            fields(
                &a["device"],
                "is_emulator is_emulator_basis avd_name serial_no boot_hardware bootconfig_excerpt cmdline_has_kvm_arm_protected",
                "device",
            )?;
            fields(
                &a["kernel"],
                "version_line version_release cmdline config_archived",
                "kernel",
            )?;
            fields(&a["users"], "current_user_id list cross_check", "users")?;
            array_fields(
                &a["users"]["list"],
                "id name flags flags_hex is_primary parent_id type state serial_no created_ago start_time_ago last_logged_in_ago unlock_time_ago last_entered_foreground_ago",
                "users",
            )?;
            array_fields(
                &a["scope"]["sections"],
                "title ordinal command status start_line end_line duration_seconds",
                "sections",
            )?;
            fields(
                &a["storage"],
                "mountinfo_pids anr_files_archived",
                "storage",
            )?;
            fields(
                &a["archive_entries"],
                "proto fs_paths lshal_debug visible_windows dumpstate_board",
                "archive inventory",
            )?;
            array_fields(
                &a["hal"]["hidl_services"],
                "interface_instance transport server_pid arch threads_used threads_total released interface_hash",
                "HAL",
            )?;
        }
        "apps" => {
            fields(
                &a["inventory"],
                "counts packages processes shared_uids index",
                "inventory",
            )?;
            array_fields(
                &a["channels"],
                "name present first_at last_at duration_seconds covers_log_window coverage_ratio incomplete_reason snapshot_at carries",
                "app channels",
            )?;
            array_fields(
                &a["inventory"]["packages"],
                "package_name identity_hash app_id shared_uid_name version_name version_code min_sdk target_sdk code_path resource_path code_path_prefix tier tier_basis flags private_flags apk_signing_version apex_module_name package_source package_source_name installer_package_name initiating_package_name last_update_time time_stamp users declared_permissions install_permissions provenance",
                "packages",
            )?;
            array_fields(
                &a["inventory"]["processes"],
                "process_key start_seq record_kind record_identity_hash pid process_name runtime_key attribution_key user_id uid uid_formatted app_id gids isolated_uid packages package_dependencies start_uptime_ago start_elapsed_time_ago start_wall_clock start_wall_clock_evidence last_activity_time_ago persistent removed start_seq_conflict code_path data_dir instruction_set required_abi class_name compat oom_adj oom_adj_name cur_proc_state set_proc_state capability lru services providers receivers connections lifecycle_refs provenance",
                "processes",
            )?;
            array_fields(
                &a["lifecycle_events"],
                "kind at uid_column_raw pid tid user_id start_uid process_name hosting_type hosting_component oom_adj reason rss_kib process_key provenance",
                "lifecycle",
            )?;
        }
        "anr" => {
            array_fields(
                &a["channels"],
                "channel emitter_kind present counts_ref provenance",
                "ANR channels",
            )?;
            array_fields(
                &a["anr_incidents"],
                "incident_id process pid user_id user_id_basis occurred_at activity_component reason parent_component error_id frozen app_flags_hex processes trace_files evidence_channels provenance",
                "ANR incidents",
            )?;
            fields(
                &a["trace_files"],
                "count listing_total_kib_as_printed total_bytes oldest_created_at newest_created_at span_seconds all_outside_log_window files",
                "ANR files",
            )?;
            fields(
                &a["last_anr_trace"],
                "section_header source_path file_mtime_local trace_first_stamp trace_last_stamp process_count has_dumping_pid_marker uptime_ms_at_dump processes lock_contention_markers provenance",
                "last trace",
            )?;
        }
        "crashes" => {
            for key in ["java_crash_groups", "native_crash_groups"] {
                array_fields(
                    &a[key],
                    "group_id language signature packages package_count repeat_count raw_line_hits duplicate_hits duplicate_detection first_seen last_seen window_seconds rate_per_second inter_gap_p50_ms inter_gap_max_ms pid_span classification classification_thresholds classification_met manager_kill_count manager_kill_reason rescueparty_failure_count rescueparty_disabled corroboration provenance",
                    "crash group",
                )?;
            }
            for key in ["java_crash_incidents", "native_crash_incidents"] {
                array_fields(
                    &a[key],
                    "incident_id group_id language package process pid prefix_pid prefix_uid thread_name exception_class exception_message top_frames signal_number signal_name sigcode_name fault_addr abort_message occurred_at process_uptime_seconds flags_hex crash_handler evidence_channels provenance",
                    "crash incident",
                )?;
            }
        }
        "memory" => {
            fields(
                &a["system"],
                "total_ram_kib memory_level free_ram_kib free_ram_components_kib free_ratio used_ram_kib used_pss_kib kernel_used_kib total_pss_kib total_pss_kib_derivation lost_ram_kib tuning gpu_kib dma_buf provenance",
                "memory system",
            )?;
            fields(
                &a["swap"],
                "swap_total_kib swap_used_kib swap_used_pct zram_physical_kib zram_physical_pct_of_ram compression_ratio compression_ratio_derived zram_block_io provenance",
                "swap",
            )?;
            array_fields(
                &a["processes"],
                "pid process_name user_id pss_total_kib pss_private_dirty_kib pss_shared_clean_kib rss_total_kib swap_pss_kib swap_uss_kib zswap_kib oom_adj heap_size_kib heap_alloc_kib heap_free_kib source_channel provenance",
                "memory processes",
            )?;
            array_fields(
                &a["categories"],
                "category pss_total_kib swap_pss_kib depth parent_category provenance",
                "memory categories",
            )?;
            array_fields(
                &a["procstats"],
                "sub_section scope_label_verbatim period_start_clock period_total_uptime period_total_elapsed period_complete period_flags aggregated_over_n backing_state_file sys_mem_usage checkin_period provenance",
                "procstats",
            )?;
            fields(
                &a["pressure_indicators"],
                "psi_memory psi_io vmstat_oom_kill vmstat_pswpin vmstat_pswpout",
                "pressure",
            )?;
            for key in ["slab", "pagetables", "stacks", "cma"] {
                require(
                    a["kernel_memory"][key].is_object() || a["kernel_memory"][key].is_null(),
                    "kernel memory must be a measured object or null",
                )?;
            }
        }
        "cpu" => {
            array_fields(
                &a["utilization"],
                "channel line_start window window_source age_at_print_ms total_pct pct_user pct_kernel pct_iowait pct_irq pct_softirq pct_nice pct_steal null_reasons online_cpus configured_cpus threads_total running sleeping stopped zombie provenance",
                "CPU utilization",
            )?;
            array_fields(
                &a["per_cpu_core"],
                "core_id is_online user_time_ms nice_time_ms system_time_ms idle_time_ms iowait_time_ms irq_time_ms softirq_time_ms steal_time_ms guest_time_ms guest_nice_time_ms busy_pct_since_boot busy_pct_derivation read_at_uptime_ms provenance",
                "CPU core",
            )?;
            fields(
                &a["contention"],
                "art_java_monitor art_runtime_mutex art_rate_limiting dvm_lock_sample provenance",
                "contention",
            )?;
            for sample in a["utilization"].as_array().into_iter().flatten() {
                fields(
                    &sample["window"],
                    "start_utc end_utc duration_ms",
                    "CPU window",
                )?;
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
                    if sample[key].is_null() {
                        require(
                            sample["null_reasons"][key].is_string(),
                            "null CPU ratio needs reason",
                        )?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
