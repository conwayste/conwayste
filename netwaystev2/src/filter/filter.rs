use super::interface::{FilterCmd, FilterMode, FilterNotice, FilterRsp, SeqNum};
use super::ping::LatencyFilter;
use super::sortedbuffer::SequencedMinHeap;
use super::PingPong;
use crate::common::Endpoint;
use crate::protocol::{Packet, RequestAction, ResponseCode};
use crate::settings::{DEFAULT_RETRY_INTERVAL_NS, FILTER_CHANNEL_LEN};
use crate::transport::{
    PacketSettings, TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp,
    TransportRspRecv,
};
use anyhow::anyhow;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::watch;

use std::time::Duration;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    num::Wrapping,
    time::Instant,
};

#[derive(PartialEq, Debug)]
pub(crate) enum SeqNumAdvancement {
    BrandNew,
    Contiguous,
    OutOfOrder,
    Duplicate,
}

pub enum FilterEndpointData {
    OtherEndClient(OtherEndClient),
    OtherEndServer(OtherEndServer),
}

pub struct OtherEndClient {
    request_actions:              SequencedMinHeap<RequestAction>,
    last_request_sequence_seen:   Option<SeqNum>,
    last_response_sequence_sent:  Option<SeqNum>,
    last_request_seen_timestamp:  Option<Instant>,
    last_response_sent_timestamp: Option<Instant>,
    unacked_outgoing_packet_tids: VecDeque<(SeqNum, ProcessUniqueId)>,
}

pub struct OtherEndServer {
    response_codes:               SequencedMinHeap<ResponseCode>,
    last_request_sequence_sent:   Option<SeqNum>,
    last_response_sequence_seen:  Option<SeqNum>,
    last_request_sent_timestamp:  Option<Instant>,
    last_response_seen_timestamp: Option<Instant>,
    unacked_outgoing_packet_tids: VecDeque<(SeqNum, ProcessUniqueId)>,
}

#[derive(Debug, thiserror::Error)]
pub enum FilterEndpointDataError {
    #[error("Filter mode ({mode:?}) is not configured to receive {invalid_data}")]
    UnexpectedData {
        mode:         FilterMode,
        invalid_data: String,
    },
    #[error("Internal Filter layer error: {problem}")]
    InternalError { problem: String },
    #[error("Filter does not contain an entry for the endpoint: {endpoint:?}")]
    EndpointNotFound { endpoint: Endpoint },
}

#[derive(Debug, thiserror::Error)]
pub enum FilterCommandError {
    #[error("Filter is shutting down. Graceful: {graceful}")]
    ShutdownRequested { graceful: bool },
}

pub type FilterCmdSend = Sender<FilterCmd>;
type FilterCmdRecv = Receiver<FilterCmd>;
type FilterRspSend = Sender<FilterRsp>;
pub type FilterRspRecv = Receiver<FilterRsp>;
type FilterNotifySend = Sender<FilterNotice>;
pub type FilterNotifyRecv = Receiver<FilterNotice>;

pub type FilterInit = (Filter, FilterCmdSend, FilterRspRecv, FilterNotifyRecv);

#[derive(Copy, Clone, Debug)]
enum Phase {
    Running,
    ShutdownInProgress,
    ShutdownComplete,
}

pub struct Filter {
    transport_cmd_tx:    TransportCmdSend,
    transport_rsp_rx:    Option<TransportRspRecv>,    // TODO no option
    transport_notice_rx: Option<TransportNotifyRecv>, // TODO no option
    filter_cmd_rx:       Option<FilterCmdRecv>,       // TODO no option
    filter_rsp_tx:       FilterRspSend,
    filter_notice_tx:    FilterNotifySend,
    mode:                FilterMode,
    per_endpoint:        HashMap<Endpoint, FilterEndpointData>,
    phase_watch_tx:      Option<watch::Sender<Phase>>, // Temp. holding place. This is only Some(...) between new() and run() calls
    phase_watch_rx:      watch::Receiver<Phase>,
    ping_endpoints:      HashMap<Endpoint, (LatencyFilter, PingPong, Option<ProcessUniqueId>)>,
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

        let per_endpoint = HashMap::new();
        let ping_endpoints = HashMap::new();

