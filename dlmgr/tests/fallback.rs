use dlmgr::consumers::in_memory_hashing::HashingChunkConsumer;
use dlmgr::{ConcurrencyBehaviour, DownloadTaskBuilder};

// When the server does not advertise range support, `Prefer` (the default)
// should fall back to the sequential downloader and still produce correct bytes.
#[tokio::test]
async fn sequential_fallback_when_range_unsupported() {
    let server = range_server::no_range::start_no_range_server(1024 * 1024).await;

    let (download_target, result_rx) = HashingChunkConsumer::new_with_hash_provider();

    let download = DownloadTaskBuilder::new()
        .begin_download(server.url.clone().try_into().unwrap(), download_target)
        .await
        .unwrap();

    // Content-Length is reported by the server, so progress tracking should work.
    assert_eq!(download.content_length(), 1024 * 1024);

    download.await_completion().await.unwrap();

    let downloaded_hash = result_rx.await.unwrap().unwrap();

    assert_eq!(server.sha256, downloaded_hash);
}

#[tokio::test]
async fn sequential_fallback_when_concurrency_disabled() {
    let server = range_server::simple::start_temp_server().await;

    let (download_target, result_rx) = HashingChunkConsumer::new_with_hash_provider();

    let download = DownloadTaskBuilder::new()
        .with_concurrency_behaviour(ConcurrencyBehaviour::Disabled)
        .begin_download(
            server.tmpfile_url.clone().try_into().unwrap(),
            download_target,
        )
        .await
        .unwrap();

    download.await_completion().await.unwrap();

    let downloaded_hash = result_rx.await.unwrap().unwrap();

    //TODO: this should assert that we didn't get a concurrent download

    assert_eq!(server.tmpfile_sha256, downloaded_hash);
}

#[tokio::test]
async fn concurrency_require() {
    let server = range_server::no_range::start_no_range_server(100 * 1024 * 1024).await;

    let (download_target, _result_rx) = HashingChunkConsumer::new_with_hash_provider();

    DownloadTaskBuilder::new()
        .with_concurrency_behaviour(ConcurrencyBehaviour::Require)
        .begin_download(server.url.clone().try_into().unwrap(), download_target)
        .await
        .unwrap_err();
}
