use crate::common::Endpoint;
use crate::filter::{Filter, FilterCmd, FilterMode, FilterNotice};
use crate::protocol::{Packet, RequestAction, ResponseCode};
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

    let (mut filter, filter_cmd_tx, filter_rsp_rx, mut filter_notify_rx) = Filter::new(
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
    let request_action_from_client = RequestAction::Connect {
        name:           "Sheeana".to_owned(),
        client_version: "0.3.2".to_owned(),
    };
    let sequence_from_client = 1;
    let packet_from_client = Packet::Request {
        sequence:     sequence_from_client,
        response_ack: None,
        cookie:       None,
        action:       request_action_from_client.clone(),
    };
    transport_notice_tx
        .send(TransportNotice::PacketDelivery {
            endpoint,
            packet: packet_from_client,
        })
        .await
        .unwrap();

    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await; // TODO: once we add a test for a timing out flow, we can move this to that test and the timeout_at below

    // Check that we got a filter notification
    let timeout_result = timeout_at(expiration, filter_notify_rx.recv()).await;
    let filter_notification = timeout_result.expect("we should not have timed out getting a filter notification");

    let filter_notification = filter_notification.expect("channel should not have been closed");

    // Check that the correct notification was passed up to the app layer
    match filter_notification {
        FilterNotice::NewRequestAction {
            endpoint: _endpoint,
            action: _request_action,
        } => {
            assert_eq!(endpoint, _endpoint);
            assert_eq!(request_action_from_client, _request_action);
        }
        _ => panic!("Unexpected filter notification: {:?}", filter_notification),
    };

    // Send a logged in message from App layer to the Filter layer we are testing here
    let resp_code_for_client = ResponseCode::LoggedIn {
        cookie:         "fakecookie".to_owned(),
        server_version: "1.2.3.4.5".to_owned(),
    };
    filter_cmd_tx
        .send(FilterCmd::SendResponseCode {
            endpoint,
            code: resp_code_for_client.clone(),
        })
        .await
        .expect("should successfully send a command from App layer down to Filter layer");

    // Check that the LoggedIn response code was sent down to the Transport layer
    let transport_cmd = transport_cmd_rx
        .recv()
        .await
        .expect("should have gotten a TransportCmd from Filter");
    let packet_to_client;
    match transport_cmd {
        TransportCmd::SendPackets {
            endpoint: _endpoint,
            packets,
            ..
        } => {
            assert_eq!(endpoint, _endpoint);
            // No need to test packet_infos
            packet_to_client = packets
                .into_iter()
                .next()
                .expect("expected at least one packet for Transport layer");
        }
        _ => panic!("unexpected TransportCmd"),
    };
    match packet_to_client {
        Packet::Response {
            sequence,
            request_ack,
            code,
        } => {
            assert_eq!(sequence, 1);
            assert_eq!(request_ack, Some(sequence_from_client));
            assert_eq!(code, resp_code_for_client);
        }
        _ => panic!("expected a Packet::Response, got {:?}", packet_to_client),
    }

    // Shut down
    filter_cmd_tx
        .send(FilterCmd::Shutdown { graceful: false })
        .await
        .unwrap();
    filter_shutdown_watcher.await;
}

//XXX basic_client_filter_flow
