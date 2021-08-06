use futures::join;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{self, Instant, timeout_at};
use crate::common::Endpoint;
use crate::filter::{Filter, FilterMode};
use crate::transport::{TransportCmd, TransportRsp, TransportNotice};
use crate::settings::TRANSPORT_CHANNEL_LEN;
use crate::protocol::{Packet, RequestAction};

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

    let mut filter = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Server,
    );

    // Start the filter's task in the background
    tokio::spawn(async move { filter.run().await });

    // Send a mock transport notification
    let endpoint = Endpoint(("1.2.3.4", 5678).to_socket_addrs().unwrap().next().unwrap());
    let packet = Packet::Request{
        sequence: 1,
        response_ack: None,
        cookie: None,
        action: RequestAction::Connect{name: "Sheeana".to_owned(), client_version: "0.3.2".to_owned()},
    };
    transport_notice_tx.send(TransportNotice::PacketDelivery{endpoint, packet}).await.unwrap();

    let expiration = Instant::now() + Duration::from_secs(3);

    let (_, timeout_result) =
        join!(
            time::advance(Duration::from_secs(5)),
            timeout_at(expiration, transport_cmd_rx.recv()),
        );


    assert!(timeout_result.is_err()); //TODO PR_GATE wrong, we should have a command, not a timeout! Requires filter reworking

    //XXX test for expected transport command(s) sent
}

//XXX basic_client_filter_flow
