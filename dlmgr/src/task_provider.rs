use crate::worker::DlWorkerTask;
use std::cmp::min;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub(crate) struct TaskProvider {
    inner: Arc<TaskProviderInner>,
}

struct TaskProviderInner {
    chunk_size: u64,
    content_length: u64,
    task_state: RwLock<TaskState>,
}

#[derive(Default)]
struct TaskState {
    offset: u64,
}

impl TaskProvider {
    pub fn new_provider(chunk_size: u64, content_length: u64) -> Self {
        Self {
            inner: Arc::new(TaskProviderInner {
                chunk_size,
                content_length,
                task_state: Default::default(),
            }),
        }
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
}
