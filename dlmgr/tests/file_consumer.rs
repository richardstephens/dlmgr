use dlmgr::DownloadTaskBuilder;
use dlmgr::consumers::atomic_file_consumer_sha256::{AtomicFileConsumerSha256, CompletionMessage};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[tokio::test]
async fn file_consumer_known_hash() {
    let dir = tempfile::tempdir().unwrap();
    let target_path = dir.path().join(Uuid::new_v4().to_string());

    let server = range_server::simple::start_temp_server().await;
    let (download_target, result_rx) = AtomicFileConsumerSha256::new(
        target_path.clone(),
        server.tmpfile_sha256.clone().try_into().unwrap(),
    )
    .await
    .unwrap();

    let download = DownloadTaskBuilder::new()
        .begin_download(
            server.tmpfile_url.clone().try_into().unwrap(),
            download_target,
        )
        .await
        .unwrap();

    download.await_completion().await.unwrap();

    let completion: CompletionMessage = result_rx.await.unwrap().unwrap();
    assert_eq!(
        completion,
        CompletionMessage {
            path: target_path.clone(),
            sha256: server.tmpfile_sha256.clone().try_into().unwrap(),
        }
    );

    assert!(target_path.exists());

    let mut hasher = Sha256::new();
    hasher.update(tokio::fs::read(&target_path).await.unwrap());
    assert_eq!(hasher.finalize().to_vec(), server.tmpfile_sha256);
}
