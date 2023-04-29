mod config;
use config::*;

mod contract;
use contract::*;

use std::io::ErrorKind;

use anyhow::anyhow;
use clap::{self, Args, Command, FromArgMatches};
use tokio::net::UnixStream;
use tracing::*;
use tracing_subscriber::FmtSubscriber;

/// Simple program to greet a person
#[derive(Args, Debug)]
#[command(author, version, about, long_about = None)]
struct CtlArgs {
    #[arg(short, long, help = "Path to netwaysted.toml file.")]
    config_file: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.) will be written to stdout.
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let subcommands = Command::new("netwaystectl")
        .subcommand(Command::new("status"))
        .subcommand(Command::new("exit")); // TODO: define other command types
    let matches = CtlArgs::augment_args(subcommands).get_matches();
    let ctl_args = CtlArgs::from_arg_matches(&matches)?;

    if let Some((subcmd, _matches)) = matches.subcommand() {
        debug!("Received subcommand: {}", subcmd); // TODO: send subcommand in message to server rather than merely logging it
    }

    let toml_config = config_from_file(&ctl_args.config_file)?;

    let sock = connect_to_control_socket(&toml_config.control).await?;
    sock.writable().await?;

    let mut return_status = Ok(());
    let control_message = b"Sky Haussmann";
    match sock.try_write(control_message) {
        Ok(n) => {
            if n != control_message.len() {
                warn!(
                    "Failed to write all bytes to stream. Wrote {} of {}",
                    n,
                    control_message.len()
                );
            }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
        Err(e) => {
            return_status = Err(e.into());
        }
    }

    sock.readable().await?;

    // Try to read data, this may still fail with `WouldBlock` if the readiness event is a false positive.
    let mut msg = vec![0; MAX_CONTROL_MESSAGE_LEN];
    match sock.try_read(&mut msg) {
        Ok(n) => {
            msg.truncate(n);

            if let Ok(msg_as_str) = String::from_utf8(msg) {
                info!("Netwayste server responded with '{}'", msg_as_str);
            } else {
                error!("Netwayste server response is not valid UTF-8");
            }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            warn!("Dropping read, would block");
        }
        Err(e) => {
            error!("Failed to read message");
            return_status = Err(e.into());
        }
    }

    return return_status;
}

async fn connect_to_control_socket(ctrl_cfg: &ControlConfig) -> anyhow::Result<UnixStream> {
    info!("Connecting to socket...");
    UnixStream::connect(&ctrl_cfg.socket_path).await.map_err(|e| anyhow!(e))
}
