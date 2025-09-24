#[tokio::main]
async fn main() {
    mqcat_centrifuge::run::<true>(std::env::args()).await;
}
