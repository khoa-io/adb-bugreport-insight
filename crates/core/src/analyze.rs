use crate::{
    archive::{open_archive, validate_archive},
    error::Error,
    model::Finding,
    render::render,
    scanners::{scan_activity_timeline, scan_broadcast_latency, scan_watchdog},
    sessions::sessions,
    source::read_primary_report,
};
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

/// Opens `input`, validates the archive, and reads its primary bugreport text. Shared by the
/// default full-report `run()` and by focused subcommands (`anr`, `apps`, `stacktrace`,
/// `memory`, `cpu`, ...) that only need one or two scanners rather than the whole suite.
/// The returned archive stays open so callers needing raw entries (e.g. tombstones for
/// `scan_native_crashes`) can keep reading from it.
pub fn open_report(input: &Path) -> Result<(ZipArchive<BufReader<File>>, String, String), Error> {
    let mut archive = open_archive(input)?;
    validate_archive(&mut archive)?;
    let (source, report) = read_primary_report(&mut archive)?;
    Ok((archive, source, report))
}

/// Findings from this crate's own scanners (`watchdog`, `broadcast_latency`,
/// `activity_timeline` — the ones with no owning built-in subcommand). The `bugreport-insight`
/// CLI merges these with findings from the `anr`, `stacktrace`, `memory`, and `cpu` subcommand
/// libraries to assemble the full-report finding set passed to [`render_insight`].
pub fn unclaimed_findings(source: &str, report: &str) -> Vec<Finding> {
    let mut findings = scan_watchdog(source, report);
    findings.extend(scan_broadcast_latency(source, report));
    findings.extend(scan_activity_timeline(source, report));
    findings
}

/// Writes the `insight/` artifact set for an already-opened report under `output` (or a
/// directory derived from `input`'s name when `output` is `None`), given the caller's complete
/// merged `findings`. Returns the path to the generated `insight/` directory.
///
/// Splitting this from report-opening and scanning (unlike the old all-in-one `run`) lets the
/// caller (the `bugreport-insight` CLI) assemble `findings` from scanners that live in separate
/// `bugreport-insight-<verb>` crates, without this crate depending on them.
pub fn render_insight(
    input: &Path,
    output: Option<PathBuf>,
    force: bool,
    source: &str,
    report: &str,
    findings: &[Finding],
) -> Result<PathBuf, Error> {
    let output = output.unwrap_or_else(|| default_output(input));
    prepare_output(&output, force)?;
    let sessions = sessions(source, report);
    render(&output, input, source, findings, &sessions)?;
    Ok(output.join("insight"))
}

fn default_output(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    parent.join(format!(".{stem}"))
}

fn prepare_output(output: &Path, force: bool) -> Result<(), Error> {
    if output.exists() {
        let non_empty = fs::read_dir(output)?.next().is_some();
        if non_empty && !force {
            return Err(Error::OutputExists(output.display().to_string()));
        }
        if force {
            fs::remove_dir_all(output)?;
        }
    }
    fs::create_dir_all(output.join("insight"))?;
    Ok(())
}
