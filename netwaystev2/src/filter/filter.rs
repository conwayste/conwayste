use super::interface::{ClientAuthFields, FilterCmd, FilterMode, FilterNotice, FilterRsp, SeqNum};
use super::ping::LatencyFilter;
#[allow(unused)] // ToDo: need this?
use super::{ClientRoom, FilterEndpointData, FilterError, OtherEndClient, OtherEndServer, PerEndpoint, PingPong};
use super::{FilterCmdRecv, FilterCmdSend, FilterNotifyRecv, FilterNotifySend, FilterRspRecv, FilterRspSend};
use crate::common::{Endpoint, ShutdownWatcher};
#[allow(unused)] // ToDo: need this?
use crate::protocol::{GameUpdate, GenStateDiffPart, Packet, RequestAction, ResponseCode};
use crate::settings::{DEFAULT_ENDPOINT_TIMEOUT_INTERVAL, DEFAULT_RETRY_INTERVAL, FILTER_CHANNEL_LEN};
use crate::transport::{
    PacketSettings, TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp,
    TransportRspRecv,
};
#[allow(unused)]
use crate::{nwdebug, nwerror, nwinfo, nwtrace, nwwarn};
#[allow(unused)]
use anyhow::{anyhow, bail};
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::{self, error::SendError};
use tokio::sync::watch;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::Wrapping,
    time::Duration,
};

#[derive(PartialEq, Debug)]
pub(crate) enum SeqNumAdvancement {
    BrandNew,
    Contiguous,
    OutOfOrder,
    Duplicate,
}

pub type FilterInit = (Filter, FilterCmdSend, FilterRspRecv, FilterNotifyRecv);

#[derive(Copy, Clone, Debug)]
enum Phase {
    Running,
    ShutdownInProgress,
    ShutdownComplete,
}

pub struct Filter {
    transport_cmd_tx:    TransportCmdSend,
    transport_rsp_rx:    TransportRspRecv,
    transport_notice_rx: TransportNotifyRecv,
    filter_cmd_rx:       FilterCmdRecv,
    filter_rsp_tx:       FilterRspSend,
    filter_notice_tx:    FilterNotifySend,
    mode:                FilterMode,
    per_endpoint:        PerEndpoint,
    phase_watch_tx:      watch::Sender<Phase>,
    phase_watch_rx:      watch::Receiver<Phase>,
    /// Endpoints for pinging; the endpoints here aren't necessarily in `per_endpoint`
    ping_endpoints:      HashMap<Endpoint, LatencyFilter<PingPong>>,
    auth_requests:       HashSet<Endpoint>,
}

impl Filter {
    pub fn new(
        transport_cmd_tx: TransportCmdSend,
        transport_rsp_rx: TransportRspRecv,
        transport_notice_rx: TransportNotifyRecv,
        mode: FilterMode,
    ) -> FilterInit {
        let (filter_cmd_tx, filter_cmd_rx): (FilterCmdSend, FilterCmdRecv) = mpsc::channel(FILTER_CHANNEL_LEN);
        let (filter_rsp_tx, filter_rsp_rx): (FilterRspSend, FilterRspRecv) = mpsc::channel(FILTER_CHANNEL_LEN);
        let (filter_notice_tx, filter_notice_rx): (FilterNotifySend, FilterNotifyRecv) =
            mpsc::channel(FILTER_CHANNEL_LEN);

        let per_endpoint = PerEndpoint::new();
        let ping_endpoints = HashMap::new();
        let auth_requests = HashSet::new();

        let (phase_watch_tx, phase_watch_rx) = watch::channel(Phase::Running);

        (
            Filter {
                transport_cmd_tx,
                transport_rsp_rx,
                transport_notice_rx,
                filter_cmd_rx,
                filter_rsp_tx,
                filter_notice_tx,
                mode,
                per_endpoint,
                phase_watch_tx,
                phase_watch_rx,
                ping_endpoints,
                auth_requests,
            },
            filter_cmd_tx,
            filter_rsp_rx,
            filter_notice_rx,
        )
    }

