use crate::api::client_provider::{DefaultReqwestClientProvider, ReqwestClientProvider};
use crate::api::sequential_chunk_consumer::SequentialChunkConsumer;
use crate::error::{DlMgrSetupError, TaskBuilderError};
use crate::task::DownloadTask;
use crate::urlset::UrlSet;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DownloadProps {
    pub(crate) task_count: u8,
    pub(crate) chunk_size: u32,
    pub(crate) max_buffer_size: Option<usize>,
    pub(crate) client_provider: Arc<Box<dyn ReqwestClientProvider>>,
}

#[derive(Clone)]
pub struct DownloadTaskBuilder {
    props: DownloadProps,
}

impl DownloadTaskBuilder {
    pub fn new() -> Self {
        Self {
            props: DownloadProps {
                task_count: 4,
                chunk_size: 4 * 1024 * 1024,
                max_buffer_size: None,
                client_provider: Arc::new(Box::new(DefaultReqwestClientProvider)),
            },
        }
    }

    pub fn with_task_count(mut self, task_count: u8) -> Result<Self, TaskBuilderError> {
        if task_count == 0 {
            Err(TaskBuilderError::InvalidParameterValue("task_count"))
        } else {
            self.props.task_count = task_count;
            Ok(self)
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: u32) -> Result<Self, TaskBuilderError> {
        if chunk_size == 0 {
            Err(TaskBuilderError::InvalidParameterValue("chunk_size"))
        } else {
            self.props.chunk_size = chunk_size;
            Ok(self)
        }
    }

    pub fn with_max_buffer_size(
        mut self,
        max_buffer_size: usize,
    ) -> Result<Self, TaskBuilderError> {
        if max_buffer_size == 0 {
            Err(TaskBuilderError::InvalidParameterValue("max_buffer_size"))
        } else {
            self.props.max_buffer_size = Some(max_buffer_size);
            Ok(self)
        }
    }

    pub fn with_client_provider(
        mut self,
        provider: Box<dyn ReqwestClientProvider>,
    ) -> Result<Self, TaskBuilderError> {
        self.props.client_provider = Arc::new(provider);
        Ok(self)
    }

    pub async fn begin_download(
        &self,
        url: UrlSet,
        target: impl Into<Box<dyn SequentialChunkConsumer>>,
    ) -> Result<DownloadTask, DlMgrSetupError> {
        crate::spawner::spawn_download_task(url.into(), target.into(), self.props.clone()).await
    }
}
