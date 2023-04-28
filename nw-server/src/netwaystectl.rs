mod config;
use config::*;

use anyhow::anyhow;
use clap::{self, Parser};
use tokio::net::UnixStream;
use toml;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config_file: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let toml_config_str = std::fs::read_to_string(args.config_file)?;
    let toml_config: Config = toml::from_str(toml_config_str.as_str())?;

    println!("{:#?}", toml_config);

    let sock = connect_to_control_socket(&toml_config.control).await?;

    Ok(())
}

async fn connect_to_control_socket(ctrl_cfg: &ControlConfig) -> anyhow::Result<UnixStream> {
    println!("Connecting to socket...");
    UnixStream::connect(&ctrl_cfg.socket_path).await.map_err(|e| anyhow!(e))
}