    pub async fn run(&mut self) {
        // Transport cmd tx is cloned as used by the shutdown path to notify the transport layer of a shutdown event
        let transport_cmd_tx = self.transport_cmd_tx.clone();
        tokio::pin!(transport_cmd_tx);

        let filter_rsp_tx = self.filter_rsp_tx.clone();
        let filter_notice_tx = self.filter_notice_tx.clone();
        tokio::pin!(filter_rsp_tx);
        tokio::pin!(filter_notice_tx);

        let mut ping_interval_stream = tokio::time::interval(Duration::from_millis(200));

        loop {
            tokio::select! {
                response = self.transport_rsp_rx.recv() => {
                    // trace!("[F<-T,R]: {:?}", response);

                    if let Some(response) = response {
                        match response {
                            TransportRsp::Accepted => {
                                nwtrace!(self, "[F<-T,R] Command Accepted");
                            }
                            TransportRsp::SendPacketsLengthMismatch => {
                                nwerror!(self, "[F<-T,R] bug in filter layer! Length mismatch between parallel arrays in SendPackets command")
                            }
                            TransportRsp::BufferFull => {
                                // ToDo: understand if there is other action that needs to be taken besides logging
                                nwerror!(self, "[F<-T,R] Transmit buffer is full");
                            }
                            TransportRsp::ExceedsMtu {tid, size, mtu} => {
                                // ToDo: reduce outgoing packet size and resend
                                nwerror!(self, "[F<-T,R] Packet exceeds MTU size of {}. Tid={} and size is {}", mtu, tid, size);
                            }
                            TransportRsp::EndpointError {error} => {
                                nwerror!(self, "[F<-T,R] Endpoint error: {:?}", error);
                            }
                        }
                    }
                }
                notice = self.transport_notice_rx.recv() => {
                    if let Some(notice) = notice {
                        match notice {
                            TransportNotice::PacketDelivery{
                                endpoint,
                                packet,
                            } => {
                                nwtrace!(self, "[F<-T,N] For {:?}, took packet {:?}", endpoint, packet);
                                if let Err(e) = self.process_transport_packet(endpoint, packet, &mut filter_notice_tx).await {
                                    nwerror!(self, "[F] handling of incoming packet failed: {:?}", e);

                                    // Should not return unless it's a SendError
                                    if e.downcast_ref::<SendError<TransportCmd>>().is_some() || e.downcast_ref::<SendError<FilterNotice>>().is_some() {
                                        nwerror!(self, "[F] run() exiting");
                                        return;
                                    }
                                } else {
                                    // Nothing to do for Ok
                                }
                            }
                            TransportNotice::EndpointTimeout {
                                endpoint,
                            } => {
                                nwinfo!(self, "[F<-T,N] {:?} timed-out. Dropping.", endpoint);
                                self.per_endpoint.remove(&endpoint);
                                if transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await.is_err() {
                                    nwerror!(self, "[F] transport cmd receiver has been dropped");
                                    nwerror!(self, "[F] run() exiting");
                                    return;
                                }

                                // Pass it up to App layer
                                if filter_notice_tx.send(FilterNotice::EndpointTimeout{endpoint}).await.is_err() {
                                    nwerror!(self, "[F] filter notice receiver (App layer) has been dropped");
                                    nwerror!(self, "[F] run() exiting");
                                    return;
                                }
                            }
                            TransportNotice::EndpointIdle { endpoint } => {
                                nwinfo!(self, "[F<-T,N] {:?} reported as idle; may send KeepAlive (if client and not ping endpoint)", endpoint);
                                if self.mode.is_client() && !self.ping_endpoints.contains_key(&endpoint) {
                                    // response_ack filled in later (see HACK)
                                    let action = RequestAction::KeepAlive { latest_response_ack: 0 };
                                    if let Err(e) = self.send_request_action_to_server(endpoint, action).await {
                                        nwwarn!(self, "[F] error sending KeepAlive for idle {:?}: {}", endpoint, e);
                                    }
                                }
                            }
                        }
                    }
                }
                command = self.filter_cmd_rx.recv() => {
                    if let Some(command) = command {
                        nwtrace!(self, "[F<-A,C] New command: {:?}", command);

                        if let Err(e) = self.process_filter_command(command).await {
                            if let Some(err) = e.downcast_ref::<FilterError>() {
                                use FilterError::*;
                                match err {
                                    ShutdownRequested{graceful} => {
                                        nwinfo!(self, "[F] shutting down");
                                        let phase;
                                        if *graceful {
                                            phase = Phase::ShutdownComplete;
                                        } else {
                                            phase = Phase::ShutdownInProgress
                                        }
                                        let _ = self.phase_watch_tx.send(phase); // OK to ignore error
                                        return;
                                    }
                                    EndpointNotFound{endpoint} => {
                                        if filter_rsp_tx.send(FilterRsp::NoSuchEndpoint{endpoint: *endpoint}).await.is_err() {
                                            nwerror!(self, "[F] run() exiting -- all receivers on FilterRsp channel have been dropped");
                                            return;
                                        }
                                    }
                                    UnexpectedData { mode, .. } => {
                                        nwerror!(self, "[F] [{:?}] unexpected data: {}", mode, err);
                                        // TODO: pass error to App layer
                                        if filter_rsp_tx.send(FilterRsp::Accepted).await.is_err() {
                                            nwerror!(self, "[F] run() exiting -- all receivers on FilterRsp channel have been dropped");
                                            return;
                                        }
                                    }
                                    InternalError { .. } => {
                                        nwerror!(self, "[F] internal error: {}", err);
                                        // TODO: pass error to App layer
                                        if filter_rsp_tx.send(FilterRsp::Accepted).await.is_err() {
                                            nwerror!(self, "[F] run() exiting -- all receivers on FilterRsp channel have been dropped");
                                            return;
                                        }
                                    }
                                }
                            }
                            nwerror!(self, "[F<-A,C] command processing failed: {}", e);
                        } else {
                            if filter_rsp_tx.send(FilterRsp::Accepted).await.is_err() {
                                nwerror!(self, "[F] run() exiting -- all receivers on FilterRsp channel have been dropped");
                                return;
                            }
                        }
                    }
                }
                _instant = ping_interval_stream.tick() => {
                    if self.mode.is_client() {
                        if self.ping_endpoints.keys().len() != 0 {
                            nwinfo!(self, "[F] About to send pings to servers: {:?}", self.ping_endpoints.keys());
                        }
                        if let Err(e) = self.send_pings().await {
                            nwerror!(self, "[F->T,C] Failed to send pings: {}", e);
                        }
                    }
                }
            }
        }
    }

