//! App inventory scanning, owned by the `bugreport-insight apps` subcommand: PID, package name,
//! data dir, and installed path. Exposed as a library so both the `bugreport-insight-apps`
//! binary and other consumers can call [`scan_apps`] directly.

use bugreport_insight_core::model::{AppInfo, Evidence};
use bugreport_insight_core::text::bounded;
use std::collections::BTreeMap;

/// A parsed `dumpsys package` `Package [name] (hash):` block: the installed path (`codePath`)
/// and the first per-user data directory (`dataDir`) found before the next `Package [` block.
struct PackageDetails {
    line: usize,
    code_path: Option<String>,
    data_dir: Option<String>,
}

/// A parsed `dumpsys activity` `PID mappings:` entry:
/// `PID #<pid>: ProcessRecord{<hash> <pid>:<process>/<uid>}`. The package is the process name up
/// to its first `:` (isolated/secondary processes suffix their base package, e.g.
/// `com.example:remote`).
struct PidMapping<'a> {
    line: usize,
    pid: u32,
    process: &'a str,
    uid: &'a str,
}

fn parse_package_blocks(lines: &[&str]) -> BTreeMap<String, PackageDetails> {
    let mut packages = BTreeMap::new();
    let mut current: Option<(String, PackageDetails)> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(name) = line
            .strip_prefix("  Package [")
            .and_then(|rest| rest.split_once(']'))
            .map(|(name, _)| name)
        {
            if let Some((name, details)) = current.take() {
                packages.insert(name, details);
            }
            current = Some((
                name.to_string(),
                PackageDetails {
                    line: index + 1,
                    code_path: None,
                    data_dir: None,
                },
            ));
            continue;
        }
        let Some((_, details)) = current.as_mut() else {
            continue;
        };
        let trimmed = line.trim_start();
        if details.code_path.is_none()
            && let Some(path) = trimmed.strip_prefix("codePath=")
        {
            details.code_path = Some(path.to_string());
        }
        // Each package lists one `User N: ...` block per device user profile, each with its own
        // `dataDir=` line below it — `dataDir=null` when the app isn't installed for that
        // profile. Skip `null` and keep the first real path across profiles.
        if let Some(path) = trimmed.strip_prefix("dataDir=")
            && path != "null"
            && details.data_dir.is_none()
        {
            details.data_dir = Some(path.to_string());
        }
    }
    if let Some((name, details)) = current.take() {
        packages.insert(name, details);
    }
    packages
}

fn parse_pid_mappings<'a>(lines: &'a [&'a str]) -> Vec<PidMapping<'a>> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = line.trim_start().strip_prefix("PID #")?;
            let (pid, rest) = rest.split_once(':')?;
            let pid = pid.trim().parse::<u32>().ok()?;
            let rest = rest.trim().strip_prefix("ProcessRecord{")?;
            let (_hash, rest) = rest.split_once(' ')?;
            let (_pid_again, rest) = rest.split_once(':')?;
            let (process, uid) = rest.trim_end_matches('}').split_once('/')?;
            Some(PidMapping {
                line: index + 1,
                pid,
                process,
                uid,
            })
        })
        .collect()
}

