mod config;
use config::*;

mod contract;
use contract::*;

mod room;

use std::path::Path;
use std::time::Duration;
use std::{fs::remove_file, io::ErrorKind};

use anyhow::anyhow;
use clap::{self, Parser};
use netwaystev2::{app::server::*, common::*, filter::*, transport::*};
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};
use tracing::*;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct DaemonArgs {
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

    let args = DaemonArgs::parse();

    let toml_config = config_from_file(&args.config_file)?;
    if args.dump_config {
        let toml_config_str = format!("{:#?}", toml_config);
        for line in toml_config_str.lines() {
            info!("{}", line);
        }
        return Ok(());
    }

    let listener = open_control_socket(&toml_config.control)?;
    let layers = spin_up_layers(&toml_config)
        .await
        .expect("Failed to create netwayste layers");
    let exit_status = run(&listener, layers).await;

    info!("[D] Server exiting...");

    return exit_status;
}

async fn spin_up_layers(cfg: &Config) -> anyhow::Result<(Transport, Filter, AppServer)> {
    // Create the lowest (Transport) layer, returning the layer itself plus three channel halves
    // (one outgoing and two incoming) for communicating with it.
    let (transport, transport_cmd_tx, transport_rsp_rx, transport_notice_rx) =
        Transport::new(None, Some(cfg.server.bind_port), TransportMode::Server).await?;

    // Create the three channels for communication between filter and application
    // Join the middle filter layer to the transport
    let (filter, filter_cmd_tx, filter_rsp_rx, filter_notice_rx) = Filter::new(
        transport_cmd_tx.clone(),
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Client,
    );

    let registry_params = cfg.registry.as_ref().map(|registry| RegistryParams {
        public_addr:  format!("{}:{}", registry.public_host, registry.public_port),
        registry_url: registry.url.clone(),
    });

    if registry_params.is_some() {
        info!("[D] This server is registering itself with the registrar");
    } else {
        info!("[D] This server is private");
    }

    // Join the top application server layer to the filter
    let (app_server, _unigen_cmd_rx, _unigen_rsp_tx, _unigen_notice_tx) =
        AppServer::new(filter_cmd_tx.clone(), filter_rsp_rx, filter_notice_rx, registry_params);

    trace!(
        "[D] Networking layers created with local address of {}",
        transport.local_addr()
    );

    Ok((transport, filter, app_server))
}

async fn run(
    listener: &ListenerWrapper,
    (mut transport, mut filter, mut app): (Transport, Filter, AppServer),
) -> anyhow::Result<()> {
    let mut server_status = Ok(());

    // Capture SIGTERM, and SIGINT to clean up the socket gracefully now that it's open.
    // SIGKILL (SignalKind::kill()) is not specified as there is no opportunity to clean up any system resources
    let mut sigint = signal(SignalKind::interrupt()).expect("Could not capture SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("Could not capture SIGTERM");

    // TODO: Watch all layers for early termination
    let _transport_shutdown_watcher = transport.get_shutdown_watcher();
    let _filter_shutdown_watcher = filter.get_shutdown_watcher();
    let _app_shutdown_watcher = app.get_shutdown_watcher();

    // Start the transport's task in the background
    tokio::spawn(async move { transport.run().await });

    // Start the filter's task in the background
    tokio::spawn(async move { filter.run().await });

    // Start the app's task in the background
    tokio::spawn(async move { app.run().await });

    'main: loop {
        tokio::select! {
            _ = sigint.recv() => {
                info!("[D] SIGINT received, cleaning up");
                drop(listener);
                break 'main;
            },

            _ = sigterm.recv() => {
                info!("[D] SIGTERM received, cleaning up");
                drop(listener);
                break 'main;
            }

            new_message = listener.0.accept() => {
                match new_message {
                    Ok((stream, _addr)) => {
                        // Wait for the socket to be readable
                        stream.readable().await?;
                        info!("[D] Control message received");

                        let mut response = String::new();

                        // Try to read data
                        // This can fail with `WouldBlock` if the readiness event is a false positive
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
                                warn!("[D] Dropping read, would block");
                                continue;
                            }
                            Err(e) => {
                                error!("[D] Failed to read message");
                                server_status = Err(e.into());
                            }
                        }

                        if let Err(_) = server_status {
                            // XXX: Terminate early if the read fails while server is still under development
                            break 'main;
                        }

                        // Try to write data
                        // This can fail with `WouldBlock` if the readiness event is a false positive
                        stream.writable().await?;
                        match stream.try_write(response.as_bytes()) {
                            Ok(n) => {
                                if n != response.len() {
                                    warn!("[D] Failed to write all bytes to stream. Wrote {} of {}", n, response.len());
                                }
                            }
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                warn!("[D] Dropping write, would block");
                                continue;
                            }
                            Err(e) => {
                                error!("[D] Failed to respond");
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
    info!("[D] Opening socket...");
    UnixListener::bind(&ctrl_cfg.socket_path)
        .map(|l| ListenerWrapper(l))
        .map_err(|e| {
            anyhow!(format!(
                "Could not bind to socket. Check if '{}' exists and remove. Error: {}",
                ctrl_cfg.socket_path, e
            ))
        })
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
