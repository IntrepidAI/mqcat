use std::time::Duration;

use anyhow::{anyhow, bail};
use futures_util::Stream;
use zenoh::Session;
use zenoh::query::QueryTarget;

use crate::mqtrait::MessageQueue;

struct ZenohMQ {
    client: Session,
}

impl Drop for ZenohMQ {
    fn drop(&mut self) {
        let _ = zenoh::Wait::wait(self.client.close().timeout(Duration::ZERO));
    }
}

impl MessageQueue for ZenohMQ {
    async fn connect(addr: Option<&str>) -> anyhow::Result<Self> {
        let mut config = zenoh::Config::default();
        if let Some(addr) = addr {
            config
                .insert_json5("connect/endpoints", &serde_json::json!([addr]).to_string())
                .unwrap();
        }

        let zenoh = zenoh::open(config).await
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self { client: zenoh })
    }

    async fn publish(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        let publisher = self.client.declare_publisher(topic.to_owned())
            .await
            .map_err(|err| anyhow!("declare failed: {}", err))?;

        let matching_listener = publisher.matching_listener()
            .await
            .map_err(|err| anyhow!("matching listener failed: {}", err))?;

        match tokio::time::timeout(Duration::from_secs(5), matching_listener.recv_async()).await {
            Ok(Ok(result)) => {
                log::debug!("matching listener status: {:?}", result.matching());
            }
            Ok(Err(err)) => {
                bail!("recv failed: {}", err)
            }
            Err(_) => {
                bail!("failed to find matching listeners")
            }
        }

        publisher.put(payload.to_vec()).await
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

    async fn request(&self, topic: &str, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
        let querier = self.client.declare_querier(topic.to_owned())
            .target(QueryTarget::BestMatching)
            .await
            .map_err(|err| anyhow!("declare failed: {}", err))?;
        let replies = querier.get().payload(payload.to_vec()).await
            .map_err(|err| anyhow!("query failed: {}", err))?;
        let reply = replies.recv_async().await
            .map_err(|err| anyhow!("recv failed: {}", err))?;
        let result = reply.result().map_err(|err| anyhow!("result failed: {}", err))?;
        Ok(result.payload().to_bytes().to_vec())
    }
}

pub async fn run(args: impl Iterator<Item = String>) {
    crate::cli::run::<ZenohMQ>(args).await;
}
