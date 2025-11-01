use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: ServerType,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ServerType {
    Simple,
    ZeroServer(ZeroServerArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct ZeroServerArgs {
    pub zero_len: u64,
}
