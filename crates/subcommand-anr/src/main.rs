use bugreport_insight_anr::scan_anrs;
use bugreport_insight_core::open_report;
use bugreport_insight_subcommand_support::run_subcommand;
use std::process::ExitCode;

fn main() -> ExitCode {
    run_subcommand(
        "Show ANR (Application Not Responding) findings extracted from the bugreport",
        |cli| {
            let (_archive, source, report) = open_report(&cli.input)?;
            Ok(scan_anrs(&source, &report))
        },
    )
}