    async fn process_transport_packet(
        &mut self,
        endpoint: Endpoint,
        packet: Packet,
        filter_notice_tx: &mut FilterNotifySend,
    ) -> anyhow::Result<()> {
        if !self.per_endpoint.contains_key(&endpoint) {
            let mut ignore_unknown_endpoint = true;
            if self.mode.is_server() {
                // If I am a server, ignore any incoming packets from unknown hosts other
                // than GetStatus and Request->Connect (with no new cookie).
                if let Packet::GetStatus { .. } = &packet {
                    ignore_unknown_endpoint = false;
                } else if is_valid_connect_packet(&packet) {
                    ignore_unknown_endpoint = false;

                    // New client connection!

                    self.per_endpoint.insert(
                        endpoint,
                        FilterEndpointData::OtherEndClient(OtherEndClient::new(endpoint)),
                    );
                    self.transport_cmd_tx
                        .send(TransportCmd::NewEndpoint {
                            endpoint,
                            timeout: DEFAULT_ENDPOINT_TIMEOUT_INTERVAL,
                        })
                        .await?;
                }
            } else {
                // If I am a client, ignore any incoming packets from unknown hosts (servers
                // I previously sent pings to are excluded).
                if self.ping_endpoints.contains_key(&endpoint) {
                    ignore_unknown_endpoint = false;
                }
            }

            if ignore_unknown_endpoint {
                // The connection was not accepted for this new endpoint. No need to log it.
                if self.mode.is_server() {
                    self.send_not_connected(endpoint).await;
                }
                return Ok(());
            }
        }

        // TODO: this badly needs to be refactored. It's a lot for one function.
        match packet {
            Packet::Request {
                sequence,
                ref action,
                response_ack,
                ref cookie,
            } => {
                let client =
                    self.per_endpoint
                        .other_end_client_ref_mut(&endpoint, &self.mode, Some("RequestAction"))?;

                client
                    .resend_and_drop_enqueued_response_codes(&self.transport_cmd_tx, response_ack)
                    .await?;

                match action {
                    RequestAction::Connect { .. } => {} // No cookie OK here
                    _ => {
                        if *cookie != client.cookie {
                            // Cookie must match, since it's not a Connect action
                            bail!(
                                "Expected cookie {:?} from client {:?} but received cookie {:?}",
                                client.cookie,
                                endpoint,
                                *cookie
                            );
                        }
                    }
                }

                match determine_seq_num_advancement(sequence, client.last_request_sequence_seen) {
                    SeqNumAdvancement::Duplicate => {
                        // This can happen under normal network conditions
                        return Ok(());
                    }
                    SeqNumAdvancement::BrandNew | SeqNumAdvancement::Contiguous => {
                        client.last_request_sequence_seen = Some(Wrapping(sequence));
                    }
                    SeqNumAdvancement::OutOfOrder => {
                        // Nothing to do but add it to the heap in the next step
                    }
                }

                client.request_actions.add(sequence, action.clone());

                // Loop over the heap, finding all requests which can be sent to the app layer based on their sequence number.
                // If any are found, send them to the app layer and advance the last seen sequence number.
                // ToDo: unit test wrapping logic
                let ref mut expected_seq_num = if let Some(request_seq) = client.last_request_sequence_seen {
                    request_seq
                } else {
                    // Shouldn't be possible; if we hit this, it's a bug somewhere above
                    return Err(anyhow!(FilterError::InternalError {
                        problem: "sequence number should not be None at this point".to_owned(),
                    }));
                };

                while let Some(request_action) = client.request_actions.take_if_matching(expected_seq_num.0) {
                    let notice = if let Some(caf) = client_auth_fields_from_connect_packet(&packet) {
                        FilterNotice::ClientAuthRequest { endpoint, fields: caf }
                    } else {
                        client.process_request_action(&request_action);
                        FilterNotice::NewRequestAction {
                            endpoint,
                            action: request_action,
                        }
                    };
                    filter_notice_tx.send(notice).await?;
                    *expected_seq_num += Wrapping(1);
                }
            }
            Packet::Response {
                sequence,
                request_ack,
                code,
            } => {
                let server = self
                    .per_endpoint
                    .other_end_server_ref_mut(&endpoint, &self.mode, Some("ResponseCode"))?;

                match determine_seq_num_advancement(sequence, server.last_response_sequence_seen) {
                    SeqNumAdvancement::Duplicate => {
                        // This can happen under normal network conditions
                        return Ok(());
                    }
                    SeqNumAdvancement::BrandNew | SeqNumAdvancement::Contiguous => {
                        server.last_response_sequence_seen = Some(Wrapping(sequence));
                    }
                    SeqNumAdvancement::OutOfOrder => {
                        // Nothing to do but add it to the heap in the next step
                    }
                }

                let mut tids_to_drop = vec![];
                if let Some(request_ack) = request_ack {
                    tids_to_drop = take_tids_to_drop(&mut server.unacked_outgoing_packet_tids, Wrapping(request_ack));
                }
                for tid_to_drop in tids_to_drop {
                    self.transport_cmd_tx
                        .send(TransportCmd::DropPacket {
                            endpoint,
                            tid: tid_to_drop,
                        })
                        .await?;
                }

                server.response_codes.add(sequence, code);

                // Loop over the heap, finding all responses which can be sent to the app layer based on their sequence number.
                // If any are found, send them to the app layer and advance the last seen sequence number.
                // ToDo: unit test wrapping logic
                let ref mut expected_seq_num = if let Some(response_seq) = server.last_response_sequence_seen {
                    response_seq
                } else {
                    // Shouldn't be possible; if we hit this, it's a bug somewhere above
                    return Err(anyhow!(FilterError::InternalError {
                        problem: "sequence number should not be None at this point".to_owned(),
                    }));
                };
                while let Some(response_code) = server.response_codes.take_if_matching(expected_seq_num.0) {
                    // TODO: move response code handling into separate function
                    match response_code {
                        ResponseCode::JoinedRoom { .. } => {
                            server.room = Some(ClientRoom::new());
                            // When joining or leaving a room, the game_updates are reset
                            server.game_update_seq = None;
                        }
                        ResponseCode::LeaveRoom => {
                            // When joining or leaving a room, the game_updates are reset
                            server.game_update_seq = None;
                        }
                        ResponseCode::LoggedIn { ref cookie, .. } => {
                            server.cookie = Some(cookie.clone());
                        }
                        ResponseCode::PlayerList { ref players } => {
                            if let Some(ref mut room) = server.room {
                                // ToDo: move to update_player_list func on the room and make `other_layers` private again.
                                // We are in room and just asked for player list (probably means we
                                // just entered the room). Update room.other_players based on the
                                // received list of player names.
                                for player in players {
                                    if server.player_name == *player {
                                        // Self
                                        continue;
                                    }
                                    room.other_players.entry(player.clone()).or_insert(None);
                                }

                                // Also remove any extraneous entries in other_players
                                let mut to_remove = vec![];
                                for name in room.other_players.keys() {
                                    if !players.contains(name) {
                                        to_remove.push(name.clone());
                                    }
                                }
                                for name in to_remove {
                                    room.other_players.remove(&name);
                                }
                            }
                        }
                        _ => {} // Most variants don't need any client-side state changes in the
                                // Filter layer
                    }

                    // Send the ResponseCode up to the app layer
                    filter_notice_tx
                        .send(FilterNotice::NewResponseCode {
                            endpoint,
                            code: response_code,
                        })
                        .await?;
                    *expected_seq_num += Wrapping(1);
                }
            }
            Packet::Update {
                chats,
                game_update_seq,
                game_updates,
                universe_update,
                ping,
            } => {
                let server = self
                    .per_endpoint
                    .other_end_server_ref_mut(&endpoint, &self.mode, Some("Update"))?;
                if let Some(ref mut room) = server.room {
                    if let Some(ref mut game) = room.game {
                        if let Some(gen_state_diff) = game.process_genstate_diff_part(universe_update)? {
                            filter_notice_tx
                                .send(FilterNotice::NewGenStateDiff {
                                    diff: gen_state_diff,
                                    endpoint,
                                })
                                .await?;
                        }
                    }
                }

                server
                    .process_game_updates(endpoint, game_update_seq, &game_updates, &filter_notice_tx)
                    .await?;

                if let Some(ref mut room) = server.room {
                    room.process_chats(endpoint, &chats, &filter_notice_tx).await?;
                }

                server.server_ping = ping;

                // Send reply (even if unchanged)
                server.send_update_reply(endpoint, &mut self.transport_cmd_tx).await?;
            }
            Packet::Status {
                player_count,
                ref pong,
                room_count,
                server_name,
                server_version,
            } => {
                if self.mode.is_server() {
                    return Err(anyhow!(FilterError::UnexpectedData {
                        mode:         self.mode.clone(),
                        invalid_data: "Status".to_owned(),
                    }));
                }
                if !self.ping_endpoints.contains_key(&endpoint) {
                    // Not error-worthy, since a ClearPingEndpoints can happen at any time, while
                    // Status packets from servers are in flight.
                    nwinfo!(self, "[F<-T,N] Received Status packet from server we have not pinged (or purged from ping_endpoints)");
                    return Ok(());
                }
                let latency_filter = self.ping_endpoints.get_mut(&endpoint).unwrap(); // unwrap OK because of above check

                // Update the round-trip time
                latency_filter.update(*pong);

                // Latency is Some(<n>) once the filter has seen enough data
                let latency = latency_filter.get_millis();
                nwinfo!(self, "[F] Latency for remote server {:?} is {:?}", endpoint, latency);

                // Notify App layer of the Server information and population
                filter_notice_tx
                    .send(FilterNotice::PingResult {
                        endpoint,
                        latency,
                        server_name,
                        server_version,
                        room_count,
                        player_count,
                    })
                    .await?;
            }
            Packet::GetStatus { ping } => {
                if self.mode.is_client() {
                    return Err(anyhow!(FilterError::UnexpectedData {
                        mode:         self.mode.clone(),
                        invalid_data: "GetStatus".to_owned(),
                    }));
                }

                self.send_server_status(endpoint, ping).await?;
            }
            Packet::UpdateReply {
                ref cookie,
                last_chat_seq,
                last_game_update_seq,
                last_full_gen,
                ref partial_gen,
                pong: _,
            } => {
                if self.mode.is_client() {
                    return Err(anyhow!(FilterError::UnexpectedData {
                        mode:         self.mode.clone(),
                        invalid_data: "UpdateReply".to_owned(),
                    }));
                }

                let client = self
                    .per_endpoint
                    .other_end_client_ref_mut(&endpoint, &self.mode, Some("UpdateReply"))?;

                // Validate cookie
                if client.cookie.is_none() || client.cookie.as_ref().unwrap() != cookie {
                    return Ok(());
                }

                // Process all of the UpdateReply components
                client.process_chat_ack(last_chat_seq).await?;
                client.process_game_update_ack(last_game_update_seq).await?;
                client.process_gen_ack(last_full_gen, partial_gen.as_ref()).await?;
            }
        }

        Ok(())
    }

