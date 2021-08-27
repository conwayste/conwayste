use crate::common::Endpoint;
use crate::filter::{Filter, FilterCmd, FilterMode};
use crate::protocol::{Packet, RequestAction};
use crate::settings::TRANSPORT_CHANNEL_LEN;
use crate::transport::{TransportCmd, TransportNotice, TransportRsp};
use std::net::ToSocketAddrs;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{self, timeout_at, Instant};

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
    let (transport_cmd_tx, mut transport_cmd_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_rsp_tx, transport_rsp_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_notice_tx, transport_notice_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);

    let (mut filter, filter_cmd_tx, filter_rsp_rx, filter_notify_rx) = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Server,
    );

    let filter_shutdown_watcher = filter.get_shutdown_watcher(); // No await; get the future

    // Start the filter's task in the background
    tokio::spawn(async move { filter.run().await });

    // Send a mock transport notification
    let endpoint = Endpoint(("1.2.3.4", 5678).to_socket_addrs().unwrap().next().unwrap());
    let packet = Packet::Request {
        sequence:     1,
        response_ack: None,
        cookie:       None,
        action:       RequestAction::Connect {
            name:           "Sheeana".to_owned(),
            client_version: "0.3.2".to_owned(),
        },
    };
    transport_notice_tx
        .send(TransportNotice::PacketDelivery { endpoint, packet })
        .await
        .unwrap();

    filter_cmd_tx
        .send(FilterCmd::Shutdown { graceful: false })
        .await
        .unwrap();

    let expiration = Instant::now() + Duration::from_secs(3);

    time::advance(Duration::from_secs(5)).await;

    let timeout_result = timeout_at(expiration, transport_cmd_rx.recv()).await;
    let transport_cmd = timeout_result.expect("we should not have timed out getting a transport command");

    let transport_cmd = transport_cmd.expect("channel should not have been closed");
    println!("transport_cmd: {:?}", transport_cmd);

    //XXX test for expected transport command(s) sent

    filter_shutdown_watcher.await;
}

//XXX basic_client_filter_flow
