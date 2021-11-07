use crate::common::Endpoint;
use crate::filter::{
    Filter, FilterCmd, FilterCmdSend, FilterMode, FilterNotice, FilterNotifyRecv, FilterRspRecv, PingPong,
};
use crate::protocol::{BroadcastChatMessage, Packet, RequestAction, ResponseCode};
use crate::settings::TRANSPORT_CHANNEL_LEN;
use crate::transport::{TransportCmd, TransportCmdRecv, TransportNotice, TransportNotifySend, TransportRsp};
use lazy_static::lazy_static;
use snowflake::ProcessUniqueId;
use std::future::Future;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{self, timeout_at, Instant};

lazy_static! {
    static ref CLIENT_ENDPOINT: Endpoint = Endpoint(("1.2.3.4", 5678).to_socket_addrs().unwrap().next().unwrap());
    static ref SERVER_ENDPOINT: Endpoint = Endpoint(("2.4.6.8", 5678).to_socket_addrs().unwrap().next().unwrap());
}

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
    let (
        transport_notice_tx,
        mut transport_cmd_rx,
        filter_cmd_tx,
        _filter_rsp_rx,
        _filter_notify_rx,
        logged_in_resp_pkt_tid,
        filter_shutdown_watcher,
    ) = setup_server().await;

    // The logged_in_resp_pkt_tid is the transport ID of the LoggedIn packet sent by the server's
    // filter layer down to the transport layer, for sending to the client. Once the server's
    // filter layer receives acknowledgement from the client, the transport layer should receive a
    // DropPacket transport command with that transport ID.
    let action = RequestAction::KeepAlive {
        latest_response_ack: 1, // Must match the sequence sent in last Response server sent to this client (LoggedIn)
    };
    let packet = Packet::Request {
        sequence: 2,
        response_ack: Some(1), // Must match the sequence sent in last Response server sent to this client (LoggedIn)
        cookie: Some("fakecookie".to_owned()),
        action,
    };
    transport_notice_tx
        .send(TransportNotice::PacketDelivery {
            endpoint: *CLIENT_ENDPOINT,
            packet,
        })
        .await
        .unwrap();

    // Check for DropPacket (this should happen before response comes down from app layer)
    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await;
    let transport_cmd = timeout_at(expiration, transport_cmd_rx.recv())
        .await
        .expect("we should not have timed out getting a transport cmd from filter layer");
    let transport_cmd = transport_cmd.expect("should have gotten a TransportCmd from Filter");
    match transport_cmd {
        TransportCmd::DropPacket { endpoint, tid } => {
            assert_eq!(endpoint, *CLIENT_ENDPOINT);
            assert_eq!(tid, logged_in_resp_pkt_tid);
        }
        _ => panic!("unexpected transport command {:?}", transport_cmd),
    }

    // Shut down
    filter_cmd_tx
        .send(FilterCmd::Shutdown { graceful: false })
        .await
        .unwrap();
    filter_shutdown_watcher.await;
}

#[ignore] // TODO: PR_GATE: once more of the Update/UpdateReply stuff is written, re-enable this test
#[tokio::test]
async fn server_send_chats_with_ack_should_drop() {
    let (
        transport_notice_tx,
        mut transport_cmd_rx,
        filter_cmd_tx,
        _filter_rsp_rx,
        _filter_notify_rx,
        _,
        filter_shutdown_watcher,
    ) = setup_server().await;

    let chat = BroadcastChatMessage {
        chat_seq:    Some(0),
        player_name: "Teg".to_owned(),
        message:     "text from another player!".to_owned(),
    };
    filter_cmd_tx
        .send(FilterCmd::SendChats {
            endpoints: vec![*CLIENT_ENDPOINT],
            messages:  vec![chat.clone()],
        })
        .await
        .expect("sending a command down to Filter layer should succeed");

    let transport_cmd = transport_cmd_rx
        .recv()
        .await
        .expect("should have gotten a transport command");
    let packet_to_client;
    let update_pkt_tid;
    match transport_cmd {
        TransportCmd::SendPackets {
            endpoint: _endpoint,
            packets,
            packet_infos,
        } => {
            assert_eq!(*CLIENT_ENDPOINT, _endpoint);
            // No need to test packet_infos
            packet_to_client = packets
                .into_iter()
                .next()
                .expect("expected at least one packet for Transport layer");
            assert_eq!(
                packet_infos.len(),
                1,
                "multiple packets sent when one was expected for Update message to client"
            );
            update_pkt_tid = packet_infos[0].tid;
        }
        _ => panic!("unexpected TransportCmd"),
    }
    match packet_to_client {
        Packet::Update { chats, .. } => {
            assert_eq!(
                chats.len(),
                1,
                "expected a single chat message from Filter layer (server mode)"
            );
            assert_eq!(chats[0], chat);
        }
        _ => panic!("expected a Packet::Update, got {:?}", packet_to_client),
    }

    // Simulate an UpdateReply from the client that acks the chat message
    let packet_from_client = Packet::UpdateReply {
        cookie:               "fakecookie".to_owned(),
        last_chat_seq:        Some(0), // should match chat_seq from BroadcastChatMessage above
        last_game_update_seq: None,
        last_full_gen:        None,
        partial_gen:          None,
        pong:                 PingPong { nonce: 0 },
    };
    transport_notice_tx
        .send(TransportNotice::PacketDelivery {
            endpoint: *CLIENT_ENDPOINT,
            packet:   packet_from_client,
        })
        .await
        .expect("sending packets from Transport up to Filter should succeed");

    // Verify the Filter layer sent a DropPacket for the Update packet it sent earlier with the
    // acked chat message
    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await;
    let transport_cmd = timeout_at(expiration, transport_cmd_rx.recv())
        .await
        .expect("we should not have timed out getting a transport cmd from filter layer");
    let transport_cmd = transport_cmd.expect("should have gotten a TransportCmd from Filter");
    match transport_cmd {
        TransportCmd::DropPacket { endpoint, tid } => {
            assert_eq!(endpoint, *CLIENT_ENDPOINT);
            assert_eq!(tid, update_pkt_tid);
        }
        _ => panic!("unexpected transport command {:?}", transport_cmd),
    }

    // Shut down
    filter_cmd_tx
        .send(FilterCmd::Shutdown { graceful: false })
        .await
        .unwrap();
    filter_shutdown_watcher.await;
}

