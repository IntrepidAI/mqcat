use anyhow::{anyhow, bail};
use async_nats::Client;
use futures_util::{Stream, StreamExt};

use crate::mqtrait::MessageQueue;

struct NatsMQ {
    client: Client,
}

impl MessageQueue for NatsMQ {
    async fn connect(addr: Option<&str>) -> anyhow::Result<Self> {
        let client = async_nats::connect(addr.unwrap_or("nats://localhost:4222")).await?;
        Ok(Self { client })
    }

    async fn publish(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        if topic.is_empty() {
            bail!("subject is empty");
        }
        self.client.publish(topic.to_owned(), payload.to_vec().into()).await
            .map_err(|err| anyhow!("failed to publish: {}", err))?;
        self.client.flush().await?;
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> impl Stream<Item = anyhow::Result<(String, Vec<u8>)>> {
        let subscriber = self.client.subscribe(topic.to_owned());

        async_stream::try_stream! {
            let mut subscriber = subscriber.await?;
            while let Some(message) = subscriber.next().await {
                yield (message.subject.to_string(), message.payload.into());
            }
        }
    }

    async fn request(&self, topic: &str, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
        if topic.is_empty() {
            bail!("subject is empty");
        }
        let res = self.client.request(topic.to_owned(), payload.to_vec().into()).await
            .map_err(|err| anyhow!("failed to request: {}", err))?;
        Ok(res.payload.into())
    }
}

pub async fn run(args: impl Iterator<Item = String>) {
    crate::cli::run::<NatsMQ>(args).await;
}
