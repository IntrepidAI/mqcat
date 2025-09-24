#[tokio::main]
async fn main() {
    mqcat_zenoh::run(std::env::args()).await;
}
