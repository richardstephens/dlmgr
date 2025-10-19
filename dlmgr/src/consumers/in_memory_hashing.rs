use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use anyhow::Error;
use async_trait::async_trait;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};
use tokio::sync::oneshot;
use tracing::{error, info};

trait SendSyncDigest: DynDigest + Send + Sync {}

impl SendSyncDigest for Sha256 {}

pub struct HashingChunkConsumer {
    hasher: Box<dyn SendSyncDigest>,
    completion: Option<oneshot::Sender<Result<Vec<u8>, ()>>>,
}

impl HashingChunkConsumer {
    pub fn new() -> Self {
        let hasher = Box::new(Sha256::new());
        Self {
            hasher,
            completion: None,
        }
    }

    pub fn new_with_hash_provider() -> (Self, oneshot::Receiver<Result<Vec<u8>, ()>>) {
        let hasher = Box::new(Sha256::new());
        let (tx, rx) = oneshot::channel();
        (
            Self {
                hasher,
                completion: Some(tx),
            },
            rx,
        )
    }
}

#[async_trait]
impl SequentialChunkConsumer for HashingChunkConsumer {
    async fn consume_bytes(&mut self, chunk: Vec<u8>) -> Result<(), Error> {
        self.hasher.update(&chunk);
        Ok(())
    }

    async fn finalise(mut self: Box<Self>) {
        let hash = self.hasher.finalize();
        info!("Hash: {}", hex::encode(&hash));
        if let Some(completion) = self.completion.take() {
            completion.send(Ok(hash.to_vec())).ok();
        }
    }

    async fn on_failure(mut self: Box<Self>) {
        error!("Download failed");
        if let Some(completion) = self.completion.take() {
            completion.send(Err(())).ok();
        }
    }
}