// TODO: basic_client_filter_flow

#[tokio::test]
async fn client_measure_latency_to_server() {
    let (
        transport_notice_tx,
        mut transport_cmd_rx,
        filter_cmd_tx,
        _filter_rsp_rx,
        mut filter_notify_rx,
        filter_shutdown_watcher,
    ) = setup_client().await;

    filter_cmd_tx
        .send(FilterCmd::AddPingEndpoints {
            endpoints: vec![*SERVER_ENDPOINT],
        })
        .await
        .expect("sending a command down to Filter layer should succeed");

    println!("Sent filter cmd");

    // Allow for one ping interval stream tick to occur
    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await;
    let transport_cmd = timeout_at(expiration, transport_cmd_rx.recv())
        .await
        .expect("we should not have timed out getting a transport cmd from filter layer");

    let transport_cmd = transport_cmd
        .expect("should have gotten a transport command");
    let packet_to_server;
    let pong;
    let pingpong_tid;
    match transport_cmd {
        TransportCmd::SendPackets {
            endpoint: _endpoint,
            packets,
            packet_infos,
        } => {
            assert_eq!(*SERVER_ENDPOINT, _endpoint);
            // No need to test packet_infos
            packet_to_server = packets
                .into_iter()
                .next()
                .expect("expected at least one packet for Transport layer");
            assert_eq!(
                packet_infos.len(),
                1,
                "multiple packets sent when one was expected for GetStatus message to server"
            );
            pingpong_tid = packet_infos[0].tid;
        }
        _ => panic!("unexpected TransportCmd"),
    }
    match packet_to_server {
        Packet::GetStatus { ping } => {
            pong = ping;
        }
        _ => panic!("expected a Packet::GetStatus, got {:?}", packet_to_server),
    }

    println!("Sent GetStatus cmd");

    // Simulate an Status from the Server with the Ping's Pong
    let server_player_count = 10;
    let server_room_count = 20;
    let server_name = "Chapterhouse".to_owned();
    let server_version = "1.2.3".to_owned();
    let packet_from_server = Packet::Status {
        player_count: server_player_count,
        room_count: server_room_count,
        server_name: server_name.clone(),
        server_version: server_version.clone(),
        pong,
    };
    transport_notice_tx
        .send(TransportNotice::PacketDelivery {
            endpoint: *SERVER_ENDPOINT,
            packet:   packet_from_server,
        })
        .await
        .expect("sending packets from Transport up to Filter should succeed");

    println!("Sent PacketDelivery cmd");

    // Verify the Filter layer sent a DropPacket for the Update packet it sent earlier with the
    // acked chat message
    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await;
    let transport_cmd = timeout_at(expiration, transport_cmd_rx.recv())
        .await
        .expect("we should not have timed out getting a transport cmd from filter layer");
    let transport_cmd = transport_cmd.expect("should have gotten a TransportCmd from Filter");
    match transport_cmd {
        TransportCmd::DropPacket { endpoint, tid } => {
            assert_eq!(endpoint, *CLIENT_ENDPOINT);
            assert_eq!(tid, pingpong_tid);
        }
        _ => panic!("unexpected transport command {:?}", transport_cmd),
    }

    println!("Transport cmd received");

    // Verify that the Filter layer sent a message to the App layer with the ping result. The ping result produce no
    // latency measurement after one cycle.
    let expiration = Instant::now() + Duration::from_secs(3);
    time::advance(Duration::from_secs(5)).await;
    let filter_notification = timeout_at(expiration, filter_notify_rx.recv())
        .await
        .expect("we should not have timed out getting a notification from the filter layer");
    let filter_notification = filter_notification.expect("should have gotten a FilterNotice from Filter");
    match filter_notification {
        FilterNotice::PingResult {
            endpoint,
            latency,
            player_count,
            room_count,
            server_name: name,
            server_version: version,
        } => {
            assert_eq!(endpoint, *SERVER_ENDPOINT);
            assert_eq!(latency, 0);
            assert_eq!(player_count, server_player_count);
            assert_eq!(room_count, server_room_count);
            assert_eq!(name, server_name);
            assert_eq!(version, server_version)
        }
        _ => panic!("unexpected transport command {:?}", transport_cmd),
    }

    // Shut down
    filter_cmd_tx
        .send(FilterCmd::Shutdown { graceful: false })
        .await
        .unwrap();
    filter_shutdown_watcher.await;
}

