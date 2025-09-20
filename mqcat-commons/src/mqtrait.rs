pub trait MessageQueue {
    fn connect() -> impl Future<Output = anyhow::Result<Self>> where Self: Sized;
    fn publish(&self, topic: &str, payload: &[u8]) -> impl Future<Output = anyhow::Result<()>>;
    fn subscribe(&self, topic: &str) -> impl futures_util::Stream<Item = anyhow::Result<(String, Vec<u8>)>>;
    fn request(&self, topic: &str, payload: &[u8]) -> impl Future<Output = anyhow::Result<Vec<u8>>>;
}
