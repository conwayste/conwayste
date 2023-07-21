use std::time::Duration;

use crate::{
    app::server::registry::{self, REGISTER_INTERVAL},
    app::server::room::*,
    filter::{FilterCmd, FilterCmdSend, FilterNotifyRecv, FilterRspRecv, FilterNotice},
    protocol::RequestAction::{*, self},
    settings::APP_CHANNEL_LEN,
};

use anyhow::{anyhow, Result};
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
    rooms:            RoomBlock,
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
            rooms: RoomBlock::new(),
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
                        if let Err(e) = self.handle_filter_notice(filter_notice) {
                            error!("[A] filter notice processing failed: {}", e);
                        }
                    } else {
                        info!("filter notice channel is closed; shutting down");
                        break;
                    }
                }
                _instant = register_interval_stream.tick() => {
                    if let Some(ref registry_params) = self.registry_params {
                        registry::try_register(registry_params.clone()).await;
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

    fn handle_filter_notice(&mut self, notice: FilterNotice) -> Result<()> {
        Ok(())
    }
}
