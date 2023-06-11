use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use crate::{
    app::server::registry::{self, REGISTER_INTERVAL},
    filter::{FilterCmd, FilterCmdSend, FilterNotifyRecv, FilterRspRecv},
    settings::APP_CHANNEL_LEN,
    Endpoint,
};

use futures::Future;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    watch,
};

use super::registry::RegistryParams;

#[derive(Copy, Clone, Debug)]
enum Phase {
    Running,
    ShutdownInProgress,
    ShutdownComplete,
}

pub struct AppServer {
    filter_cmd_tx:    FilterCmdSend,
    filter_rsp_rx:    Option<FilterRspRecv>,
    filter_notice_rx: Option<FilterNotifyRecv>,
    phase_watch_tx:   Option<watch::Sender<Phase>>, // Temp. holding place. This is only Some(...) between new() and run() calls
    phase_watch_rx:   watch::Receiver<Phase>,
    registry_params:  Option<RegistryParams>, // If None, then not a public server
}

impl AppServer {
    pub fn new(
        filter_cmd_tx: FilterCmdSend,
        filter_rsp_rx: FilterRspRecv,
        filter_notice_rx: FilterNotifyRecv,
        registry_params: Option<RegistryParams>,
    ) -> AppServer {
        let (phase_watch_tx, phase_watch_rx) = watch::channel(Phase::Running);

        AppServer {
            filter_cmd_tx,
            filter_rsp_rx: Some(filter_rsp_rx),
            filter_notice_rx: Some(filter_notice_rx),
            phase_watch_tx: Some(phase_watch_tx),
            phase_watch_rx,
            registry_params,
        }
    }

    pub async fn run(&mut self) {
        let filter_cmd_tx = self.filter_cmd_tx.clone();
        let filter_rsp_rx = self.filter_rsp_rx.take().expect("run() is single-use");
        let filter_notice_rx = self.filter_notice_rx.take().expect("run() is single-use");
        tokio::pin!(filter_cmd_tx);
        tokio::pin!(filter_rsp_rx);
        tokio::pin!(filter_notice_rx);

        let phase_watch_tx = self.phase_watch_tx.take().expect("run() is single-use");

        let mut register_interval_stream = tokio::time::interval(REGISTER_INTERVAL);

        loop {
            tokio::select! {
                response = filter_rsp_rx.recv() => {
                    if let Some(filter_rsp)  = response {
                        trace!("[A<-F,R] {:?}", filter_rsp);
                        //TODO: handle filter response
                    } else {
                        info!("filter response channel is closed; shutting down");
                        break;
                    }
                }
                notice = filter_notice_rx.recv() => {
                    if let Some(filter_notice) = notice {
                        trace!("[A<-F,N] {:?}", filter_notice);
                        //TODO: handle filter notice
                    } else {
                        info!("filter notice channel is closed; shutting down");
                        break;
                    }
                }
                _instant = register_interval_stream.tick() => {
                    if let Some(ref registry_params) = self.registry_params {
                        if registry::try_register(registry_params.clone()).await {
                            //let registry_address = format!("{}:{}", "157.230.134.224", 2016);
                            let registry_address = format!(
                                "{}:{}",
                                registry_params.registry_url.trim_end_matches("/addServer"),
                                2016
                            );

                            let registry_addresses: Vec<_> = registry_address
                                .to_socket_addrs()
                                .expect("Unable to parse a SocketAddress from the registry public address")
                                .collect();

                            if registry_address.len() == 0 {
                                error!(
                                    "Failed to resolve {} to an IP address",
                                    registry_params.registry_url
                                );
                                break;
                            }

                            // Result ignored because AddPingEndpoints will silently continue
                            let _ = filter_cmd_tx
                                .send(FilterCmd::AddPingEndpoints {
                                    endpoints: vec![
                                        // Pick the very first address we find since to_socket_addrs returns a list
                                        Endpoint {
                                            0: registry_addresses[0],
                                        },
                                    ],
                                })
                                .await;
                        }
                    }
                }
            }
        }
        let _ = phase_watch_tx.send(Phase::ShutdownComplete);
    }

    pub fn get_shutdown_watcher(&mut self) -> impl Future<Output = ()> + 'static {
        let mut phase_watch_rx = self.phase_watch_rx.clone();
        let filter_cmd_tx = self.filter_cmd_tx.clone();
        async move {
            loop {
                let phase = *phase_watch_rx.borrow();
                match phase {
                    Phase::ShutdownComplete => {
                        break;
                    }
                    _ => {}
                }
                if phase_watch_rx.changed().await.is_err() {
                    // channel closed
                    trace!("[A] phase watch channel was dropped");
                    break;
                }
            }
            // Also shutdown the layer below
            let _ = filter_cmd_tx.send(FilterCmd::Shutdown { graceful: true }).await;
        }
    }
}
