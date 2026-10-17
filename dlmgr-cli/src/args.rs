use crate::tasks::download::DownloadArgs;
use clap::{Args, Parser};
use tracing::Level;

#[derive(Parser, Debug)]
pub struct AppArgs {
    #[clap(flatten)]
    pub global: GlobalArgs,

    #[clap(flatten)]
    pub dl_args: DownloadArgs,
}

#[derive(Args, Debug)]
pub struct GlobalArgs {
    #[clap(short = 'v', long, default_value_t = false)]
    pub verbose: bool,
}

impl GlobalArgs {
    pub fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
}
