use async_trait::async_trait;

#[async_trait]
pub trait SequentialChunkConsumer: Send + Sync {
    async fn consume_bytes(&mut self, chunk: Vec<u8>) -> Result<(), anyhow::Error>;

    async fn finalise(self: Box<Self>);

    async fn on_failure(self: Box<Self>);
}

impl<T> From<T> for Box<dyn SequentialChunkConsumer>
where
    T: SequentialChunkConsumer + 'static,
{
    fn from(value: T) -> Self
    where
        T: SequentialChunkConsumer + 'static,
    {
        Box::new(value)
    }
}
