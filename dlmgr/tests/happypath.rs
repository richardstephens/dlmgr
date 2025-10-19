use dlmgr::DownloadTaskBuilder;
use dlmgr::consumers::in_memory_hashing::HashingChunkConsumer;

#[tokio::test]
async fn happy_path() {
    let tmp_server = range_server::simple::start_temp_server().await;

    let task_builder = DownloadTaskBuilder::new();

    let (download_target, result_rx) = HashingChunkConsumer::new_with_hash_provider();

    let download = task_builder
        .with_chunk_size(32 * 1024)
        .unwrap()
        .with_task_count(16)
        .unwrap()
        .begin_download(
            tmp_server.tmpfile_url.clone().try_into().unwrap(),
            download_target,
        )
        .await
        .unwrap();

    download.await_completion().await.unwrap();

    let downloaded_hash = result_rx.await.unwrap().unwrap();

    assert_eq!(tmp_server.tmpfile_sha256, downloaded_hash);
}
