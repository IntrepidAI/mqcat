use clap::error::ErrorKind;
use clap::CommandFactory;
use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;

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
}

#[derive(Parser, Debug)]
#[command(bin_name = "mqcat")]
#[command(disable_help_subcommand = true)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(ignore_errors = true)]
#[command(styles = get_styles())]
#[command(after_help = "Run `mqcat zenoh --help` for further info.")]
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
    #[command(about = "centrifuge (centrifugal.dev) client, json encoding\ndefault: cfj+ws://localhost:8000/connection/websocket\n")]
    Cfj,
    #[command(about = "centrifuge (centrifugal.dev) client, protobuf encoding\ndefault: cfp+ws://localhost:8000/connection/websocket?format=protobuf\n")]
    Cfp,
    #[command(about = "nats (nats.io) client\ndefault: nats://localhost:4222\n")]
    Nats,
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

    if let Ok(args) = HelpVersionOnly::try_parse_from(args.iter()) {
        if args.help {
            BaseArgs::command().print_help().unwrap();
            return;
        }
        if args.version {
            mqcat_commons::version::print_version();
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
    let (transport, url) = mqcat_commons::url_transport::parse(url);
    let transport = transport.to_owned();
    let url = url.to_owned();
    args[transport_idx] = url;

    match &*transport {
        "cfj" => {
            mqcat_centrifuge::run(args.into_iter()).await;
        }
        "cfp" => {
            mqcat_centrifuge::run(args.into_iter()).await;
        }
        "nats" => {
            mqcat_nats::run(args.into_iter()).await;
        }
        "zenoh" => {
            mqcat_zenoh::run(args.into_iter()).await;
        }
        _ => {
            BaseArgs::command().error(
                ErrorKind::InvalidValue,
                format!("invalid transport: '{}', available transports are: 'cfj', 'cfp', 'nats', 'zenoh'", transport),
            ).exit();
        }
    }
}
