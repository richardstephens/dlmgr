use crate::args::GlobalArgs;
use anyhow::{anyhow, bail};
use clap::Args;
use dlmgr::api::sequential_chunk_consumer::SequentialChunkConsumer;
use dlmgr::consumers::atomic_file_consumer_sha256::AtomicFileConsumerSha256;
use dlmgr::consumers::in_memory_hashing::HashingChunkConsumer;
use dlmgr::{DownloadTask, DownloadTaskBuilder, ProgressProvider};
use indicatif::ProgressBar;
use std::path::PathBuf;
use tracing::info;
use url::Url;

#[derive(Args, Debug)]
pub struct DownloadArgs {
    #[clap(long)]
    pub url: Vec<String>,

    #[clap(long)]
    pub output: Option<PathBuf>,
    #[clap(long)]
    pub expected_sha256: Option<String>,
}

pub async fn download_main(global_args: &GlobalArgs, args: &DownloadArgs) -> anyhow::Result<()> {
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

    let download: DownloadTask = task_builder
        .begin_download(urls.into_iter().collect(), consumer)
        .await?;

    let progress: ProgressProvider = download.progress_provider();

    if !global_args.verbose {
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