/// This is a helper to simplify setting up the filter layer in server mode with one client connection. Call like this:
///
/// ```rust
/// let (transport_notice_tx, transport_cmd_rx, filter_cmd_tx, filter_rsp_rx, filter_notify_rx, logged_in_resp_pkt_tid, filter_shutdown_watcher) = setup_server().await;
/// ```
async fn setup_server() -> (
    TransportNotifySend,
    TransportCmdRecv,
    FilterCmdSend,
    FilterRspRecv,
    FilterNotifyRecv,
    ProcessUniqueId,
    impl Future<Output = ()> + 'static,
) {
    time::pause();
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
            endpoint: *CLIENT_ENDPOINT,
            packet:   packet_from_client,
        })
        .await
        .unwrap();

    let expiration = Instant::now() + Duration::from_secs(3);
    // Advance the time so that we can call timeout_at() below, which reduces the chance of a bug causing this test to block forever.
    time::advance(Duration::from_secs(5)).await;

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
            assert_eq!(*CLIENT_ENDPOINT, _endpoint);
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
            endpoint: *CLIENT_ENDPOINT,
            code:     resp_code_for_client.clone(),
        })
        .await
        .expect("should successfully send a command from App layer down to Filter layer");

    // Check that the LoggedIn response code was sent down to the Transport layer
    let transport_cmd = transport_cmd_rx
        .recv()
        .await
        .expect("should have gotten a TransportCmd from Filter");
    let packet_to_client;
    let logged_in_resp_pkt_tid;
    match transport_cmd {
        TransportCmd::SendPackets {
            endpoint: _endpoint,
            packets,
            packet_infos,
        } => {
            assert_eq!(*CLIENT_ENDPOINT, _endpoint);
            // No need to test packet_infos
            packet_to_client = packets
                .into_iter()
                .next()
                .expect("expected at least one packet for Transport layer");
            assert_eq!(
                packet_infos.len(),
                1,
                "multiple packets sent when one was expected for LoggedIn response"
            );
            logged_in_resp_pkt_tid = packet_infos[0].tid;
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
    (
        transport_notice_tx,
        transport_cmd_rx,
        filter_cmd_tx,
        filter_rsp_rx,
        filter_notify_rx,
        logged_in_resp_pkt_tid,
        filter_shutdown_watcher,
    )
}

/// This is a helper to simplify setting up the filter layer in client mode with one server connection. Call like this:
///
/// ```rust
/// let (transport_notice_tx, transport_cmd_rx, filter_cmd_tx, filter_rsp_rx, filter_notify_rx, logged_in_resp_pkt_tid, filter_shutdown_watcher) = setup_client().await;
/// ```
async fn setup_client() -> (
    TransportNotifySend,
    TransportCmdRecv,
    FilterCmdSend,
    FilterRspRecv,
    FilterNotifyRecv,
    impl Future<Output = ()> + 'static,
) {
    time::pause();
    // Mock transport channels
    let (transport_cmd_tx, mut transport_cmd_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_rsp_tx, transport_rsp_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
    let (transport_notice_tx, transport_notice_rx) = mpsc::channel(TRANSPORT_CHANNEL_LEN);

    let (mut filter, filter_cmd_tx, filter_rsp_rx, mut filter_notify_rx) = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Client,
    );

    let filter_shutdown_watcher = filter.get_shutdown_watcher(); // No await; get the future

    // Start the filter's task in the background
    tokio::spawn(async move { filter.run().await });

    (
        transport_notice_tx,
        transport_cmd_rx,
        filter_cmd_tx,
        filter_rsp_rx,
        filter_notify_rx,
        filter_shutdown_watcher,
    )
}
