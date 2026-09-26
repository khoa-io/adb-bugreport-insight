use bugreport_insight_apps::scan_apps;
use bugreport_insight_core::open_report;
use bugreport_insight_subcommand_support::run_subcommand;
use std::process::ExitCode;

fn main() -> ExitCode {
    run_subcommand(
        "Show installed and running app info: PID, package name, data dir, install path",
        |cli| {
            let (_archive, source, report) = open_report(&cli.input)?;
            Ok(scan_apps(&source, &report))
        },
    )
}
