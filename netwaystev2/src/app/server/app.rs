use std::time::Duration;

use crate::{
    app::server::interface::{UniGenCmd, UniGenNotice, UniGenRsp},
    app::server::registry::{self, REGISTER_INTERVAL},
    filter::{FilterCmd, FilterCmdSend, FilterNotifyRecv, FilterRspRecv},
    settings::APP_CHANNEL_LEN,
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

type UniGenCmdSend = Sender<UniGenCmd>;
pub type UniGenCmdRecv = Receiver<UniGenCmd>;
pub type UniGenRspSend = Sender<UniGenRsp>;
type UniGenRspRecv = Receiver<UniGenRsp>;
pub type UniGenNotifySend = Sender<UniGenNotice>;
type UniGenNotifyRecv = Receiver<UniGenNotice>;

pub type AppServerInit = (AppServer, UniGenCmdRecv, UniGenRspSend, UniGenNotifySend);

pub struct AppServer {
    filter_cmd_tx:    FilterCmdSend,
    filter_rsp_rx:    Option<FilterRspRecv>,
    filter_notice_rx: Option<FilterNotifyRecv>,
    unigen_cmd_tx:    UniGenCmdSend,
    unigen_rsp_rx:    Option<UniGenRspRecv>,
    unigen_notice_rx: Option<UniGenNotifyRecv>,
    phase_watch_tx:   Option<watch::Sender<Phase>>, // Temp. holding place. This is only Some(...) between new() and run() calls
    phase_watch_rx:   watch::Receiver<Phase>,
    registry_params:  RegistryParams,
}

impl AppServer {
    pub fn new(
        filter_cmd_tx: FilterCmdSend,
        filter_rsp_rx: FilterRspRecv,
        filter_notice_rx: FilterNotifyRecv,
        registry_params: RegistryParams,
    ) -> AppServerInit {
        let (unigen_cmd_tx, unigen_cmd_rx): (UniGenCmdSend, UniGenCmdRecv) = mpsc::channel(APP_CHANNEL_LEN);
        let (unigen_rsp_tx, unigen_rsp_rx): (UniGenRspSend, UniGenRspRecv) = mpsc::channel(APP_CHANNEL_LEN);
        let (unigen_notice_tx, unigen_notice_rx): (UniGenNotifySend, UniGenNotifyRecv) = mpsc::channel(APP_CHANNEL_LEN);

        let (phase_watch_tx, phase_watch_rx) = watch::channel(Phase::Running);

        (
            AppServer {
                filter_cmd_tx,
                filter_rsp_rx: Some(filter_rsp_rx),
                filter_notice_rx: Some(filter_notice_rx),
                unigen_cmd_tx: unigen_cmd_tx,
                unigen_rsp_rx: Some(unigen_rsp_rx),
                unigen_notice_rx: Some(unigen_notice_rx),
                phase_watch_tx: Some(phase_watch_tx),
                phase_watch_rx,
                registry_params,
            },
            unigen_cmd_rx,
            unigen_rsp_tx,
            unigen_notice_tx,
        )
    }

    pub async fn run(&mut self) {
        let filter_cmd_tx = self.filter_cmd_tx.clone();
        let filter_rsp_rx = self.filter_rsp_rx.take().unwrap();
        let filter_notice_rx = self.filter_notice_rx.take().unwrap();
        tokio::pin!(filter_cmd_tx);
        tokio::pin!(filter_rsp_rx);
        tokio::pin!(filter_notice_rx);

        let unigen_cmd_tx = self.unigen_cmd_tx.clone();
        let unigen_rsp_rx = self.unigen_rsp_rx.take().unwrap();
        let unigen_notice_rx = self.unigen_notice_rx.take().unwrap();
        tokio::pin!(unigen_cmd_tx);
        tokio::pin!(unigen_rsp_rx);
        tokio::pin!(unigen_notice_rx);

        let mut register_interval_stream = tokio::time::interval(REGISTER_INTERVAL);

        loop {
            tokio::select! {
                response = filter_rsp_rx.recv() => {
                    if let Some(response) = response {
                        trace!("[A<-F,R] {:?}", response);
                    }
                }
                notice = filter_notice_rx.recv() => {
                    if let Some(notice) = notice {
                        trace!("[A<-F,N] {:?}", notice);
                    }
                }
                response = unigen_rsp_rx.recv() => {
                    if let Some(response) = response {
                        trace!("[A<-F,UGR] {:?}", response);

                    }
                }
                notice = unigen_notice_rx.recv() => {
                    if let Some(notice) = notice {
                        trace!("[A<-F,UGN] {:?}", notice);
                    }
                }
                _instant = register_interval_stream.tick() => {
                    registry::try_register(self.registry_params.clone()).await;
                }
            }
        }
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
