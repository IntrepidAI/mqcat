use clap::Parser;

#[derive(Parser, Debug)]
struct CustomArgs {
}

async fn run(mut args: CustomArgs) {
    log::info!("Hello, world!");
}

#[tokio::main]
async fn main() {
    mqcat_commons::cli::init(|args| async move {
        run(args).await;
    }).await;
}