    async fn send_not_connected(&self, endpoint: Endpoint) {
        let packets = vec![Packet::Response {
            code:        ResponseCode::NotConnected {
                error_msg: "Unknown client".into(),
            },
            sequence:    1,
            request_ack: None,
        }];

        let packet_infos = vec![PacketSettings {
            tid:            ProcessUniqueId::new(),
            retry_interval: Duration::ZERO, // Do not retry
        }];

        let _ = self
            .transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint,
                packet_infos,
                packets,
            })
            .await;
    }

    /// Warning: panics if not Server filter mode!
    async fn send_server_status(&mut self, endpoint: Endpoint, ping: PingPong) -> anyhow::Result<()> {
        let server_status = self.mode.server_status_mut().expect("should be server filter mode");
        let packets = vec![server_status.to_packet(ping)];
        let packet_infos = vec![PacketSettings {
            tid:            ProcessUniqueId::new(),
            retry_interval: Duration::ZERO,
        }];

        nwinfo!(
            self,
            "[F] Sending Status packet {:?} back to client {:?}",
            ping,
            endpoint
        );
        self.transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint,
                packet_infos,
                packets,
            })
            .await
            .map_err(|e| anyhow!(e))
    }

    async fn process_filter_command(&mut self, command: FilterCmd) -> anyhow::Result<()> {
        match command {
            FilterCmd::SendRequestAction { endpoint, action } => {
                if !self.per_endpoint.contains_key(&endpoint) {
                    // Create a new endpoint only on Connect messages
                    match &action {
                        RequestAction::Connect { name, .. } => {
                            self.per_endpoint.insert(
                                endpoint,
                                FilterEndpointData::OtherEndServer(OtherEndServer::new(name.clone())),
                            );
                            self.transport_cmd_tx
                                .send(TransportCmd::NewEndpoint {
                                    endpoint,
                                    timeout: DEFAULT_ENDPOINT_TIMEOUT_INTERVAL,
                                })
                                .await?;
                        }
                        _ => return Err(anyhow!(FilterError::EndpointNotFound { endpoint: endpoint })),
                    }
                }

                self.send_request_action_to_server(endpoint, action).await?
            }
            FilterCmd::SendResponseCode { endpoint, code } => {
                let client =
                    self.per_endpoint
                        .other_end_client_ref_mut(&endpoint, &self.mode, Some("SendResponseCode"))?;

                client.send_response_code(&self.transport_cmd_tx, code).await?;
            }
            FilterCmd::SendChats { endpoints, messages: _ } => {
                for endpoint in endpoints {
                    let _client = self.per_endpoint.other_end_client_ref_mut(
                        &endpoint,
                        &self.mode,
                        Some("Chats are not sent to servers"),
                    )?;
                    // TODO: send all the messages to this client (rename _client to client)
                }
            }
            // TODO: implement the following
            FilterCmd::SendGameUpdates {
                endpoints: _,
                updates: _,
            } => {}
            FilterCmd::CompleteAuthRequest { endpoint, decision } => {
                let client =
                    self.per_endpoint
                        .other_end_client_ref_mut(&endpoint, &self.mode, Some("CompleteAuthRequest"))?;
                let code: ResponseCode = decision.into();
                match code {
                    ResponseCode::LoggedIn { ref cookie, .. } => {
                        // ToDo: generate a random cookie here instead of needing it from LoggedIn
                        client.cookie = Some(cookie.clone());
                    }
                    _ => {}
                }
                client.send_response_code(&self.transport_cmd_tx, code).await?;
                self.auth_requests.remove(&endpoint);
            }
            // TODO: implement the following
            FilterCmd::SendGenStateDiff { endpoints: _, diff: _ } => {}
            FilterCmd::AddPingEndpoints { endpoints } => {
                for e in endpoints {
                    // An hashmap insert for an existing key will override the value. This would obsolete any ping
                    // already underway.
                    if self.ping_endpoints.contains_key(&e) {
                        continue;
                    }

                    self.ping_endpoints.insert(e, LatencyFilter::new());
                }
            }
            FilterCmd::ClearPingEndpoints => {
                nwinfo!(
                    self,
                    "[F<-A,C] clearing all ping endpoints: {:?}",
                    self.ping_endpoints.keys()
                );
                // Cancel any in progress pings
                for (endpoint, _ping_endpoint) in self.ping_endpoints.iter() {
                    let endpoint = *endpoint;
                    self.transport_cmd_tx
                        .send(TransportCmd::DropEndpoint { endpoint })
                        .await?;
                }
                self.ping_endpoints.clear();
            }
            FilterCmd::DropEndpoint { endpoint } => {
                nwinfo!(self, "[F<-A,C] dropping {:?}", endpoint);

                // Remove the endpoint from the ping list
                self.ping_endpoints.remove(&endpoint);

                // Remove the endpoint metadata
                self.per_endpoint.remove(&endpoint);

                // Drop auth_request data
                self.auth_requests.remove(&endpoint);

                // Inform the transport to drop the endpoint
                self.transport_cmd_tx
                    .send(TransportCmd::DropEndpoint { endpoint })
                    .await?;
            }
            FilterCmd::Shutdown { graceful } => {
                // Main logic is in error handling
                return Err(anyhow!(FilterError::ShutdownRequested { graceful }));
            }
            #[deny(unused_variables)]
            FilterCmd::ChangeServerStatus {
                server_version,
                player_count,
                room_count,
                server_name,
            } => {
                let server_status = match self.mode.server_status_mut() {
                    Some(server_status) => server_status,
                    None => {
                        return Err(anyhow!(FilterError::UnexpectedData {
                            mode:         self.mode.clone(),
                            invalid_data: "cannot ChangeServerStatus for a client".to_owned(),
                        }));
                    }
                };
                if let Some(server_version) = server_version {
                    server_status.server_version = server_version.clone();
                }
                if let Some(player_count) = player_count {
                    server_status.player_count = player_count;
                }
                if let Some(room_count) = room_count {
                    server_status.room_count = room_count;
                }
                if let Some(server_name) = server_name {
                    server_status.server_name = server_name.clone();
                }
            }
        }

        Ok(())
    }

    pub fn get_shutdown_watcher(&mut self) -> ShutdownWatcher {
        let mut phase_watch_rx = self.phase_watch_rx.clone();
        let transport_cmd_tx = self.transport_cmd_tx.clone();
        Box::pin(async move {
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
                    trace!("[F] phase watch channel was dropped");
                    break;
                }
            }
            // Also shutdown the layer below
            let _ = transport_cmd_tx.send(TransportCmd::Shutdown).await;
        })
    }

    async fn send_pings(&mut self) -> anyhow::Result<()> {
        for (endpoint, latency_filter) in self.ping_endpoints.iter_mut() {
            let pingpong = PingPong::ping();

            let pi = PacketSettings {
                retry_interval: Duration::ZERO,
                tid:            ProcessUniqueId::new(),
            };

            self.transport_cmd_tx
                .send(TransportCmd::SendPackets {
                    endpoint:     *endpoint,
                    packet_infos: vec![pi],
                    packets:      vec![Packet::GetStatus { ping: pingpong }],
                })
                .await?;
            latency_filter.start(pingpong);
        }
        Ok(())
    }

    async fn send_request_action_to_server(
        &mut self,
        endpoint: Endpoint,
        mut action: RequestAction,
    ) -> anyhow::Result<()> {
        let server = self
            .per_endpoint
            .other_end_server_ref_mut(&endpoint, &self.mode, Some("ResponseCode"))?;

        if let Some(ref mut sn) = server.last_request_sequence_sent {
            *sn += Wrapping(1u64);
        } else {
            server.last_request_sequence_sent = Some(Wrapping(1));
        }

        // Unwrap ok b/c the immediate check above guarantees Some(..)
        let sequence = server.last_request_sequence_sent.unwrap().0;

        let response_ack = server.last_response_sequence_seen.map(|response_sn| response_sn.0);

        // HACK: fill in latest_response_ack. The solution is probably to change the protocol,
        // removing latest_response_ack, which is redundant anyway.
        match action {
            RequestAction::KeepAlive {
                ref mut latest_response_ack,
            } => *latest_response_ack = response_ack.unwrap_or(0),
            _ => {}
        };

        let cookie = server.cookie.clone();

        let packets = vec![Packet::Request {
            action,
            cookie,
            sequence,
            response_ack,
        }];

        let tid = ProcessUniqueId::new();
        server.unacked_outgoing_packet_tids.push_back((Wrapping(sequence), tid));
        let packet_infos = vec![PacketSettings {
            tid,
            retry_interval: DEFAULT_RETRY_INTERVAL,
        }];

        self.transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint,
                packet_infos,
                packets,
            })
            .await
            .map_err(|e| anyhow!(e))
    }
}

