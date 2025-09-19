#![allow(clippy::uninlined_format_args)]

use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{Args, Parser};
use tokio::sync::mpsc::error::TrySendError;
use tracing_subscriber::filter;
use tracing_subscriber::prelude::*;

#[derive(Parser, Debug)]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(styles = get_styles())]
pub struct Arguments<T: Args> {
    #[clap(flatten)]
    pub args: T,
    #[arg(global = true, short, long, help = "increase logging verbosity", action = clap::ArgAction::Count, conflicts_with = "quiet")]
    verbose: u8,
    #[arg(global = true, short, long, help = "decrease logging verbosity", action = clap::ArgAction::Count, conflicts_with = "verbose")]
    quiet: u8,
    #[arg(global = true, short, long, help = "print this help message", action = clap::ArgAction::Help)]
    help: Option<bool>,
    #[arg(short = 'V', long, help = "print version and build info")]
    version: bool,
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

pub async fn init<T: Args>(run_app: impl AsyncFnOnce(T)) {
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

    let args = Arguments::<T>::parse();

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
        _ = run_app(args.args) => {}
    }
}
