use anyhow::{anyhow, bail};
use async_nats::Client;
use futures_util::{Stream, StreamExt};
use mqcat_commons::mqtrait::MessageQueue;

struct NatsMQ {
    client: Client,
}

impl MessageQueue for NatsMQ {
    async fn connect() -> anyhow::Result<Self> {
        let client = async_nats::connect("nats://172.17.183.14:4222").await?;
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
}

#[tokio::main]
async fn main() {
    mqcat_commons::cli::run::<NatsMQ>().await;
}
