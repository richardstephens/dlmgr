use crate::error::DlMgrSetupError;
use crate::task::TaskStats;
use crate::task_builder::DownloadProps;
use crate::worker::DlWorkerTask;
use std::cmp::min;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub(crate) struct TaskProvider {
    inner: Arc<TaskProviderInner>,
}

struct TaskProviderInner {
    chunk_size: u64,
    content_length: u64,
    task_state: RwLock<TaskState>,
    task_stats: Arc<TaskStats>,
    max_buffer_chunks: u32,
    cache_space: Arc<Semaphore>,
}

#[derive(Default)]
struct TaskState {
    offset: u64,
}

impl TaskProvider {
    pub fn new_provider(
        props: &DownloadProps,
        task_stats: Arc<TaskStats>,
        content_length: u64,
    ) -> Result<Self, DlMgrSetupError> {
        let max_buffer_chunks: u32 = props
            .max_buffer_chunks
            .unwrap_or_else(|| props.task_count as u32 * 2);

        if max_buffer_chunks as usize > Semaphore::MAX_PERMITS {
            return Err(DlMgrSetupError::InvalidMaxBufferChunks);
        }

        let cache_space = Arc::new(Semaphore::new(max_buffer_chunks as usize));

        Ok(Self {
            inner: Arc::new(TaskProviderInner {
                chunk_size: props.chunk_size,
                content_length,
                task_state: Default::default(),
                task_stats,
                max_buffer_chunks,
                cache_space,
            }),
        })
    }

    pub fn next_task(&self) -> Option<DlWorkerTask> {
        let mut state_write = self.inner.task_state.write().unwrap();
        if state_write.offset < self.inner.content_length {
            let remaining_bytes = self.inner.content_length - state_write.offset;
            let len = min(remaining_bytes, self.inner.chunk_size);
            let task = DlWorkerTask {
                offset: state_write.offset,
                len,
            };
            state_write.offset += len;
            Some(task)
        } else {
            None
        }
    }
    pub async fn next_task_throttled(
        &self,
    ) -> anyhow::Result<Option<(DlWorkerTask, OwnedSemaphorePermit)>> {
        let next_task = self.next_task();
        //let mut throttled = false;
        if let Some(next_task) = next_task {
            let permit = self.inner.cache_space.clone().acquire_owned().await?;
            Ok(Some((next_task, permit)))
        } else {
            Ok(None)
        }
        /*
        if throttled {
            self.inner
                .task_stats
                .throttle_events
                .fetch_add(1, Ordering::SeqCst);
        }*/
    }
}
