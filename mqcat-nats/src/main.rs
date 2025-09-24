#[tokio::main]
async fn main() {
    mqcat_nats::run(std::env::args()).await;
}
