mod cli;
mod plugin;
mod report;

use clap::{CommandFactory, Parser};
use cli::Cli;
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();

    if let Some(first) = args.first() {
        if first == "-h" || first == "--help" {
            print_main_help();
            return ExitCode::SUCCESS;
        }
        if first == "help" {
            return match args.get(1) {
                None => {
                    print_main_help();
                    ExitCode::SUCCESS
                }
                Some(verb) => dispatch_help(&verb.to_string_lossy()),
            };
        }
    }

    // git/cargo-style plugin dispatch: `bugreport-insight <verb> ...` runs the sibling
    // `bugreport-insight-<verb>` binary instead of being handled inline. Falls through to the
    // default full-report behavior below when `verb` isn't a known or discoverable subcommand.
    if let Some(verb) = args.first().map(|a| a.to_string_lossy())
        && !verb.starts_with('-')
        && let Some(exit_code) = plugin::dispatch(&verb, &args[1..])
    {
        return exit_code;
    }

    let cli = Cli::parse();
    match report::run(&cli.input, cli.output, cli.force) {
        Ok(output) => {
            println!("Generated insight artifacts in {}", output.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_main_help() {
    Cli::command().print_help().expect("stdout is writable");
    print_subcommand_help();
}

/// Handles `bugreport-insight help <verb>` by execing `bugreport-insight-<verb> --help`, so a
/// subcommand's own (more specific) help text is shown instead of the main program's.
fn dispatch_help(verb: &str) -> ExitCode {
    match plugin::dispatch(verb, &[std::ffi::OsString::from("--help")]) {
        Some(exit_code) => exit_code,
        None => {
            eprintln!(
                "error: no such subcommand: `{verb}` (run `bugreport-insight help` to list built-ins)"
            );
            ExitCode::FAILURE
        }
    }
}

fn print_subcommand_help() {
    println!("\nBuilt-in subcommands (each dispatches to a `bugreport-insight-<verb>` binary):");
    for verb in plugin::BUILTIN_SUBCOMMANDS {
        println!("  {verb}");
    }
    println!(
        "\nThird-party subcommands are supported the same way: install a `bugreport-insight-<verb>`\nexecutable on PATH and `bugreport-insight <verb> ...` will run it."
    );
}