fn is_valid_connect_packet(packet: &Packet) -> bool {
    if let Packet::Request { action, cookie, .. } = packet {
        if let RequestAction::Connect { .. } = action {
            if cookie.is_none() {
                return true;
            }
        }
    }
    false
}

fn client_auth_fields_from_connect_packet(packet: &Packet) -> Option<ClientAuthFields> {
    if let Packet::Request { action, cookie, .. } = packet {
        if let RequestAction::Connect { name, client_version } = action {
            if cookie.is_none() {
                return Some(ClientAuthFields {
                    player_name:    name.clone(),
                    client_version: client_version.clone(),
                });
            }
        }
    }
    None
}

// I've deemed 'far away' to mean the half of the max value of the type.
fn is_seq_sufficiently_far_away(a: u64, b: u64) -> bool {
    static HALFWAYPOINT: u64 = u64::max_value() / 2;
    if a > b {
        a - b > HALFWAYPOINT
    } else {
        b - a > HALFWAYPOINT
    }
}

/// `pkt_sequence` is the sequence number of the packet under process by the filter layer
/// `last_seen_sn` is the last seen sequence number for either a request OR a response, depending on the context
/// Returns a `SeqNumAdvancement` which determines if the inbound packet will need to be buffered (`OutOfOrder`) or
/// if it can be sent to the application layer immediately (`Contiguous`). Packet sequence numbers that are smaller in
/// value than `last_seen_sn` are considered `Duplicate`. The exception to this is if the sequence numbers are
/// about to wrap from `u64::MAX` to zero; these are still considered `OutOfOrder` by examining the distance between the
/// two numbers.
pub(crate) fn determine_seq_num_advancement(pkt_sequence: u64, last_seen_sn: Option<SeqNum>) -> SeqNumAdvancement {
    if let Some(last_sn) = last_seen_sn {
        let sequence_wrapped = Wrapping(pkt_sequence);

        if sequence_wrapped == last_sn + Wrapping(1) {
            return SeqNumAdvancement::Contiguous;
        } else if sequence_wrapped <= last_sn {
            if is_seq_sufficiently_far_away(sequence_wrapped.0, last_sn.0) {
                return SeqNumAdvancement::OutOfOrder;
            } else {
                return SeqNumAdvancement::Duplicate;
            }
        } else {
            return SeqNumAdvancement::OutOfOrder;
        }
    } else {
        return SeqNumAdvancement::BrandNew;
    }
}

fn take_tids_to_drop(
    unacked_outgoing_packet_tids: &mut VecDeque<(SeqNum, ProcessUniqueId)>,
    ack_seq: Wrapping<u64>,
) -> Vec<ProcessUniqueId> {
    let mut tids_to_drop = vec![];
    loop {
        if unacked_outgoing_packet_tids.len() == 0 {
            return tids_to_drop;
        }
        // unwraps OK below because of above length check
        let seq_at_front = unacked_outgoing_packet_tids.front().unwrap().0;
        if seq_at_front <= ack_seq
            || (is_seq_sufficiently_far_away(seq_at_front.0, ack_seq.0) && (seq_at_front >= ack_seq))
        {
            tids_to_drop.push(unacked_outgoing_packet_tids.pop_front().unwrap().1);
        } else {
            return tids_to_drop;
        }
    }
}
