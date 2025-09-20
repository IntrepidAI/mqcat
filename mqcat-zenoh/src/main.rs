use std::time::Duration;

use anyhow::anyhow;
use futures_util::Stream;
use mqcat_commons::mqtrait::MessageQueue;
use zenoh::Session;

struct ZenohMQ {
    client: Session,
}

impl Drop for ZenohMQ {
    fn drop(&mut self) {
        let _ = zenoh::Wait::wait(self.client.close().timeout(Duration::ZERO));
    }
}

impl MessageQueue for ZenohMQ {
    async fn connect() -> anyhow::Result<Self> {
        let mut config = zenoh::Config::default();
        config
            .insert_json5("connect/endpoints", &serde_json::json!(["tcp/172.17.183.14:7447"]).to_string())
            .unwrap();

        let zenoh = zenoh::open(config).await
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self { client: zenoh })
    }

    async fn publish(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        self.client.put(topic.to_owned(), payload.to_vec()).await
            .map_err(|err| anyhow!("failed to publish: {}", err))?;
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> impl Stream<Item = anyhow::Result<(String, Vec<u8>)>> {
        let subscriber = self.client.declare_subscriber(topic.to_owned());

        async_stream::try_stream! {
            let subscriber = subscriber.await
                .map_err(|err| anyhow!("declare failed: {}", err))?;
            loop {
                let sample = subscriber.recv_async().await
                    .map_err(|err| anyhow!("recv failed: {}", err))?;
                yield (sample.key_expr().to_string(), sample.payload().to_bytes().to_vec());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    mqcat_commons::cli::run::<ZenohMQ>().await;
}
