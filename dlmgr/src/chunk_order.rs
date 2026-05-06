use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use crate::error::{DownloadWorkerError, ReorderChunkError};
use crate::task::TaskStats;
use std::cmp::max;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{OwnedSemaphorePermit, mpsc};
use tracing::error;

pub async fn reorder_chunks(
    mut chunk_rx: mpsc::UnboundedReceiver<(u64, Vec<u8>, OwnedSemaphorePermit)>,
    mut output: Box<dyn SequentialChunkConsumer>,
    task_stats: Arc<TaskStats>,
) -> Result<(), DownloadWorkerError> {
    let mut next_offset = 0;
    let mut pending_chunks: HashMap<u64, (Vec<u8>, OwnedSemaphorePermit)> = HashMap::new();
    let mut err = None;

    let mut furthest_offset: u64 = 0; // used for tracking cached bytes for backpressure

    while let Some((offset, chunk, permit)) = chunk_rx.recv().await {
        furthest_offset = max(offset + chunk.len() as u64, furthest_offset);
        task_stats
            .bytes_downloaded
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
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

            while let Some((chunk, permit)) = pending_chunks.remove(&next_offset) {
                let len = chunk.len() as u64;
                if let Err(e) = output.consume_bytes(chunk).await {
                    error!("Failed to write to output: {e}");
                    err = Some(e);
                    break;
                }
                drop(permit);
                next_offset += len;
            }

            // TODO: is this check necessary? this *should* be provably impossible.
            // need to think about the offset math a bit more.
            if let Some(cached_bytes) = furthest_offset.checked_sub(next_offset) {
                task_stats
                    .cached_bytes
                    .store(cached_bytes, Ordering::SeqCst);
            } else {
                return Err(DownloadWorkerError::ReorderChunks(
                    ReorderChunkError::NextOffsetAheadOfFurthestOffset(
                        next_offset,
                        furthest_offset,
                    ),
                ));
            }
        } else if offset > next_offset {
            // todo: might it be beneficial to re-request backlogged data?
            // we may need to handle overlapping chunks.
            if pending_chunks.insert(offset, (chunk, permit)).is_some() {
                return Err(DownloadWorkerError::ReorderChunks(
                    ReorderChunkError::DuplicateChunk(offset),
                ));
            }
        } else {
            return Err(DownloadWorkerError::ReorderChunks(
                ReorderChunkError::ReceivedUnexpectedChunk(offset, next_offset),
            ));
        }
    }

    if pending_chunks.is_empty() && err.is_none() {
        output.finalise().await;
        Ok(())
    } else if let Some(e) = err {
        output.on_failure().await;
        Err(DownloadWorkerError::ReorderChunks(
            ReorderChunkError::ChunkConsumer(e),
        ))
    } else if !pending_chunks.is_empty() {
        // it may be needed to handle cases where we received some amount
        // of duplicate data. but maybe also better to detect duplicate
        // data and not store? need to think about this more.
        return Err(DownloadWorkerError::ReorderChunks(
            ReorderChunkError::LeftoverChunks(pending_chunks.len()),
        ));
    } else {
        // this should be unreachable.
        Err(DownloadWorkerError::ReorderChunks(
            ReorderChunkError::UnreachableCondition,
        ))
    }
}
