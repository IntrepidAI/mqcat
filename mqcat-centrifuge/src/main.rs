#[tokio::main]
async fn main() {
    mqcat_centrifuge::run(std::env::args()).await;
}
