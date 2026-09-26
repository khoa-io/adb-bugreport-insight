//! Assembles the default full-report finding set. Each built-in subcommand's scanner(s) are
//! library functions in their own `bugreport-insight-<verb>` crate (`anr`, `stacktrace`,
//! `memory`, `cpu`; `apps` is not part of the full report — it is `apps`-subcommand-only), so
//! this is the one place that composes them together with `bugreport-insight-core`'s own
//! "unclaimed" scanners (`watchdog`, `broadcast_latency`, `activity_timeline`) into a single
//! `findings.json`/`README.md` artifact set.

use bugreport_insight_core::{Error, open_report, render_insight, unclaimed_findings};
use std::path::{Path, PathBuf};

/// Reads `input` (a bugreport ZIP), runs every built-in scanner, and writes the `insight/`
/// artifact set under `output` (or a directory derived from `input`'s name when `output` is
/// `None`). Returns the path to the generated `insight/` directory.
pub fn run(input: &Path, output: Option<PathBuf>, force: bool) -> Result<PathBuf, Error> {
    let (mut archive, source, report) = open_report(input)?;
    let mut findings = bugreport_insight_anr::scan_anrs(&source, &report);
    findings.extend(bugreport_insight_stacktrace::scan_java_crashes(
        &source, &report,
    ));
    findings.extend(bugreport_insight_stacktrace::scan_native_crashes(
        &mut archive,
    )?);
    findings.extend(bugreport_insight_memory::scan_memory_pressure(
        &source, &report,
    ));
    findings.extend(bugreport_insight_cpu::scan_cpu_pressure(&source, &report));
    findings.extend(bugreport_insight_cpu::scan_monitor_contention(
        &source, &report,
    ));
    findings.extend(unclaimed_findings(&source, &report));
    render_insight(input, output, force, &source, &report, &findings)
}
