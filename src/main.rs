use clap::error::ErrorKind;
use clap::CommandFactory;
use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;

#[cfg(feature = "self-upgrade")]
mod upgrade;

#[derive(Parser, Debug)]
#[command(bin_name = "mqcat")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(ignore_errors = true)]
#[command(styles = get_styles())]
pub struct HelpVersionOnly {
    #[arg(global = true, short, long)]
    /// print this help message
    help: bool,
    #[arg(global = true, short = 'V', long)]
    /// print version and build info
    version: bool,
    #[cfg(feature = "self-upgrade")]
    #[arg(long)]
    /// upgrade executable to the latest version
    upgrade: bool,
}

#[derive(Parser, Debug)]
#[command(bin_name = "mqcat")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(styles = get_styles())]
pub struct BaseArgs {
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
    #[command(subcommand)]
    /// transport name or server url address
    url: Transport,
}

#[derive(Parser, Debug)]
#[command(subcommand_help_heading = "Transports", subcommand_value_name = "URL")]
enum Transport {
    #[cfg(feature = "backend-centrifuge")]
    #[command(about = "centrifuge (centrifugal.dev) client, json encoding\ndefault: cfj+ws://localhost:8000/connection/websocket")]
    Cfj,
    #[cfg(feature = "backend-centrifuge")]
    #[command(about = "centrifuge (centrifugal.dev) client, protobuf encoding\ndefault: cfp+ws://localhost:8000/connection/websocket?format=protobuf")]
    Cfp,
    #[cfg(feature = "backend-nats")]
    #[command(about = "nats (nats.io) client\ndefault: nats://localhost:4222")]
    Nats,
    #[cfg(feature = "backend-zenoh")]
    #[command(about = "zenoh (zenoh.io) client\ndefault: zenoh+tcp://localhost:7558")]
    Zenoh,
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

#[tokio::main]
async fn main() {
    let mut args = std::env::args().collect::<Vec<String>>();

    if let Ok(basic_args) = HelpVersionOnly::try_parse_from(args.iter()) {
        #[cfg(feature = "self-upgrade")]
        if basic_args.upgrade {
            upgrade::run_app(args).await;
            return;
        }
        if basic_args.help {
            BaseArgs::command().print_help().unwrap();
            return;
        }
        if basic_args.version {
            mqcat::version::print_version();
            return;
        }
    }

    let mut transport_idx = None;
    for (i, arg) in args.iter().enumerate() {
        if i != 0 && !arg.starts_with('-') {
            transport_idx = Some(i);
            break;
        }
    }

    let Some(transport_idx) = transport_idx else {
        BaseArgs::command().print_help().unwrap();
        return;
    };

    let url = args.get(transport_idx).unwrap();
    let (transport, url) = mqcat::url_transport::parse(url);
    let transport = transport.to_owned();
    let url = url.to_owned();
    args[transport_idx] = url;

    match &*transport {
        #[cfg(feature = "backend-centrifuge")]
        "cfj" => {
            mqcat::backends::centrifuge::run::<true>(args.into_iter()).await;
        }
        #[cfg(feature = "backend-centrifuge")]
        "cfp" => {
            mqcat::backends::centrifuge::run::<false>(args.into_iter()).await;
        }
        #[cfg(feature = "backend-nats")]
        "nats" => {
            mqcat::backends::nats::run(args.into_iter()).await;
        }
        #[cfg(feature = "backend-zenoh")]
        "zenoh" => {
            mqcat::backends::zenoh::run(args.into_iter()).await;
        }
        _ => {
            let transports: Vec<&str> = vec![
                #[cfg(feature = "backend-centrifuge")]
                "cfj",
                #[cfg(feature = "backend-centrifuge")]
                "cfp",
                #[cfg(feature = "backend-nats")]
                "nats",
                #[cfg(feature = "backend-zenoh")]
                "zenoh",
            ];

            BaseArgs::command().error(
                ErrorKind::InvalidValue,
                format!(
                    "invalid transport: '{}', available transports are: {}",
                    transport,
                    transports.iter().map(|t| format!("'{}'", t)).collect::<Vec<String>>().join(", "),
                ),
            ).exit();
        }
    }
}
