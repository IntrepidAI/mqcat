use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, bail};
use futures_util::Stream;
use zenoh::Session;
use zenoh::bytes::Encoding;
use zenoh::query::QueryTarget;

use crate::mqtrait::{Frame, MessageQueue};
use crate::utils::format_table;

struct ZenohMQ {
    client: Session,
    // config: Config,
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

    async fn info(&self) -> anyhow::Result<String> {
        let mut info = vec![];
        let session_info = self.client.info();
        info.push(("Client ID", session_info.zid().await.to_string()));
        for router in session_info.routers_zid().await {
            info.push(("Connected Router ID", router.to_string()));
        }
        for peer in session_info.peers_zid().await {
            info.push(("Connected Peer ID", peer.to_string()));
        }

        Ok(format_table(&info))
    }

    async fn publish(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> anyhow::Result<()> {
        let mut encoding = Encoding::default();
        for (key, value) in headers {
            if key.eq_ignore_ascii_case("content-type") {
                encoding = Encoding::from_str(value)?;
            } else {
                log::warn!("unknown header: {}, zenoh only supports Content-Type", key);
            }
        }

        let publisher = self.client.declare_publisher(topic.to_owned())
            .encoding(encoding)
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

    fn subscribe(&self, topic: &str) -> impl Stream<Item = anyhow::Result<Frame>> {
        let subscriber = self.client.declare_subscriber(topic.to_owned());

        async_stream::try_stream! {
            let subscriber = subscriber.await
                .map_err(|err| anyhow!("declare failed: {}", err))?;
            loop {
                let sample = subscriber.recv_async().await
                    .map_err(|err| anyhow!("recv failed: {}", err))?;
                let mut frame = Frame {
                    topic: sample.key_expr().to_string(),
                    headers: Default::default(),
                    payload: sample.payload().to_bytes().to_vec(),
                };
                if sample.encoding() != &Encoding::default() {
                    frame.headers.insert("Content-Type".to_string(), vec![sample.encoding().to_string()]);
                }
                yield frame;
            }
        }
    }

    async fn request(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> anyhow::Result<Frame> {
        let mut encoding = Encoding::default();
        for (key, value) in headers {
            if key.eq_ignore_ascii_case("content-type") {
                encoding = Encoding::from_str(value)?;
            } else {
                log::warn!("unknown header: {}, zenoh only supports Content-Type", key);
            }
        }

        let querier = self.client.declare_querier(topic.to_owned())
            .target(QueryTarget::BestMatching)
            .await
            .map_err(|err| anyhow!("declare failed: {}", err))?;
        let replies = querier.get().payload(payload.to_vec()).encoding(encoding).await
            .map_err(|err| anyhow!("query failed: {}", err))?;
        let reply = replies.recv_async().await
            .map_err(|err| anyhow!("recv failed: {}", err))?;
        let result = reply.result().map_err(|err| anyhow!("result failed: {}", err))?;
        let mut frame = Frame {
            topic: result.key_expr().to_string(),
            headers: Default::default(),
            payload: result.payload().to_bytes().to_vec(),
        };
        if result.encoding() != &Encoding::default() {
            frame.headers.insert("Content-Type".to_string(), vec![result.encoding().to_string()]);
        }
        Ok(frame)
    }
}

pub async fn run(args: impl Iterator<Item = String>) {
    crate::cli::run::<ZenohMQ>(args).await;
}
