//! `ARCH-CLI-FLAGS`: the `bugreport-insight` binary.
//!
//! Three committed flags — `--mcp`, `--help`, `--verbose` (`REQ-CLI-03`,
//! `REQ-CLI-04`, `REQ-CLI-05`). It owns argv parsing and mode selection, hands
//! control to `crates/mcp` on `--mcp`, and is a peer of that crate rather than a
//! parent: it never names an `rmcp` symbol (`architecture.md` §4).
//!
//! stdout is the MCP transport under `--mcp`, so every diagnostic here goes to
//! stderr. Exit codes: 0 success, 2 bad usage, 1 archive or transport failure.

use std::path::PathBuf;
use std::process::ExitCode;

use bugreport_insight_mcp::diag;
use clap::Parser;

/// Read-only, bounded access to one Android bugreport ZIP, as an MCP server
/// over stdio.
#[derive(Debug, Parser)]
#[command(
    name = "bugreport-insight",
    version,
    about = "Read-only MCP server over an Android bugreport ZIP archive",
    long_about = "Opens one bugreport ZIP in place and serves it to an AI agent over the Model \
                  Context Protocol on stdio. Nothing is extracted, written, or modified. Seven \
                  tools: archive_info, list_entries, find_entries, entry_info, find_by_type, \
                  read_entry, grep_archive."
)]
struct Cli {
    /// The bugreport ZIP to serve. Required with --mcp.
    #[arg(value_name = "BUGREPORT")]
    archive: Option<PathBuf>,

    /// Serve the archive as an MCP server over stdio.
    #[arg(long)]
    mcp: bool,

    /// Log verbosely to stderr. stdout carries JSON-RPC only, never logs.
    #[arg(long, short)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        diag::set_verbose(true);
    }

    if !cli.mcp {
        // `--help` and `--version` are handled by clap during `parse`. Reaching
        // here without `--mcp` means no mode was selected.
        eprintln!("bugreport-insight: error: no mode selected; pass --mcp");
        eprintln!("Try 'bugreport-insight --help' for usage.");
        return ExitCode::from(2);
    }

    let Some(archive) = cli.archive else {
        eprintln!("bugreport-insight: error: --mcp requires a BUGREPORT archive path");
        eprintln!("Try 'bugreport-insight --help' for usage.");
        return ExitCode::from(2);
    };

    if !archive.is_file() {
        eprintln!(
            "bugreport-insight: error: `{}` is not a file",
            archive.display()
        );
        return ExitCode::from(1);
    }

    diag::debug(&format!("archive path: {}", archive.display()));

    // The archive is opened before the stdio loop starts, so a failure here is a
    // non-zero exit with a stderr diagnostic and never an MCP response
    // (`design/05-mcp-server-design.md` §2).
    match bugreport_insight_mcp::serve(&archive).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bugreport-insight: error: {e}");
            ExitCode::from(1)
        }
    }
}
