use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use atomic_write_file::AtomicWriteFile;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tracing::warn;

pub struct AtomicFileConsumerSha256 {
    file: Arc<Mutex<Option<AtomicWriteFile>>>,
    hasher: Sha256,
    expected_hash: Option<[u8; 32]>,
    tx: Option<oneshot::Sender<anyhow::Result<()>>>,
}

impl AtomicFileConsumerSha256 {
    pub async fn new(
        target_file: PathBuf,
        expected_hash: [u8; 32],
    ) -> Result<
        (
            AtomicFileConsumerSha256,
            oneshot::Receiver<anyhow::Result<()>>,
        ),
        Error,
    > {
        let file =
            tokio::task::spawn_blocking(move || AtomicWriteFile::open(target_file)).await??;
        let hasher = Sha256::new();

        let (tx, rx) = oneshot::channel();

        Ok((
            AtomicFileConsumerSha256 {
                file: Arc::new(Mutex::new(Some(file))),
                hasher,
                expected_hash: Some(expected_hash),
                tx: Some(tx),
            },
            rx,
        ))
    }

    pub async fn new_unknown_hash(
        target_file: PathBuf,
    ) -> Result<
        (
            AtomicFileConsumerSha256,
            oneshot::Receiver<anyhow::Result<()>>,
        ),
        Error,
    > {
        let file =
            tokio::task::spawn_blocking(move || AtomicWriteFile::open(target_file)).await??;
        let hasher = Sha256::new();

        let (tx, rx) = oneshot::channel();

        Ok((
            AtomicFileConsumerSha256 {
                file: Arc::new(Mutex::new(Some(file))),
                hasher,
                expected_hash: None,
                tx: Some(tx),
            },
            rx,
        ))
    }
}

#[async_trait]
impl SequentialChunkConsumer for AtomicFileConsumerSha256 {
    async fn consume_bytes(&mut self, chunk: Vec<u8>) -> Result<(), Error> {
        self.hasher.update(&chunk);
        let file = Arc::clone(&self.file);
        tokio::task::spawn_blocking(move || {
            if let Some(file) = file.blocking_lock().as_mut() {
                use std::io::Write;
                file.write_all(&chunk)
            } else {
                //TODO : this should be an error
                Ok(())
            }
        })
        .await??;
        Ok(())
    }

    async fn finalise(mut self: Box<Self>) {
        let hash: [u8; 32] = self.hasher.finalize().into();
        if let Some(expected_hash) = self.expected_hash
            && hash != expected_hash
        {
            if let Some(tx) = self.tx.take() {
                tx.send(Err(anyhow!("Hash mismatch: actual={}", hex::encode(hash))))
                    .ok();
            }
            return;
        }
        let outcome = commit_file(&self.file).await;
        if let Some(tx) = self.tx.take() {
            tx.send(outcome).ok();
        } else {
            warn!(
                "atomic_file_consumer finalise but completion channel missing. outcome={outcome:?}"
            );
        }
    }

    async fn on_failure(mut self: Box<Self>) {
        if let Some(tx) = self.tx.take() {
            tx.send(Err(anyhow!("dlmgr reported download failure")))
                .ok();
        }
    }
}

async fn commit_file(file: &Arc<Mutex<Option<AtomicWriteFile>>>) -> anyhow::Result<()> {
    if let Some(file) = file.lock().await.take() {
        match tokio::task::spawn_blocking(move || -> Result<(), Error> {
            file.commit()?;
            Ok(())
        })
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(anyhow!("Commit file panicked: {:?}", e)),
        }
    } else {
        Err(anyhow!("file missing"))
    }
}
