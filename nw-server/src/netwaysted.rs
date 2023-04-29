mod config;
use config::*;

mod contract;
use contract::*;

use std::path::Path;
use std::{fs::remove_file, io::ErrorKind};

use anyhow::anyhow;
use clap::{self, Parser};
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};
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
    let exit_status = run(&listener).await;

    info!("Server exiting...");

    return exit_status;
}

async fn run(listener: &ListenerWrapper) -> anyhow::Result<()> {
    let mut server_status = Ok(());

    // Capture SIGTERM, and SIGINT to clean up the socket gracefully now that it's open.
    // SIGKILL (SignalKind::kill()) is not specified as there is no opportunity to clean up any system resources
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    'main: loop {
        tokio::select! {
            _ = sigint.recv() => {
                info!("SIGINT received, cleaning up");
                drop(listener);
                break 'main;
            },
            _ = sigterm.recv() => {
                info!("SIGTERM received, cleaning up");
                drop(listener);
                break 'main;
            }

            new_message = listener.0.accept() => {
                match new_message {
                Ok((stream, _addr)) => {
                    // Wait for the socket to be readable
                    stream.readable().await?;
                    info!("Control message received");

                    let mut response = String::new();

                    // Try to read data, this may still fail with `WouldBlock` if the readiness event is a false positive.
                    let mut msg = vec![0; MAX_CONTROL_MESSAGE_LEN];
                    match stream.try_read(&mut msg) {
                        Ok(n) => {
                            msg.truncate(n);

                            if let Ok(msg_as_str) = String::from_utf8(msg) {
                                response = format!("Hello {}", msg_as_str);
                            } else {
                                response = "Control command must be valid UTF-8".to_owned();
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            warn!("Dropping read, would block");
                            continue;
                        }
                        Err(e) => {
                            error!("Failed to read message");
                            server_status = Err(e.into());
                        }
                    }

                    if let Err(_) = server_status {
                        // XXX: Terminate early if the read fails while server is still under development
                        break 'main;
                    }

                    // Try to write data, this may still fail with `WouldBlock` if the readiness event is a false positive.
                    stream.writable().await?;
                    match stream.try_write(response.as_bytes()) {
                        Ok(n) => {
                            if n != response.len() {
                                warn!("Failed to write all bytes to stream. Wrote {} of {}", n, response.len());
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            warn!("Dropping write, would block");
                            continue;
                        }
                        Err(e) => {
                            error!("Failed to respond");
                            server_status = Err(e.into());
                            break 'main;
                        }
                    }
                }
                Err(e) => {
                    server_status = Err(anyhow!(format!("Connection failed: '{:?}'", e)));
                    break 'main;
                }
            }
        }
        }
    }

    return server_status;
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
