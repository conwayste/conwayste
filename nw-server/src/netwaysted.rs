mod config;
use config::*;

use std::path::Path;
use std::fs::remove_file;

use anyhow::anyhow;
use clap::{self, Parser};
use tokio::net::{UnixStream, UnixListener};
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

    let listener = open_control_socket(&toml_config.control)?;

    Ok(())
}

fn open_control_socket(ctrl_cfg: &ControlConfig) -> anyhow::Result<ListenerWrapper> {
    println!("Opening socket...");
    UnixListener::bind(&ctrl_cfg.socket_path).map(|l| ListenerWrapper(l)).map_err(|e| anyhow!(e))
}

fn cleanup_socket(path: &Path) {
    let _ = remove_file(path);
}

struct ListenerWrapper(UnixListener);

impl Drop for ListenerWrapper {
    fn drop(&mut self) {
        let socket_addr = self.0.local_addr().expect("pathname in listener");
        let path = socket_addr.as_pathname().expect("valid pathname in UnixListener socket");
        cleanup_socket(&path);
    }
}
