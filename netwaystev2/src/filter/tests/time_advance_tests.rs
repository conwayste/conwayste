use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{self, Instant};
use crate::filter::{Filter, FilterMode};
use crate::transport::{TransportCmd, TransportRsp, TransportNotice};
use crate::settings::TRANSPORT_CHANNEL_LEN;

#[tokio::test]
async fn time_advancing_works() {
    time::pause();
    let start = Instant::now();
    time::advance(Duration::from_secs(5)).await;
    let end = Instant::now();
    assert_eq!(end - start, Duration::from_secs(5));
}

#[tokio::test]
async fn basic_server_filter_flow() {
    time::pause();
    /*
    XXX
    pub type TransportCmdSend = Sender<TransportCmd>;
    pub type TransportRspRecv = Receiver<TransportRsp>;
    pub type TransportNotifyRecv = Receiver<TransportNotice>;
    */
    // Mock transport channels
    let (transport_cmd_tx, transport_cmd_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_rsp_tx, transport_rsp_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_notice_tx, transport_notice_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);

    let mut filter = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Server,
    );

    // Start the filter's task in the background
    tokio::spawn(async move { filter.run().await });

    //XXX send a mock transport notification

    time::advance(Duration::from_secs(5)).await;
    // TODO PR_GATE: send commands to filter

    //XXX test for expected transport command(s) sent
}

//XXX basic_client_filter_flow
