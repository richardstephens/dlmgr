use clap::Parser;
use dlmgr::consumers::in_memory_hashing::HashingChunkConsumer;
use indicatif::ProgressBar;

use dlmgr::DownloadTaskBuilder;
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(short = 'v', long, default_value_t = false)]
    pub verbose: bool,
    #[clap(long)]
    pub url: Vec<String>,
}
impl Args {
    fn log_level(&self) -> Level {
        if self.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        }
    }
}
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr.with_max_level(args.log_level()));

    tracing_subscriber::registry().with(layer).init();

    let task_builder = DownloadTaskBuilder::new();
    let urls = args
        .url
        .iter()
        .map(|u| Url::parse(u))
        .collect::<Result<Vec<_>, _>>()?;
    let download = task_builder
        .begin_download(urls.into_iter().collect(), HashingChunkConsumer::new())
        .await?;

    if !args.verbose {
        let progress = download.progress_provider();
        let bar = ProgressBar::new(progress.content_length());
        loop {
            let bytes_downloaded = progress.bytes_downloaded();

            bar.set_position(bytes_downloaded);
            if bytes_downloaded >= progress.content_length() {
                break;
            }
        }
        bar.finish();
    }

    download.await_completion().await?;

    Ok(())
}
