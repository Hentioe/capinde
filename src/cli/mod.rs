pub mod cleanup;
pub mod healthcheck;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long,action = clap::ArgAction::SetTrue)]
    pub healthcheck: bool,
    #[arg(long,action = clap::ArgAction::SetTrue)]
    pub cleanup: bool,
    #[arg(long)]
    pub check_bind: Option<String>,
}
