mod config;
use config::*;

mod contract;
use contract::*;

use std::path::Path;

use anyhow::anyhow;
use clap::{self, Parser};
use tokio::net::UnixListener;
use tracing::*;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Path to netwaysted.toml file.")]
    config_file: String,

    #[arg(long, help = "Dump configuration and then exit with success return code.")]
    dump_config: bool,
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
    if args.dump_config {
        println!("{:#?}", toml_config);
        return Ok(());
    }

    let listener = open_control_socket(&toml_config.control)?;

    Ok(())
}

fn open_control_socket(ctrl_cfg: &ControlConfig) -> anyhow::Result<ListenerWrapper> {
    info!("Opening socket...");
    UnixListener::bind(&ctrl_cfg.socket_path)
        .map(|l| ListenerWrapper(l))
        .map_err(|e| anyhow!(e))
}

fn cleanup_socket(path: &Path) {
    let _ = remove_file(path);
}

/// Wrapper to ensure the listener socket is cleaned up when server daemon exits.
struct ListenerWrapper(UnixListener);

impl Drop for ListenerWrapper {
    fn drop(&mut self) {
        let socket_addr = self.0.local_addr().expect("local address");
        let path = socket_addr
            .as_pathname()
            .expect("valid pathname in UnixListener socket");
        cleanup_socket(&path);
    }
}
