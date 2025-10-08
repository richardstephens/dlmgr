use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use anyhow::Error;
use async_trait::async_trait;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};
use tracing::info;

trait SendSyncDigest: DynDigest + Send + Sync {}

impl SendSyncDigest for Sha256 {}

pub struct HashingChunkConsumer {
    hasher: Box<dyn SendSyncDigest>,
}

impl HashingChunkConsumer {
    pub fn new() -> Self {
        let hasher = Box::new(Sha256::new());
        Self { hasher }
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
        info!("Hash: {}", hex::encode(hash));
    }

    async fn on_failure(mut self: Box<Self>) {
        todo!()
    }
}
