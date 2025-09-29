pub struct Frame {
    pub topic: String,
    pub headers: std::collections::BTreeMap<String, Vec<String>>,
    pub payload: Vec<u8>,
}

pub trait MessageQueue {
    fn connect(addr: Option<&str>) -> impl Future<Output = anyhow::Result<Self>> where Self: Sized;
    fn info(&self) -> impl Future<Output = anyhow::Result<String>>;
    fn publish(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> impl Future<Output = anyhow::Result<()>>;
    fn subscribe(&self, topic: &str) -> impl futures_util::Stream<Item = anyhow::Result<Frame>>;
    fn request(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> impl Future<Output = anyhow::Result<Frame>>;
}
