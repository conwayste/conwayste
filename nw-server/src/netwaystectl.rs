mod config;
use config::*;

use anyhow::anyhow;
use clap::{self, Parser};
use tokio::net::UnixStream;
use tracing::*;
use tracing_subscriber::FmtSubscriber;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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

    let args = Args::parse();

    let toml_config = config_from_file(&args.config_file)?;

    let sock = connect_to_control_socket(&toml_config.control).await?;

    Ok(())
}

async fn connect_to_control_socket(ctrl_cfg: &ControlConfig) -> anyhow::Result<UnixStream> {
    info!("Connecting to socket...");
    UnixStream::connect(&ctrl_cfg.socket_path).await.map_err(|e| anyhow!(e))
}
