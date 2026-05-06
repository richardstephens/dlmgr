use anyhow::anyhow;
use async_trait::async_trait;
use dlmgr::DownloadTaskBuilder;
use dlmgr::api::sequential_chunk_consumer::SequentialChunkConsumer;

struct FailingConsumer;

#[async_trait]
impl SequentialChunkConsumer for FailingConsumer {
    async fn consume_bytes(&mut self, _chunk: Vec<u8>) -> Result<(), anyhow::Error> {
        Err(anyhow!("consumer failed"))
    }

    async fn finalise(self: Box<Self>) {}

    async fn on_failure(self: Box<Self>) {}
}

#[tokio::test]
async fn consumer_error_propagates_to_completion() {
    let tmp_server = range_server::simple::start_temp_server().await;

    let download = DownloadTaskBuilder::new()
        .with_chunk_size(32 * 1024)
        .unwrap()
        .with_task_count(4)
        .unwrap()
        .begin_download(
            tmp_server.tmpfile_url.clone().try_into().unwrap(),
            FailingConsumer,
        )
        .await
        .unwrap();

    let result = download.await_completion().await;

    assert!(
        result.is_err(),
        "expected await_completion to return Err when the chunk consumer fails, got {result:?}"
    );
}
