use std::io::Write;
use std::pin::pin;

use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use futures_util::StreamExt;
use tokio::sync::mpsc::error::TrySendError;
use tracing_subscriber::filter;
use tracing_subscriber::prelude::*;

use crate::mqtrait::MessageQueue;

#[derive(Parser, Debug)]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(styles = get_styles())]
pub struct BaseArgs/*<T: Args>*/ {
    // #[clap(flatten)]
    // pub args: T,
    #[arg(global = true, short, long, help = "increase logging verbosity", action = clap::ArgAction::Count, conflicts_with = "quiet")]
    verbose: u8,
    #[arg(global = true, short, long, help = "decrease logging verbosity", action = clap::ArgAction::Count, conflicts_with = "verbose")]
    quiet: u8,
    #[arg(global = true, short, long, help = "print this help message", action = clap::ArgAction::Help)]
    help: Option<bool>,
    #[arg(short = 'V', long, help = "print version and build info")]
    version: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser, Debug)]
enum Commands {
    #[command(about = "publish a message to a channel", alias = "pub")]
    Publish {
        #[arg(help = "channel name")]
        channel: String,
        #[arg(help = "data to publish")]
        data: String,
    },

    #[command(about = "subscribe to a channel", alias = "sub")]
    Subscribe {
        #[arg(help = "channel name")]
        channel: String,
    },

    #[command(about = "request a message from a channel", alias = "req")]
    Request {
        #[arg(help = "channel name")]
        channel: String,
        #[arg(help = "request data")]
        data: String,
    },
}

fn get_styles() -> Styles {
    // clap v3 styles, see
    // https://stackoverflow.com/questions/74068168/clap-rs-not-printing-colors-during-help
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Yellow.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

pub async fn init(run_app: impl AsyncFnOnce(BaseArgs) -> anyhow::Result<()>) {
    // set it up so:
    //  - ctrl-c stops polling current async task
    //  - double ctrl-c stops the process
    let mut abort_recv = {
        let (abort_send, abort_recv) = tokio::sync::mpsc::channel(1);
        let mut last_sent: Option<std::time::Instant> = None;

        ctrlc::set_handler(move || {
            if let Some(last_sent) = last_sent {
                if last_sent.elapsed() < std::time::Duration::from_secs(10) {
                    log::error!("Received SIGINT again, aborting...");
                    std::process::exit(1);
                }
            }

            last_sent = Some(std::time::Instant::now());
            match abort_send.try_send(()) {
                Ok(()) => {
                    log::error!("Received SIGINT, exiting...");
                }
                Err(TrySendError::Closed(_)) => {
                    log::error!("Received SIGINT, aborting...");
                    std::process::exit(1);
                }
                Err(TrySendError::Full(_)) => {
                    log::error!("Received SIGINT again, aborting...");
                    std::process::exit(1);
                }
            }
        }).unwrap();

        abort_recv
    };

    let args = BaseArgs::parse();

    // set verbosity level, default is info
    let filter_layer = filter::EnvFilter::builder()
        .with_default_directive((match (args.verbose as i32).saturating_sub(args.quiet as i32) {
            ..=-2 => filter::LevelFilter::ERROR,
            -1 => filter::LevelFilter::WARN,
            0 => filter::LevelFilter::INFO,
            1 => filter::LevelFilter::DEBUG,
            2.. => filter::LevelFilter::TRACE,
        }).into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    if args.version {
        crate::version::print_version();
        return;
    }

    // run app, abort on ctrl-c
    tokio::select! {
        _ = abort_recv.recv() => {}
        result = run_app(args) => {
            if let Err(e) = result {
                log::error!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

pub async fn run<Q: MessageQueue>() {
    init(|args: BaseArgs| async {
        match args.command {
            Some(Commands::Publish { channel, data }) => {
                let mq = Q::connect().await?;
                mq.publish(&channel, data.as_bytes()).await?;
                log::info!("published {} bytes to \"{}\"", data.len(), channel);
            }
            Some(Commands::Subscribe { channel }) => {
                let mut idx = 0;
                let mq = Q::connect().await?;
                let stream = mq.subscribe(&channel);
                let mut stream = pin!(stream);
                while let Some(msg) = stream.next().await {
                    let (channel, data) = msg?;
                    idx += 1;
                    std::io::stdout().write_all(
                        format!("[#{idx}] Received on \"{}\" ({} bytes)\n", channel, data.len()).as_bytes()
                    )?;
                    let data = String::from_utf8_lossy(&data);
                    std::io::stdout().write_all(data.as_bytes())?;
                    std::io::stdout().write_all(b"\n\n\n")?;
                    std::io::stdout().flush()?;
                }
            }
            Some(Commands::Request { channel, data }) => {
                let mq = Q::connect().await?;
                log::info!("sending request to \"{}\"", channel);
                let data = mq.request(&channel, data.as_bytes()).await?;
                std::io::stdout().write_all(
                    format!("[#0] Received on \"{}\" ({} bytes)\n", channel, data.len()).as_bytes()
                )?;
                std::io::stdout().write_all(&data)?;
                std::io::stdout().write_all(b"\n\n\n")?;
                std::io::stdout().flush()?;
            }
            None => {
                use clap::CommandFactory;
                let _ = BaseArgs::command().print_help();
            }
        }

        Ok(())
    }).await;
}
