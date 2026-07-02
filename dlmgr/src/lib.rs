pub mod api;
mod chunk_order;
pub mod consumers;
pub mod error;
pub(crate) mod response_helpers;
mod sequential;
mod spawner;
mod task;
mod task_builder;
mod task_provider;
pub(crate) mod urlset;
mod worker;

pub use task::{DownloadTask, ProgressProvider};
pub use task_builder::{ConcurrencyBehaviour, DownloadTaskBuilder};
