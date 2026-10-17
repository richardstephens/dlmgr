pub mod args;
pub mod tasks;

use crate::args::AppArgs;
use clap::Parser;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr.with_max_level(args.global.log_level()));

    tracing_subscriber::registry().with(layer).init();

    tasks::download::download_main(&args.global, &args.dl_args).await
}