// https://source.android.com/docs/core/tests/debug/read-bug-reports: `dumpsys package` names
// each installed package's on-disk installed path (`codePath`) and per-user data directory
// (`dataDir`); `dumpsys activity`'s `PID mappings:` section names each currently-running
// process's live PID and UID. Correlating the two gives a per-app inventory of where an app is
// installed, where its data lives, and whether (and as which PID) it is currently running.
pub fn scan_apps(source: &str, report: &str) -> Vec<AppInfo> {
    let lines: Vec<&str> = report.lines().collect();
    let packages = parse_package_blocks(&lines);
    let pid_mappings = parse_pid_mappings(&lines);

    let mut apps = Vec::new();
    let mut matched_packages = std::collections::BTreeSet::new();
    for mapping in &pid_mappings {
        let package = mapping
            .process
            .split_once(':')
            .map_or(mapping.process, |(base, _)| base);
        let details = packages.get(package);
        matched_packages.insert(package.to_string());
        let mut evidence = vec![Evidence {
            source: source.into(),
            line_start: mapping.line,
            line_end: mapping.line,
            excerpt: bounded(lines[mapping.line - 1]),
        }];
        if let Some(details) = details {
            evidence.push(Evidence {
                source: source.into(),
                line_start: details.line,
                line_end: details.line,
                excerpt: bounded(lines[details.line - 1]),
            });
        }
        apps.push(AppInfo {
            schema_version: 1,
            package: package.to_string(),
            pid: Some(mapping.pid),
            uid: Some(mapping.uid.to_string()),
            data_dir: details.and_then(|d| d.data_dir.clone()),
            code_path: details.and_then(|d| d.code_path.clone()),
            evidence,
        });
    }
    for (package, details) in &packages {
        if matched_packages.contains(package) {
            continue;
        }
        apps.push(AppInfo {
            schema_version: 1,
            package: package.clone(),
            pid: None,
            uid: None,
            data_dir: details.data_dir.clone(),
            code_path: details.code_path.clone(),
            evidence: vec![Evidence {
                source: source.into(),
                line_start: details.line,
                line_end: details.line,
                excerpt: bounded(lines[details.line - 1]),
            }],
        });
    }
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlates_running_pid_with_package_details() {
        let report = "  Package [com.example.app] (abc123):\n\
                       \x20\x20\x20\x20appId=10250\n\
                       \x20\x20\x20\x20codePath=/data/app/com.example.app-1/base.apk\n\
                       \x20\x20\x20\x20User 10: installed=true\n\
                       \x20\x20\x20\x20\x20\x20dataDir=/data/user/10/com.example.app\n\
                       \x20\x20PID mappings:\n\
                       \x20\x20\x20\x20PID #2818: ProcessRecord{a4eb6e5 2818:com.example.app/u10a250}\n";
        let apps = scan_apps("report.txt", report);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].package, "com.example.app");
        assert_eq!(apps[0].pid, Some(2818));
        assert_eq!(apps[0].uid.as_deref(), Some("u10a250"));
        assert_eq!(
            apps[0].data_dir.as_deref(),
            Some("/data/user/10/com.example.app")
        );
        assert_eq!(
            apps[0].code_path.as_deref(),
            Some("/data/app/com.example.app-1/base.apk")
        );
    }

    #[test]
    fn skips_null_data_dir_from_an_uninstalled_user_profile() {
        let report = "  Package [com.example.app] (abc123):\n\
                       \x20\x20\x20\x20codePath=/data/app/com.example.app-1/base.apk\n\
                       \x20\x20\x20\x20User 0: installed=false\n\
                       \x20\x20\x20\x20\x20\x20dataDir=null\n\
                       \x20\x20\x20\x20User 10: installed=true\n\
                       \x20\x20\x20\x20\x20\x20dataDir=/data/user/10/com.example.app\n";
        let apps = scan_apps("report.txt", report);
        assert_eq!(apps.len(), 1);
        assert_eq!(
            apps[0].data_dir.as_deref(),
            Some("/data/user/10/com.example.app")
        );
    }

    #[test]
    fn lists_installed_but_not_running_packages() {
        let report = "  Package [com.example.dormant] (def456):\n\
                       \x20\x20\x20\x20codePath=/system/app/Dormant/Dormant.apk\n";
        let apps = scan_apps("report.txt", report);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].package, "com.example.dormant");
        assert_eq!(apps[0].pid, None);
    }

    #[test]
    fn splits_isolated_process_suffix_to_base_package() {
        let report = "  Package [com.example.app] (abc123):\n\
                       \x20\x20\x20\x20codePath=/data/app/com.example.app-1/base.apk\n\
                       \x20\x20PID mappings:\n\
                       \x20\x20\x20\x20PID #2283: ProcessRecord{b873197 2283:com.example.app:remote/u10a208}\n";
        let apps = scan_apps("report.txt", report);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].package, "com.example.app");
        assert_eq!(apps[0].pid, Some(2283));
    }
}
