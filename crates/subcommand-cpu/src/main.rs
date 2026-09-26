use bugreport_insight_core::open_report;
use bugreport_insight_cpu::{scan_cpu_pressure, scan_monitor_contention};
use bugreport_insight_subcommand_support::run_subcommand;
use std::process::ExitCode;

fn main() -> ExitCode {
    run_subcommand(
        "Show CPU/IO pressure and lock-contention findings extracted from the bugreport",
        |cli| {
            let (_archive, source, report) = open_report(&cli.input)?;
            let mut findings = scan_cpu_pressure(&source, &report);
            findings.extend(scan_monitor_contention(&source, &report));
            Ok(findings)
        },
    )
}
