mod config;
use config::*;

mod contract;
use contract::*;
use netwaystev2::app::server::interface::{AppCmd, AppCmdSend, AppRsp, AppRspRecv};

use std::path::Path;
use std::{fs::remove_file, io::ErrorKind};

use anyhow::{anyhow, bail};
use bincode;
use clap::{self, Parser};
use netwaystev2::{app::server::*, filter::*, transport::*};
use tabled::Table;
use tokio::net::{UnixListener, UnixStream};
use tokio::signal::unix::{signal, SignalKind};
use tracing::*;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

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
    // This enables a compatibility layer between tracing 'Event's and log 'Record's.
    // Disables logging for crates we typically do not care about.
    LogTracer::builder()
        .ignore_all([
            // Usage is described below for every exclusion
            "want",    // signaling
            "rustls",  // registrar
            "reqwest", // registrar
            "mio",     // async
            "hyper",   // registrar
        ])
        .init()?;

    let subscriber = FmtSubscriber::builder()
        // Use a log level of TRACE for the daemon and netwayste crate, but only enable errors for hyper.
        // This reduces the logging 'noise' to just the things we care about.
        .with_env_filter(EnvFilter::new("hyper=error,netwaysted=trace,netwaystev2=trace"))
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
    let (layers, app_cmd_tx, app_rsp_rx) = spin_up_layers(&toml_config)
        .await
        .expect("Failed to create netwayste layers");
    let exit_status = run(&listener, layers, app_cmd_tx, app_rsp_rx).await;

    info!("Server exiting...");

    return exit_status;
}

async fn spin_up_layers(cfg: &Config) -> anyhow::Result<((Transport, Filter, AppServer), AppCmdSend, AppRspRecv)> {
    // Create the lowest (Transport) layer, returning the layer itself plus three channel halves
    // (one outgoing and two incoming) for communicating with it.
    let (transport, transport_cmd_tx, transport_rsp_rx, transport_notice_rx) =
        Transport::new(None, Some(cfg.server.bind_port), TransportMode::Server).await?;

    let mut server_status = ServerStatus::default();
    server_status.server_name = cfg.server.name.to_owned();

    // Create the three channels for communication between filter and application
    // Join the middle filter layer to the transport
    let (filter, filter_cmd_tx, filter_rsp_rx, filter_notice_rx) = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Server(server_status),
    );

    let registry_params = cfg.registry.as_ref().map(|registry| RegistryParams {
        public_addr:  format!("{}:{}", registry.public_host, registry.public_port),
        registry_url: registry.url.clone(),
    });

    if registry_params.is_some() {
        info!("This server is registering itself with the registrar");
        debug!("{:#?}", registry_params.as_ref().unwrap());
    } else {
        info!("This server is private");
    }

    // Join the top application server layer to the filter
    let (app_server, app_cmd_tx, app_rsp_rx) =
        AppServer::new(filter_cmd_tx, filter_rsp_rx, filter_notice_rx, registry_params);

    trace!(
        "Networking layers created with local address of {}",
        transport.local_addr()
    );

    Ok(((transport, filter, app_server), app_cmd_tx, app_rsp_rx))
}

async fn run(
    listener: &ListenerWrapper,
    (mut transport, mut filter, mut app): (Transport, Filter, AppServer),
    app_cmd_tx: AppCmdSend,
    mut app_rsp_rx: AppRspRecv,
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
                info!("SIGINT received, cleaning up");
                drop(listener);
                break 'main;
            },

            _ = sigterm.recv() => {
                info!("SIGTERM received, cleaning up");
                drop(listener);
                break 'main;
            }

            new_connection = listener.0.accept() => {
                match new_connection {
                    Ok((stream, _addr)) => {
                        // Wait for the socket to be readable
                        stream.readable().await?;
                        info!("Control message received");
                        match handle_new_ctl_message(&stream).await {
                            Ok(command) => {
                                app_cmd_tx.try_send(command)?;
                                if let Some(app_rsp) = app_rsp_rx.recv().await {
                                    send_ctl_reply(&stream, DaemonResponse::from(app_rsp)).await?;
                                }
                            }
                            Err(e) => {
                                error!("Control message processing failed: {}", e);
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

async fn handle_new_ctl_message(stream: &UnixStream) -> anyhow::Result<AppCmd> {
    // Try to read data
    // This can fail with `WouldBlock` if the readiness event is a false positive
    let mut msg = vec![0; MAX_CONTROL_MESSAGE_LEN];
    match stream.try_read(&mut msg) {
        Ok(n) => {
            msg.truncate(n);
            let msg_utf8 = String::from_utf8(msg).unwrap_or("control message is not valid utf8".to_string());
            info!("{}", msg_utf8);

            return AppCmd::try_from(msg_utf8);
        }
        Err(e) if e.kind() == ErrorKind::WouldBlock => {
            warn!("Dropping read, would block");
            bail!(e);
        }
        Err(e) => {
            error!("Failed to read control message");
            bail!(e);
        }
    }
}

async fn send_ctl_reply(stream: &UnixStream, response: DaemonResponse) -> anyhow::Result<()> {
    // Try to write data
    // This can fail with `WouldBlock` if the readiness event is a false positive
    stream.writable().await?;
    let serialized_response = bincode::serialize(&response)?;
    match stream.try_write(&serialized_response) {
        Ok(n) => {
            if n != serialized_response.len() {
                warn!(
                    "Failed to write all bytes to stream. Wrote {} of {}",
                    n,
                    serialized_response.len()
                );
            }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            warn!("Dropping write, would block");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to respond to control request");
            return Err(e.into());
        }
    }

    Ok(())
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

impl From<AppRsp> for DaemonResponse {
    fn from(response: AppRsp) -> Self {
        match response {
            AppRsp::RoomsStatuses(statuses) => DaemonResponse {
                message: Table::new(statuses).to_string(),
                status:  DaemonStatus::Success,
            },
        }
    }
}
