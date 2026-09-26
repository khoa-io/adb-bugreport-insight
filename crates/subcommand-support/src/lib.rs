//! Shared CLI plumbing for `bugreport-insight-*` subcommand binaries (git/cargo-style
//! plugins dispatched by `bugreport-insight <verb>`). Each subcommand binary's `main()` is a
//! single call to [`run_subcommand`]: it opens the report, runs the subcommand's specific
//! scanner(s), and emits the result as JSON.
//!
//! Third-party subcommands (for example a separately maintained `dumpstate` subcommand) can
//! depend on this crate the same way the built-in ones do, or skip it and depend on
//! `bugreport-insight-core` directly.

use clap::{CommandFactory, FromArgMatches, Parser};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// Common arguments for a focused subcommand: the bugreport ZIP to read, and where to write its
/// JSON result (defaults to stdout).
#[derive(Parser)]
pub struct SubcommandCli {
    #[arg(value_name = "BUGREPORT_ZIP")]
    pub input: PathBuf,
    /// Write the JSON result to this file instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

impl SubcommandCli {
    /// Parses `std::env::args_os()` like [`clap::Parser::parse`], but overrides the generic
    /// `about` text with a description specific to the calling subcommand binary, so
    /// `bugreport-insight-<verb> --help` (and `bugreport-insight help <verb>`, which just execs
    /// the binary with `--help`) explains what that particular subcommand does.
    pub fn parse_for(about: &str) -> Self {
        let matches = Self::command().about(about.to_string()).get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
    }
}

/// Serializes `value` as pretty JSON to `output` (or stdout when `None`).
pub fn emit(value: &(impl Serialize + ?Sized), output: Option<&Path>) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value).expect("finding models always serialize");
    match output {
        Some(path) => fs::write(path, json + "\n"),
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(json.as_bytes())?;
            stdout.write_all(b"\n")
        }
    }
}

/// Prints `error` to stderr in the same `error: {message}` shape as the main binary and returns
/// a failure exit code, for a subcommand's `main()` to return directly.
pub fn report_error(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}

/// Runs the full `main()` body shared by every built-in subcommand binary: parses
/// [`SubcommandCli`] with `about` as its description, calls `action` to open the report and run
/// the subcommand's scanner(s), then emits the result as JSON (or reports the error). This is
/// the single place that boilerplate lives, so each subcommand's `main.rs` only needs to name
/// its description and its scanning step.
pub fn run_subcommand<T: Serialize>(
    about: &str,
    action: impl FnOnce(&SubcommandCli) -> Result<T, Box<dyn std::error::Error>>,
) -> ExitCode {
    let cli = SubcommandCli::parse_for(about);
    let result = action(&cli).and_then(|value| Ok(emit(&value, cli.output.as_deref())?));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report_error(error),
    }
}
