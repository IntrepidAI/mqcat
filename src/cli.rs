use std::borrow::Cow;
use std::io::Write;
use std::pin::pin;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::error::TrySendError;
use tracing_subscriber::filter;
use tracing_subscriber::prelude::*;

use crate::mqtrait::Frame;
use crate::mqtrait::MessageQueue;

#[derive(Parser, Debug)]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(styles = get_styles())]
pub struct BaseArgs/*<T: Args>*/ {
    // #[clap(flatten)]
    // pub args: T,
    #[arg(global = true, short, long, action = clap::ArgAction::Count, conflicts_with = "quiet")]
    /// increase logging verbosity
    verbose: u8,
    #[arg(global = true, short, long, action = clap::ArgAction::Count, conflicts_with = "verbose")]
    /// decrease logging verbosity
    quiet: u8,
    #[arg(global = true, short, long, action = clap::ArgAction::Help)]
    /// print this help message
    help: Option<bool>,
    #[arg(global = true, short = 'V', long)]
    /// print version and build info
    version: bool,
    /// server address
    url: String,
    #[command(subcommand)]
    /// command (pub, sub, etc.)
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
        #[arg(short = 'H', long, help = "add header to the message", value_parser = parse_header)]
        header: Vec<(String, String)>,
        #[arg(long, help = "publish multiple messages", default_value = "1")]
        count: u32,
        #[arg(long, help = "sleep between messages", default_value = "0", value_parser = parse_duration)]
        sleep: Duration,
    },

    #[command(about = "subscribe to a channel", alias = "sub")]
    Subscribe {
        #[arg(help = "channel name")]
        channel: String,
        #[arg(long, help = "decode the message by passing it through a given command")]
        translate: Option<String>,
    },

    #[command(about = "request a message from a channel", alias = "req")]
    Request {
        #[arg(help = "channel name")]
        channel: String,
        #[arg(help = "request data")]
        data: String,
        #[arg(short = 'H', long, help = "add header to the message", value_parser = parse_header)]
        header: Vec<(String, String)>,
        #[arg(long, help = "publish multiple messages", default_value = "1")]
        count: u32,
        #[arg(long, help = "decode the message by passing it through a given command")]
        translate: Option<String>,
    },
}

fn parse_header(s: &str) -> Result<(String, String), String> {
    let parts = s.splitn(2, ':').collect::<Vec<&str>>();
    if parts.len() != 2 {
        return Err("header must be in the format of \"key: value\"".to_string());
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let duration = go_parse_duration::parse_duration(s)
        .map_err(|go_parse_duration::Error::ParseError(e)| e)?;
    if duration < 0 {
        return Err("duration must be positive".to_string());
    }
    Ok(Duration::from_nanos(duration as u64))
}

pub fn get_styles() -> Styles {
    // clap v3 styles, see
    // https://stackoverflow.com/questions/74068168/clap-rs-not-printing-colors-during-help
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Yellow.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

pub async fn ctrlc_trap(run_app: impl Future<Output = anyhow::Result<()>>) {
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

    // run app, abort on ctrl-c
    tokio::select! {
        _ = abort_recv.recv() => {}
        result = run_app => {
            if let Err(e) = result {
                log::error!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

pub fn setup_logging(verbose: u8, quiet: u8) {
    // set verbosity level, default is info
    let filter_layer = filter::EnvFilter::builder()
        .with_default_directive((match (verbose as i32).saturating_sub(quiet as i32) {
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
}

pub async fn init(
    args: impl Iterator<Item = String>,
    run_app: impl AsyncFnOnce(BaseArgs) -> anyhow::Result<()>,
) {
    let args = BaseArgs::parse_from(args);
    setup_logging(args.verbose, args.quiet);

    if args.version {
        crate::version::print_version();
        return;
    }

    ctrlc_trap(async move { run_app(args).await }).await;
}

async fn translate_data(data: &[u8], translate: &str) -> anyhow::Result<Vec<u8>> {
    let (arg0, args) = {
        let mut args = shlex::split(translate).context("invalid translate command")?;
        let arg0 = if args.is_empty() {
            String::new()
        } else {
            args.remove(0)
        };
        (arg0, args)
    };

    let mut process = tokio::process::Command::new(&arg0)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = process.stdin.take().context("failed to get stdin")?;
    stdin.write_all(data).await?;
    drop(stdin);

    let result = process.wait_with_output().await?;
    for line in String::from_utf8_lossy(&result.stderr).lines() {
        log::warn!("translate stderr: {}", line);
    }
    if !result.status.success() {
        anyhow::bail!("translate failed with exit code {}", result.status);
    }
    Ok(result.stdout)
}

async fn print_data(idx: u32, frame: &Frame, translate: &Option<String>) -> anyhow::Result<()> {
    std::io::stdout().write_all(
        format!("[#{idx}] Received on \"{}\" ({} bytes)\n", frame.topic, frame.payload.len()).as_bytes()
    )?;

    if !frame.headers.is_empty() {
        for (key, values) in frame.headers.iter() {
            for value in values {
                std::io::stdout().write_all(format!("{}: {}\n", key, value).as_bytes())?;
            }
        }
        std::io::stdout().write_all(b"\n")?;
    }

    let mut data = Cow::Borrowed(&frame.payload);
    if let Some(translate) = translate {
        data = Cow::Owned(translate_data(&data, translate).await?);
    }

    // make sure that terminal output is valid utf-8 (otherwise terminal may crash),
    // user should use --raw to override this
    let data = String::from_utf8_lossy(&data);
    std::io::stdout().write_all(data.as_bytes())?;
    if !data.ends_with(['\n', '\r']) {
        std::io::stdout().write_all(b"\n")?;
    }
    std::io::stdout().write_all(b"\n\n")?;
    std::io::stdout().flush()?;
    Ok(())
}

pub async fn run<Q: MessageQueue>(args: impl Iterator<Item = String>) {
    init(args, |args: BaseArgs| async move {
        match args.command {
            Some(Commands::Publish { channel, data, header, count, sleep }) => {
                let mq = Q::connect(if args.url.is_empty() { None } else { Some(&args.url) }).await?;
                for n in 0..count {
                    if n > 0 {
                        tokio::time::sleep(sleep).await;
                    }
                    mq.publish(&channel, &header, data.as_bytes()).await?;
                    log::info!("published {} bytes to \"{}\"", data.len(), channel);
                }
            }
            Some(Commands::Subscribe { channel, translate }) => {
                let mut idx = 0;
                let mq = Q::connect(if args.url.is_empty() { None } else { Some(&args.url) }).await?;
                let stream = mq.subscribe(&channel);
                let mut stream = pin!(stream);
                while let Some(msg) = stream.next().await {
                    let frame = msg?;
                    idx += 1;
                    print_data(idx, &frame, &translate).await?;
                }
            }
            Some(Commands::Request { channel, data, header, count, translate }) => {
                let mq = Q::connect(if args.url.is_empty() { None } else { Some(&args.url) }).await?;
                let mut idx = 0;
                for _ in 0..count {
                    log::info!("sending request to \"{}\"", channel);
                    let time = std::time::Instant::now();
                    let frame = mq.request(&channel, &header, data.as_bytes()).await?;
                    log::info!("received with rtt {:?}", time.elapsed());
                    idx += 1;
                    print_data(idx, &frame, &translate).await?;
                }
            }
            None => {
                use clap::CommandFactory;
                let _ = BaseArgs::command().print_help();
            }
        }

        Ok(())
    }).await;
}