        let (phase_watch_tx, phase_watch_rx) = watch::channel(Phase::Running);

        (
            Filter {
                transport_cmd_tx,
                transport_rsp_rx: Some(transport_rsp_rx),
                transport_notice_rx: Some(transport_notice_rx),
                filter_cmd_rx: Some(filter_cmd_rx),
                filter_rsp_tx,
                filter_notice_tx,
                mode,
                per_endpoint,
                phase_watch_tx: Some(phase_watch_tx),
                phase_watch_rx,
                ping_endpoints,
            },
            filter_cmd_tx,
            filter_rsp_rx,
            filter_notice_rx,
        )
    }

    pub async fn run(&mut self) {
        let transport_cmd_tx = self.transport_cmd_tx.clone();
        let transport_rsp_rx = self.transport_rsp_rx.take().unwrap();
        let transport_notice_rx = self.transport_notice_rx.take().unwrap();
        let phase_watch_tx = self.phase_watch_tx.take().unwrap();
        tokio::pin!(transport_cmd_tx);
        tokio::pin!(transport_rsp_rx);
        tokio::pin!(transport_notice_rx);

        let filter_cmd_rx = self.filter_cmd_rx.take().unwrap();
        let _filter_rsp_tx = self.filter_rsp_tx.clone();
        let filter_notice_tx = self.filter_notice_tx.clone();
        tokio::pin!(filter_cmd_rx);
        tokio::pin!(_filter_rsp_tx);
        tokio::pin!(filter_notice_tx);

        let mut ping_interval_stream = tokio::time::interval(Duration::new(2, 0));

        loop {
            tokio::select! {
                response = transport_rsp_rx.recv() => {
                    // trace!("[FILTER] Transport Response: {:?}", response);

                    if let Some(response) = response {
                        match response {
                            TransportRsp::Accepted => {
                                trace!("[FILTER] Transport Command Accepted");
                            }
                            TransportRsp::SendPacketsLengthMismatch => {
                                error!("Packet and PacketSettings data did not align")
                            }
                            TransportRsp::BufferFull => {
                                // XXX
                                error!("[FILTER] Transmit buffer is full");
                            }
                            TransportRsp::ExceedsMtu {tid} => {
                                // XXX
                                error!("[FILTER] Packet exceeds MTU size. Tid={}", tid);
                            }
                            TransportRsp::EndpointError {error} => {
                                error!("[FILTER] Transport Layer error: {:?}", error);
                            }
                        }
                    }
                }
                notice = transport_notice_rx.recv() => {
                    if let Some(notice) = notice {
                        match notice {
                            TransportNotice::PacketDelivery{
                                endpoint,
                                packet,
                            } => {
                                info!("[FILTER] Packet Taken from Endpoint {:?}.", endpoint);
                                trace!("[FILTER] Took packet: {:?}", packet);
                                if let Err(e) = self.process_transport_packet(endpoint, packet, &mut filter_notice_tx).await {
                                    error!("[FILTER] error processing incoming packet: {:?}", e);
                                }
                            }
                            TransportNotice::EndpointTimeout {
                                endpoint,
                            } => {
                                info!("[FILTER] Endpoint {:?} timed-out. Dropping.", endpoint);
                                self.per_endpoint.remove(&endpoint);
                                transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await.expect("transport cmd receiver should not be dropped");
                            }
                        }
                    }
                }
                command = filter_cmd_rx.recv() => {
                    if let Some(command) = command {
                        trace!("[FILTER] New command: {:?}", command);

                        if let Err(e) = self.process_filter_command(command).await {
                            if let Some(err) = e.downcast_ref::<FilterCommandError>() {
                                match err {
                                    FilterCommandError::ShutdownRequested{graceful} => {
                                        info!("[FILTER] shutting down");
                                        let phase;
                                        if *graceful {
                                            phase = Phase::ShutdownComplete;
                                        } else {
                                            phase = Phase::ShutdownInProgress
                                        }
                                        phase_watch_tx.send(phase).unwrap();
                                        return;
                                    }
                                }
                            }
                            error!("[FILTER] Filter command processing failed: {}", e);
                        }
                    }
                }
                _instant = ping_interval_stream.tick() => {
                    if let Err(e) = self.send_pings().await {
                        error!("[FILTER] Failed to send pings: {}", e);
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
        // TODO: also create endpoint data entry on a filter command to initiate a connection
        // (client mode only).
        if !self.per_endpoint.contains_key(&endpoint) {
            let mut valid_new_conn = false;
            if self.mode == FilterMode::Server {
                // Add a new endpoint record if the client connects with a `None` cookie
                if let Packet::Request { action, cookie, .. } = &packet {
                    if let RequestAction::Connect { .. } = action {
                        if cookie.is_none() {
                            valid_new_conn = true;

                            self.per_endpoint.insert(
                                endpoint,
                                FilterEndpointData::OtherEndClient(OtherEndClient {
                                    request_actions:              SequencedMinHeap::<RequestAction>::new(),
                                    last_request_sequence_seen:   None,
                                    last_response_sequence_sent:  None,
                                    last_request_seen_timestamp:  None,
                                    last_response_sent_timestamp: None,
                                    unacked_outgoing_packet_tids: VecDeque::new(),
                                }),
                            );
                        }
                    }
                }
            } else {
                // FilterMode::Client
                if self.ping_endpoints.contains_key(&endpoint) {
                    valid_new_conn = true;
                }
            }

            if !valid_new_conn {
                // The connection was not accepted for this new endpoint. No need to log it.
                return Ok(());
            }
        }

        match packet {
            Packet::Request {
                sequence,
                action,
                response_ack,
                ..
            } => {
                let client;
                let endpoint_data = self.per_endpoint.get_mut(&endpoint).unwrap();
                match endpoint_data {
                    FilterEndpointData::OtherEndServer { .. } => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "RequestAction".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndClient(other_end_client) => {
                        client = other_end_client;
                    }
                }
                client.last_request_seen_timestamp = Some(Instant::now());

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

                let mut tids_to_drop = vec![];
                if let Some(response_ack) = response_ack {
                    tids_to_drop = take_tids_to_drop(&mut client.unacked_outgoing_packet_tids, Wrapping(response_ack));
                }
                for tid_to_drop in tids_to_drop {
                    self.transport_cmd_tx
                        .send(TransportCmd::DropPacket {
                            endpoint,
                            tid: tid_to_drop,
                        })
                        .await?;
                }

                client.request_actions.add(sequence, action);

                // Loop over the heap, finding all requests which can be sent to the app layer based on their sequence number.
                // If any are found, send them to the app layer and advance the last seen sequence number.
                // TODO: unit test wrapping logic and deduplicate with below
                if client.last_request_sequence_seen.is_none() {
                    // Shouldn't be possible; if we hit this, it's a bug somewhere above
                    return Err(anyhow!(FilterEndpointDataError::InternalError {
                        problem: "sequence number should not be None at this point".to_owned(),
                    }));
                }
                let ref mut expected_seq_num = client
                    .last_request_sequence_seen
                    .expect("sequence number cannot be None by this point"); // expect OK because of above check
                while let Some(request_action) = client.request_actions.take_if_matching(expected_seq_num.0) {
                    filter_notice_tx
                        .send(FilterNotice::NewRequestAction {
                            endpoint,
                            action: request_action,
                        })
                        .await?;
                    *expected_seq_num += Wrapping(1);
                }
            }
            Packet::Response {
                sequence,
                request_ack,
                code,
            } => {
                let server;
                let endpoint_data = self.per_endpoint.get_mut(&endpoint).unwrap();
                match endpoint_data {
                    FilterEndpointData::OtherEndClient { .. } => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "ResponseCode".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndServer(other_end_server) => {
                        server = other_end_server;
                    }
                }
                server.last_response_seen_timestamp = Some(Instant::now());

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
                // TODO: unit test wrapping logic
                if server.last_response_sequence_seen.is_none() {
                    // Shouldn't be possible; if we hit this, it's a bug somewhere above
                    return Err(anyhow!(FilterEndpointDataError::InternalError {
                        problem: "sequence number should not be None at this point".to_owned(),
                    }));
                }
                let ref mut expected_seq_num = server
                    .last_response_sequence_seen
                    .expect("sequence number cannot be None by this point"); // expect OK because of above check
                while let Some(response_code) = server.response_codes.take_if_matching(expected_seq_num.0) {
                    filter_notice_tx
                        .send(FilterNotice::NewResponseCode {
                            endpoint,
                            code: response_code,
                        })
                        .await?;
                    *expected_seq_num += Wrapping(1);
                }
            }
            Packet::Status {
                player_count,
                ref pong,
                room_count,
                server_name,
                server_version,
            } => {
                if let Some((latency_filter, pingpong, opt_ping_tid)) = self.ping_endpoints.get_mut(&endpoint) {
                    // Update the round-trip time
                    if pingpong == pong {
                        latency_filter.update();
                    }

                    // Produce a latency after the filter has seen enough data
                    let mut latency = 0;
                    if let Some(average_latency_ms) = latency_filter.average_latency_ms {
                        latency = average_latency_ms;
                    }

                    // Tell the Transport layer to drop the ping packet
                    if let Some(tid) = opt_ping_tid {
                        self.transport_cmd_tx
                            .send(TransportCmd::DropPacket { endpoint, tid: *tid })
                            .await?;
                        *opt_ping_tid = None;
                    } else {
                        return Err(anyhow!(FilterEndpointDataError::InternalError {
                            problem: "ping tid is None on Pong. Should not be None at this point".to_owned(),
                        }));
                    }

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
            }
            // TODO: Add handling for Update and UpdateReply!!!!!!!
            _ => {}
        }

        return Ok(());
    }

    async fn process_filter_command(&mut self, command: FilterCmd) -> anyhow::Result<()> {
        let retry_interval = Duration::new(0, DEFAULT_RETRY_INTERVAL_NS);

        match command {
            FilterCmd::SendRequestAction { endpoint, action } => {
                check_endpoint_exists(&self.per_endpoint, endpoint).or_else(|err| match action {
                    RequestAction::Connect { .. } => {
                        self.per_endpoint.insert(
                            endpoint,
                            FilterEndpointData::OtherEndServer(OtherEndServer {
                                response_codes:               SequencedMinHeap::<ResponseCode>::new(),
                                last_request_sequence_sent:   None,
                                last_response_sequence_seen:  None,
                                last_request_sent_timestamp:  None,
                                last_response_seen_timestamp: None,
                                unacked_outgoing_packet_tids: VecDeque::new(),
                            }),
                        );
                        Ok(())
                    }
                    _ => Err(err),
                })?;

                let server;
                match self.per_endpoint.get_mut(&endpoint).unwrap() {
                    FilterEndpointData::OtherEndClient(..) => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "RequestActions are not sent to clients".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndServer(other_end_server) => {
                        server = other_end_server;
                    }
                }
                server.last_request_sent_timestamp = Some(Instant::now());

                if let Some(ref mut sn) = server.last_request_sequence_sent {
                    *sn += Wrapping(1u64);
                } else {
                    server.last_request_sequence_sent = Some(Wrapping(1));
                }

                // Unwrap ok b/c the immediate check above guarantees Some(..)
                let sequence = server.last_request_sequence_sent.unwrap().0;

                let response_ack = server.last_response_sequence_seen.map(|response_sn| response_sn.0);

                // TODO: Get cookie from app layer
                let cookie = None;

                let packets = vec![Packet::Request {
                    action,
                    cookie,
                    sequence,
                    response_ack,
                }];

                let tid = ProcessUniqueId::new();
                server.unacked_outgoing_packet_tids.push_back((Wrapping(sequence), tid));
                let packet_infos = vec![PacketSettings { tid, retry_interval }];

                self.transport_cmd_tx
                    .send(TransportCmd::SendPackets {
                        endpoint,
                        packet_infos,
                        packets,
                    })
                    .await?;
            }
            FilterCmd::SendResponseCode { endpoint, code } => {
                let client;
                match self.per_endpoint.get_mut(&endpoint).unwrap() {
                    FilterEndpointData::OtherEndServer { .. } => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "ResponseCodes are not sent to servers".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndClient(other_end_client) => {
                        client = other_end_client;
                    }
                }
                client.last_response_sent_timestamp = Some(Instant::now());

                if let Some(ref mut sn) = client.last_response_sequence_sent {
                    *sn += Wrapping(1u64);
                } else {
                    client.last_response_sequence_sent = Some(Wrapping(1));
                }

                // Unwrap ok b/c the immediate check above guarantees Some(..)
                let sequence = client.last_response_sequence_sent.unwrap().0;

                let request_ack = client.last_request_sequence_seen.map(|request_sn| request_sn.0);

                let packets = vec![Packet::Response {
                    code,
                    sequence,
                    request_ack,
                }];

                let tid = ProcessUniqueId::new();
                client.unacked_outgoing_packet_tids.push_back((Wrapping(sequence), tid));
                let packet_infos = vec![PacketSettings { tid, retry_interval }];

                self.transport_cmd_tx
                    .send(TransportCmd::SendPackets {
                        endpoint,
                        packet_infos,
                        packets,
                    })
                    .await?;
            }
            // TODO: implement these
            FilterCmd::SendChats { endpoints, messages } => {}
            FilterCmd::SendGameUpdates { endpoints, messages } => {}
            FilterCmd::Authenticated { endpoint } => {}
            FilterCmd::SendGenStateDiff { endpoints, diff } => {}
            FilterCmd::AddPingEndpoints { endpoints } => {
                for e in endpoints {
                    // An hashmap insert for an existing key will override the value. This would obsolete any ping
                    // already underway.
                    if self.ping_endpoints.contains_key(&e) {
                        continue;
                    }

                    // TIDs are assigned when the ping is sent to the transport layer
                    let opt_tid = None;
                    self.ping_endpoints
                        .insert(e, (LatencyFilter::new(), PingPong::ping(), opt_tid));
                }
            }
            FilterCmd::ClearPingEndpoints => {
                // Cancel any in progress pings
                for (endpoint, (_, _, opt_tid)) in self.ping_endpoints.iter() {
                    if let Some(tid) = opt_tid {
                        self.transport_cmd_tx
                            .send(TransportCmd::DropPacket {
                                endpoint: *endpoint,
                                tid:      *tid,
                            })
                            .await?;
                    }
                }
                self.ping_endpoints.clear();
            }
            FilterCmd::DropEndpoint { endpoint } => {}
            FilterCmd::Shutdown { graceful } => {
                return Err(anyhow!(FilterCommandError::ShutdownRequested { graceful }));
            }
        }

        Ok(())
    }

    pub fn get_shutdown_watcher(&mut self) -> impl Future<Output = ()> + 'static {
        let mut phase_watch_rx = self.phase_watch_rx.clone();
        let transport_cmd_tx = self.transport_cmd_tx.clone();
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
                    trace!("[FILTER] phase watch channel was dropped");
                    break;
                }
            }
            // Also shutdown the layer below
            let _ = transport_cmd_tx.send(TransportCmd::Shutdown).await;
        }
    }

    async fn send_pings(&mut self) -> anyhow::Result<()> {
        for (endpoint, (latency_filter, pingpong, opt_tid)) in self.ping_endpoints.iter_mut() {
            if opt_tid.is_some() {
                // There's an active ping in progress
                continue;
            }

            let tid = ProcessUniqueId::new();
            let pi = PacketSettings {
                retry_interval: Duration::ZERO,
                tid,
            };
            *opt_tid = Some(tid);

            self.transport_cmd_tx
                .send(TransportCmd::SendPackets {
                    endpoint:     *endpoint,
                    packet_infos: vec![pi],
                    packets:      vec![Packet::GetStatus { ping: *pingpong }],
                })
                .await?;
            latency_filter.start();
        }
        Ok(())
    }
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

/// Returns an error if the endpoint is not known to the filter
fn check_endpoint_exists(
    per_endpoint: &HashMap<Endpoint, FilterEndpointData>,
    endpoint: Endpoint,
) -> anyhow::Result<()> {
    if !per_endpoint.contains_key(&endpoint) {
        Err(anyhow!(FilterEndpointDataError::EndpointNotFound {
            endpoint: endpoint,
        }))
    } else {
        Ok(())
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
