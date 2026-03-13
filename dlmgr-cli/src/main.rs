use anyhow::{anyhow, bail};
use clap::Parser;
use dlmgr::consumers::in_memory_hashing::HashingChunkConsumer;
use indicatif::ProgressBar;
use std::path::PathBuf;

use dlmgr::{DownloadTask, DownloadTaskBuilder, ProgressProvider};
use dlmgr::api::sequential_chunk_consumer::SequentialChunkConsumer;
use dlmgr::consumers::atomic_file_consumer_sha256::AtomicFileConsumerSha256;
use tracing::{Level, info};
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

    #[clap(long)]
    pub output: Option<PathBuf>,
    #[clap(long)]
    pub expected_sha256: Option<String>,
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

    let (consumer, completion): (Box<dyn SequentialChunkConsumer>, Option<_>) =
        if let (Some(output), Some(expected_sha256)) =
            (args.output.as_deref(), args.expected_sha256.as_deref())
        {
            let expected_sha256: [u8; 32] = hex::decode(expected_sha256)?
                .try_into()
                .map_err(|_e| anyhow!("invalid hex for sha256 hash"))?;
            let (consumer, completion) =
                AtomicFileConsumerSha256::new(output.to_path_buf(), expected_sha256).await?;
            (Box::new(consumer), Some(completion))
        } else if args.output.is_none() && args.expected_sha256.is_none() {
            (Box::new(HashingChunkConsumer::new()), None)
        } else {
            bail!("To save the downloaded file, both `output` and `expected_sha256` are required.");
        };

    let download : DownloadTask= task_builder
        .begin_download(urls.into_iter().collect(), consumer)
        .await?;

    let progress: ProgressProvider = download.progress_provider();

    if !args.verbose {
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

    info!("Stats: {:#?}", progress);

    download.await_completion().await?;

    if let Some(completion) = completion {
        completion.await??;
    };

    Ok(())
}
