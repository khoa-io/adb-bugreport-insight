use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "bugreport-insight",
    about = "Read-only Android bugreport insight generator"
)]
pub struct Cli {
    #[arg(value_name = "BUGREPORT_ZIP")]
    pub input: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
}
