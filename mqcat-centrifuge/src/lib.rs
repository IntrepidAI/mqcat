use anyhow::anyhow;
use futures_util::Stream;
use mqcat_commons::mqtrait::MessageQueue;
use tokio_centrifuge::client::Client;
use tokio_centrifuge::config::Config;

struct CentrifugeMQ {
    client: Client,
}

impl MessageQueue for CentrifugeMQ {
    async fn connect(addr: Option<&str>) -> anyhow::Result<Self> {
        let client = Client::new(
            addr.unwrap_or("ws://localhost:8000/connection/websocket?format=protobuf"),
            Config::new().use_json()
        );

        client.on_connecting(|e| {
            log::debug!("connecting (code={}, reason={})", e.code, e.reason);
        });
        client.on_connected(|e| {
            log::debug!("connected, client_id={}, version={}", e.client_id, e.version);
        });
        client.on_disconnected(|e| {
            log::debug!("disconnected (code={}, reason={})", e.code, e.reason);
        });
        client.on_error(|err| {
            log::error!("error: {:?}", err);
        });

        client.connect().await.map_err(|_| anyhow::Error::msg("failed to connect"))?;

        Ok(Self { client })
    }

    async fn publish(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        self.client.publish(topic, payload.to_vec()).await
            .map_err(|err| anyhow!("failed to publish: {}", err))?;
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> impl Stream<Item = anyhow::Result<(String, Vec<u8>)>> {
        let sub = self.client.new_subscription(topic);
        let (recv_tx, mut recv_rx) = tokio::sync::mpsc::channel(64);
        let (unsub_tx, mut unsub_rx) = tokio::sync::mpsc::channel(1);

        sub.on_subscribed(|e| {
            log::debug!("subscribed to {}", e.channel);
        });
        let unsub_tx_ = unsub_tx.clone();
        sub.on_unsubscribed(move |e| {
            log::debug!("unsubscribed from {} (code={}, reason={})", e.channel, e.code, e.reason);
            let _ = unsub_tx_.try_send((e.code, e.reason.to_owned()));
        });
        sub.on_subscribing(|e| {
            log::debug!("subscribing to {} (code={}, reason={})", e.channel, e.code, e.reason);
        });
        sub.on_publication(move |data| {
            if let Err(err) = recv_tx.try_send((data.channel, data.data)) {
                let _ = unsub_tx.try_send((0, format!("{}", err)));
            }
        });

        sub.subscribe();

        async_stream::try_stream! {
            loop {
                tokio::select! {
                    Some((mut channel, data)) = recv_rx.recv() => {
                        if channel.is_empty() {
                            channel = topic.to_owned();
                        }
                        yield (channel, data);
                    }
                    Some((code, reason)) = unsub_rx.recv() => {
                        break Err(anyhow!("subscription failed: {} {}", code, reason));
                    }
                    else => {
                        break Err(anyhow!("subscription closed"));
                    }
                }
            }?;
        }
    }

    async fn request(&self, topic: &str, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
        let res = self.client.rpc(topic, payload.to_vec()).await
            .map_err(|err| anyhow!("failed to execute rpc: {}", err))?;
        Ok(res)
    }
}

pub async fn run(args: impl Iterator<Item = String>) {
    mqcat_commons::cli::run::<CentrifugeMQ>(args).await;
}
