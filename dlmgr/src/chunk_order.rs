use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use anyhow::bail;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::error;

pub async fn reorder_chunks(
    mut chunk_rx: mpsc::UnboundedReceiver<(u64, Vec<u8>)>,
    mut output: Box<dyn SequentialChunkConsumer>,
) -> anyhow::Result<()> {
    let mut next_offset = 0;
    let mut pending_chunks: HashMap<u64, Vec<u8>> = HashMap::new();
    let mut err = None;
    loop {
        if let Some((offset, chunk)) = chunk_rx.recv().await {
            if offset == next_offset {
                // we could avoid duplicating this segment by un-conditionally inserting
                // the chunk into the hashmap. need to experiment with this a bit more.
                let len = chunk.len() as u64;
                if let Err(e) = output.consume_bytes(chunk).await {
                    error!("Failed to write to output: {e}");
                    err = Some(e);
                    break;
                }
                next_offset += len;

                while let Some(chunk) = pending_chunks.remove(&next_offset) {
                    let len = chunk.len() as u64;
                    if let Err(e) = output.consume_bytes(chunk).await {
                        error!("Failed to write to output: {e}");
                        err = Some(e);
                        break;
                    }
                    next_offset += len;
                }
            } else {
                // todo: keep track of hashmap size
                pending_chunks.insert(offset, chunk);
            }
        } else {
            break;
        }
    }

    if pending_chunks.is_empty() && err.is_none() {
        output.finalise().await;
        Ok(())
    } else if let Some(e) = err {
        output.on_failure().await;
        Err(e)
    } else if !pending_chunks.is_empty() {
        // it may be needed to handle cases where we received some amount
        // of duplicate data. but maybe also better to detect duplicate
        // data and not store? need to think about this more.
        bail!("pending_chunks.len={} expected zero", pending_chunks.len());
    } else {
        // this should be unreachable.
        bail!("Unknown error.");
    }
}
