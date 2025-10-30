use crate::task::TaskStats;
use crate::task_builder::DownloadProps;
use crate::worker::DlWorkerTask;
use std::cmp::min;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct TaskProvider {
    inner: Arc<TaskProviderInner>,
}

struct TaskProviderInner {
    chunk_size: u64,
    content_length: u64,
    task_state: RwLock<TaskState>,
    task_stats: Arc<TaskStats>,
    max_buffer_size: u64,
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
    ) -> Self {
        let max_buffer_size = props
            .max_buffer_size
            .unwrap_or_else(|| props.chunk_size * min(1, props.task_count as u64));
        Self {
            inner: Arc::new(TaskProviderInner {
                chunk_size: props.chunk_size,
                content_length,
                task_state: Default::default(),
                task_stats,
                max_buffer_size,
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
    pub async fn next_task_throttled(&self) -> Option<DlWorkerTask> {
        let next_task = self.next_task();
        let mut throttled = false;
        if next_task.is_some() {
            loop {
                if self.inner.task_stats.cached_bytes.load(Ordering::SeqCst)
                    > self.inner.max_buffer_size
                {
                    throttled = true;
                    tokio::time::sleep(Duration::from_millis(1)).await;
                } else {
                    break;
                }
            }
        }

        if throttled {
            self.inner
                .task_stats
                .throttle_events
                .fetch_add(1, Ordering::SeqCst);
        }

        next_task
    }
}
