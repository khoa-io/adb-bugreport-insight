//! `ARCH-CLI-FLAGS`: the `bugreport-insight` binary.
//!
//! CLI flags and mode composition (`REQ-CLI-03`–`05`, `REQ-CLI-12`,
//! `REQ-INSIGHT-01`). It owns argv parsing and mode selection, hands
//! control to its `crates/mcp` dependency on `--mcp`; it never names an `rmcp` symbol (`docs/swe.2-architecture/02-components-and-dependencies.md`).
//!
//! stdout is the MCP transport under `--mcp`, so every diagnostic here goes to
//! stderr. Exit codes: 0 success, 2 bad usage, 1 archive or transport failure.

use std::path::PathBuf;
use std::process::ExitCode;

use bugreport_insight_core::ArchiveLimits;
use bugreport_insight_mcp::diag;
use clap::Parser;

/// Read or analyze one Android bugreport ZIP.
#[derive(Debug, Parser)]
#[command(
    name = "bugreport-insight",
    version,
    about = "Read or analyze one Android bugreport ZIP archive",
    long_about = "Use --mcp with a positional BUGREPORT to serve bounded, read-only archive tools \
                  over MCP stdio. Add --insight BUGREPORT_OR_DIRECTORY for insight tools, \
                  or use --insight BUGREPORT alone to write static insight artifacts."
)]
struct Cli {
    /// The bugreport ZIP to serve with --mcp (without --insight).
    #[arg(value_name = "BUGREPORT")]
    archive: Option<PathBuf>,

    /// Serve the archive as an MCP server over stdio.
    #[arg(long)]
    mcp: bool,

    /// Log verbosely to stderr. stdout carries JSON-RPC only, never logs.
    #[arg(long, short)]
    verbose: bool,

    /// Generate artifacts, or enable insight tools with --mcp for a ZIP or directory.
    #[arg(
        long,
        value_name = "BUGREPORT_OR_DIRECTORY",
        conflicts_with = "archive"
    )]
    insight: Option<PathBuf>,

    /// Raise archive and analysis limits for a large bugreport.
    #[arg(long)]
    force: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        diag::set_verbose(true);
    }

    let limits = if cli.force {
        eprintln!(
            "bugreport-insight: warning: --force raises archive and analysis limits; large bugreports may require substantial memory and processing time"
        );
        ArchiveLimits::FORCED
    } else {
        ArchiveLimits::DEFAULT
    };

    if cli.mcp {
        let insight = cli.insight.is_some();
        let Some(path) = cli.insight.or(cli.archive) else {
            eprintln!(
                "bugreport-insight: error: --mcp requires a BUGREPORT or --insight BUGREPORT_OR_DIRECTORY"
            );
            return ExitCode::from(2);
        };
        diag::debug(&format!(
            "MCP binding: {} (insight: {insight})",
            path.display()
        ));
        return mcp_exit_code_with_force(
            bugreport_insight_mcp::serve_with_options(&path, insight, limits).await,
            cli.force,
        );
    }

    if let Some(archive) = cli.insight {
        if !archive.is_file() {
            eprintln!(
                "bugreport-insight: error: `{}` is not a file",
                archive.display()
            );
            return ExitCode::from(1);
        }
        return match run_insight(&archive, limits) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                report_error(&error, cli.force);
                ExitCode::from(1)
            }
        };
    }

    eprintln!("bugreport-insight: error: no mode selected; pass --mcp or --insight BUGREPORT");
    eprintln!("Try 'bugreport-insight --help' for usage.");
    ExitCode::from(2)
}

#[cfg(test)]
fn mcp_exit_code(result: Result<(), bugreport_insight_mcp::ServeError>) -> ExitCode {
    mcp_exit_code_with_force(result, false)
}

fn mcp_exit_code_with_force(
    result: Result<(), bugreport_insight_mcp::ServeError>,
    force: bool,
) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&e, force);
            ExitCode::from(1)
        }
    }
}

fn run_insight(
    archive_path: &std::path::Path,
    limits: ArchiveLimits,
) -> Result<(), bugreport_insight_core::insight::InsightError> {
    diag::debug(&format!("analyzing {}", archive_path.display()));
    let output = bugreport_insight_core::insight::generate_to_with_limits(archive_path, limits)?;
    eprintln!(
        "bugreport-insight: wrote 7 artifacts to {}",
        output.display()
    );
    Ok(())
}

fn report_error(error: &(dyn std::error::Error + 'static), force: bool) {
    eprintln!("bugreport-insight: error: {error}");
    let mut cause = Some(error);
    while let Some(current) = cause {
        let archive_limit = current
            .downcast_ref::<bugreport_insight_core::ArchiveError>()
            .is_some_and(|error| {
                matches!(
                    error,
                    bugreport_insight_core::ArchiveError::LimitExceeded(_)
                )
            });
        let insight_limit = current
            .downcast_ref::<bugreport_insight_core::insight::InsightError>()
            .is_some_and(|error| {
                matches!(
                    error,
                    bugreport_insight_core::insight::InsightError::LimitExceeded { .. }
                )
            });
        if archive_limit || insight_limit || current.is::<bugreport_insight_core::LimitViolation>()
        {
            if force {
                eprintln!("bugreport-insight: this bugreport is too large even for --force limits");
            } else {
                eprintln!(
                    "bugreport-insight: this bugreport is too large for default limits; rerun with --force to proceed with raised limits"
                );
            }
            break;
        }
        cause = current.source();
    }
}

#[cfg(test)]
mod tests {
    use super::mcp_exit_code;
    use bugreport_insight_mcp::ServeError;
    use std::process::ExitCode;

    #[test]
    fn server_task_failure_exits_nonzero() {
        let result = Err(ServeError::ServerTask("injected test failure".to_owned()));
        assert_eq!(mcp_exit_code(result), ExitCode::from(1));
    }
}
