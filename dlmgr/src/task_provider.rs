use crate::error::DlMgrSetupError;
use crate::task_builder::DownloadProps;
use crate::worker::DlWorkerTask;
use std::cmp::min;
use std::sync::{Arc, RwLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub(crate) struct TaskProvider {
    inner: Arc<TaskProviderInner>,
}

struct TaskProviderInner {
    chunk_size: u32,
    content_length: u64,
    task_state: RwLock<TaskState>,
    cache_space: Arc<Semaphore>,
}

#[derive(Default)]
struct TaskState {
    offset: u64,
}

impl TaskProvider {
    pub fn new_provider(
        props: &DownloadProps,
        content_length: u64,
    ) -> Result<Self, DlMgrSetupError> {
        let max_buffer_size: usize = props
            .max_buffer_size
            .or_else(|| (props.chunk_size as usize).checked_mul(props.task_count as usize))
            .ok_or_else(|| DlMgrSetupError::InvalidMaxBufferSize)?;

        if max_buffer_size > Semaphore::MAX_PERMITS {
            return Err(DlMgrSetupError::InvalidMaxBufferSize);
        }

        let cache_space = Arc::new(Semaphore::new(max_buffer_size));

        Ok(Self {
            inner: Arc::new(TaskProviderInner {
                chunk_size: props.chunk_size,
                content_length,
                task_state: Default::default(),
                cache_space,
            }),
        })
    }

    pub fn next_task(&self) -> Option<DlWorkerTask> {
        let mut state_write = self.inner.task_state.write().unwrap();
        if state_write.offset < self.inner.content_length {
            let remaining_bytes = self.inner.content_length - state_write.offset;
            let len = min(remaining_bytes, self.inner.chunk_size as u64);
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
        if let Some(next_task) = next_task {
            let permits = self
                .inner
                .cache_space
                .clone()
                .acquire_many_owned(next_task.len as u32)
                .await?;
            Ok(Some((next_task, permits)))
        } else {
            Ok(None)
        }
    }
}
