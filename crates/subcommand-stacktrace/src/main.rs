use bugreport_insight_core::open_report;
use bugreport_insight_stacktrace::{scan_java_crashes, scan_native_crashes};
use bugreport_insight_subcommand_support::run_subcommand;
use std::process::ExitCode;

fn main() -> ExitCode {
    run_subcommand(
        "Show native crash, stacktrace, and tombstone findings extracted from the bugreport",
        |cli| {
            let (mut archive, source, report) = open_report(&cli.input)?;
            let mut findings = scan_java_crashes(&source, &report);
            findings.extend(scan_native_crashes(&mut archive)?);
            Ok(findings)
        },
    )
}
