use anyhow::{anyhow, bail};
use async_nats::{Client, HeaderMap};
use futures_util::{Stream, StreamExt};

use crate::{mqtrait::{Frame, MessageQueue}, utils::format_table};

struct NatsMQ {
    client: Client,
}

impl MessageQueue for NatsMQ {
    async fn connect(addr: Option<&str>) -> anyhow::Result<Self> {
        let client = async_nats::connect(addr.unwrap_or("nats://localhost:4222")).await?;
        Ok(Self { client })
    }

    async fn info(&self) -> anyhow::Result<String> {
        let mut info = vec![];
        let server_info = self.client.server_info();
        info.push(("Client ID", server_info.client_id.to_string()));
        info.push(("Client IP", server_info.client_ip));
        // info.push(("", String::new()));

        let has_name = server_info.server_name != server_info.server_id;
        info.push(("Server ID", server_info.server_id));
        if has_name {
            info.push(("Server Name", server_info.server_name));
        }

        info.push(("Server Address", format!("{}:{}", server_info.host, server_info.port)));
        info.push(("Server Version", format!("{} ({})", server_info.version, server_info.go)));

        info.push(("Headers Supported", server_info.headers.to_string()));
        info.push(("Maximum Payload", server_info.max_payload.to_string()));
        info.push(("Timeout", if let Some(timeout) = self.client.timeout() { format!("{:?}", timeout) } else { "None".to_string() }));

        Ok(format_table(&info))
    }

    async fn publish(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> anyhow::Result<()> {
        if topic.is_empty() {
            bail!("subject is empty");
        }
        let mut headermap = HeaderMap::new();
        for (key, value) in headers {
            headermap.insert(&**key, &**value);
        }
        self.client.publish_with_headers(topic.to_owned(), headermap, payload.to_vec().into()).await
            .map_err(|err| anyhow!("failed to publish: {}", err))?;
        self.client.flush().await?;
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> impl Stream<Item = anyhow::Result<Frame>> {
        let subscriber = self.client.subscribe(topic.to_owned());

        async_stream::try_stream! {
            let mut subscriber = subscriber.await?;
            while let Some(message) = subscriber.next().await {
                let mut frame = Frame {
                    topic: message.subject.to_string(),
                    payload: message.payload.into(),
                    headers: Default::default(),
                };
                if let Some(headers) = message.headers {
                    for (key, values) in headers.iter() {
                        frame.headers.insert(key.to_string(), values.iter().map(|v| v.to_string()).collect());
                    }
                }
                yield frame;
            }
        }
    }

    async fn request(&self, topic: &str, headers: &[(String, String)], payload: &[u8]) -> anyhow::Result<Frame> {
        if topic.is_empty() {
            bail!("subject is empty");
        }
        let mut headermap = HeaderMap::new();
        for (key, value) in headers {
            headermap.insert(&**key, &**value);
        }
        let res = self.client.request_with_headers(topic.to_owned(), headermap, payload.to_vec().into()).await
            .map_err(|err| anyhow!("failed to request: {}", err))?;
        let mut frame = Frame {
            topic: topic.to_owned(),
            headers: Default::default(),
            payload: res.payload.into(),
        };
        if let Some(headers) = res.headers {
            for (key, values) in headers.iter() {
                frame.headers.insert(key.to_string(), values.iter().map(|v| v.to_string()).collect());
            }
        }
        Ok(frame)
    }
}

pub async fn run(args: impl Iterator<Item = String>) {
    crate::cli::run::<NatsMQ>(args).await;
}
