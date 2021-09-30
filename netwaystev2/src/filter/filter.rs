use super::interface::{FilterCmd, FilterMode, FilterNotice, FilterRsp, SeqNum};
use super::sortedbuffer::SequencedMinHeap;
use crate::common::Endpoint;
use crate::protocol::{Packet, RequestAction, ResponseCode};
use crate::settings::{DEFAULT_RETRY_INTERVAL_US, FILTER_CHANNEL_LEN};
use crate::transport::{
    PacketSettings, TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp,
    TransportRspRecv,
};
use anyhow::anyhow;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::watch;

use std::time::Duration;
use std::{collections::HashMap, future::Future, num::Wrapping, time::Instant};

#[derive(PartialEq, Debug)]
pub(crate) enum SeqNumAdvancement {
    BrandNew,
    Contiguous,
    OutOfOrder,
    Duplicate,
}

pub enum FilterEndpointData {
    OtherEndClient {
        request_actions:              SequencedMinHeap<RequestAction>,
        last_request_sequence_seen:   Option<SeqNum>,
        last_response_sequence_sent:  Option<SeqNum>,
        last_request_seen_timestamp:  Option<Instant>,
        last_response_sent_timestamp: Option<Instant>,
    },
    OtherEndServer {
        response_codes:               SequencedMinHeap<ResponseCode>,
        last_request_sequence_sent:   Option<SeqNum>,
        last_response_sequence_seen:  Option<SeqNum>,
        last_request_sent_timestamp:  Option<Instant>,
        last_response_seen_timestamp: Option<Instant>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FilterEndpointDataError {
    #[error("Filter mode ({mode:?}) is not configured to receive {invalid_data}")]
    UnexpectedData {
        mode:         FilterMode,
        invalid_data: String,
    },
    #[error("Filter observed duplicate or already processed request action: {sequence}")]
    DuplicateRequest { sequence: u64 },
    #[error("Filter observed duplicate or already process response code : {sequence}")]
    DuplicateResponse { sequence: u64 },
    #[error("Filter does not contain an entry for the endpoint: {endpoint:?}")]
    EndpointNotFound { endpoint: Endpoint },
}

#[derive(Debug, thiserror::Error)]
pub enum FilterCommandError {
    #[error("Filter is shutting down. Graceful: {graceful}")]
    ShutdownRequested{graceful: bool},
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
        let filter_rsp_tx = self.filter_rsp_tx.clone();
        let filter_notice_tx = self.filter_notice_tx.clone();
        tokio::pin!(filter_cmd_rx);
        tokio::pin!(filter_rsp_tx);
        tokio::pin!(filter_notice_tx);

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
                                FilterEndpointData::OtherEndClient {
                                    request_actions:              SequencedMinHeap::<RequestAction>::new(),
                                    last_request_sequence_seen:   None,
                                    last_response_sequence_sent:  None,
                                    last_request_seen_timestamp:  None,
                                    last_response_sent_timestamp: None,
                                },
                            );
                        }
                    }
                }
            }

            if !valid_new_conn {
                // The connection was not accepted for this new endpoint. No need to log it.
                return Ok(());
            }
        }

        let endpoint_data = self.per_endpoint.get_mut(&endpoint).unwrap();
        match packet {
            Packet::Request { sequence, action, .. } => match endpoint_data {
                FilterEndpointData::OtherEndServer { .. } => {
                    return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                        mode:         self.mode,
                        invalid_data: "RequestAction".to_owned(),
                    }));
                }
                FilterEndpointData::OtherEndClient {
                    request_actions,
                    last_request_sequence_seen,
                    last_request_seen_timestamp,
                    ..
                } => {
                    *last_request_seen_timestamp = Some(Instant::now());

                    match determine_seq_num_advancement(sequence, last_request_sequence_seen) {
                        SeqNumAdvancement::Duplicate => {
                            return Err(anyhow!(FilterEndpointDataError::DuplicateRequest {
                                sequence: sequence,
                            }));
                        }
                        SeqNumAdvancement::BrandNew | SeqNumAdvancement::Contiguous => {
                            *last_request_sequence_seen = Some(Wrapping(sequence));
                        }
                        SeqNumAdvancement::OutOfOrder => {
                            // Nothing to do but add it to the heap in the next step
                        }
                    }

                    request_actions.add(sequence, action);

                    // Loop over the heap, finding all requests which can be sent to the app layer based on their sequence number.
                    // If any are found, send them to the app layer and advance the last seen sequence number.
                    let ref mut last_seen_sn =
                        last_request_sequence_seen.expect("sequence number cannot be None by this point");
                    while let Some(sn) = request_actions.peek_sequence_number() {
                        if last_seen_sn.0 == sn {
                            // Unwrap okay because peeking provides us with a Some(sequence_number)
                            filter_notice_tx
                                .send(FilterNotice::NewRequestAction {
                                    endpoint,
                                    action: request_actions.take().unwrap(),
                                })
                                .await?;
                            *last_seen_sn += Wrapping(1);
                        } else {
                            break;
                        }
                    }
                }
            },
            Packet::Response { sequence, code, .. } => match endpoint_data {
                FilterEndpointData::OtherEndClient { .. } => {
                    return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                        mode:         self.mode,
                        invalid_data: "ResponseCode".to_owned(),
                    }));
                }
                FilterEndpointData::OtherEndServer {
                    response_codes,
                    last_response_sequence_seen,
                    last_response_seen_timestamp,
                    ..
                } => {
                    *last_response_seen_timestamp = Some(Instant::now());

                    match determine_seq_num_advancement(sequence, last_response_sequence_seen) {
                        SeqNumAdvancement::Duplicate => {
                            return Err(anyhow!(FilterEndpointDataError::DuplicateResponse {
                                sequence: sequence,
                            }));
                        }
                        SeqNumAdvancement::BrandNew | SeqNumAdvancement::Contiguous => {
                            *last_response_sequence_seen = Some(Wrapping(sequence));
                        }
                        SeqNumAdvancement::OutOfOrder => {
                            // Nothing to do but add it to the heap in the next step
                        }
                    }

                    response_codes.add(sequence, code);

                    // Loop over the heap, finding all responses which can be sent to the app layer based on their sequence number.
                    // If any are found, send them to the app layer and advance the last seen sequence number.
                    let ref mut last_seen_sn =
                        last_response_sequence_seen.expect("sequence number cannot be None by this point");
                    loop {
                        if let Some(sn) = response_codes.peek_sequence_number() {
                            if last_seen_sn.0 == sn {
                                // Unwrap okay because peeking provides us with a Some(sequence_number)
                                filter_notice_tx
                                    .send(FilterNotice::NewResponseCode {
                                        endpoint,
                                        code: response_codes.take().unwrap(),
                                    })
                                    .await?;
                                *last_seen_sn += Wrapping(1);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            },
            // TODO: Add handling for Update and UpdateReply!!!!!!!
            _ => {}
        }

        return Ok(());
    }

    async fn process_filter_command(&mut self, command: FilterCmd) -> anyhow::Result<()> {
        let retry_interval = Duration::new(0, DEFAULT_RETRY_INTERVAL_US);

        match command {
            FilterCmd::SendRequestAction { endpoint, action } => {
                // PR_GATE: This will currently fail because the endpoint has not been created on an connect() event
                check_endpoint_exists(&self.per_endpoint, endpoint)?;

                match self.per_endpoint.get_mut(&endpoint).unwrap() {
                    FilterEndpointData::OtherEndClient { .. } => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "RequestActions are not sent to clients".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndServer {
                        last_request_sequence_sent,
                        last_request_sent_timestamp,
                        last_response_sequence_seen,
                        ..
                    } => {
                        *last_request_sent_timestamp = Some(Instant::now());

                        if let Some(sn) = last_request_sequence_sent {
                            *sn += Wrapping(1u64);
                        } else {
                            *last_request_sequence_sent = Some(Wrapping(1));
                        }

                        // Unwrap ok b/c the immediate check above guarantees Some(..)
                        let sequence = last_request_sequence_sent.unwrap().0;

                        let response_ack = last_response_sequence_seen.map(|response_sn| response_sn.0);

                        // TODO: Get cookie from app layer
                        let cookie = None;

                        let packets = vec![Packet::Request {
                            action,
                            cookie,
                            sequence,
                            response_ack,
                        }];

                        let packet_infos = vec![PacketSettings {
                            tid: ProcessUniqueId::new(),
                            retry_interval,
                        }];

                        self.transport_cmd_tx
                            .send(TransportCmd::SendPackets {
                                endpoint,
                                packet_infos,
                                packets,
                            })
                            .await?;
                    }
                }
            }
            FilterCmd::SendResponseCode { endpoint, code } => {
                match self.per_endpoint.get_mut(&endpoint).unwrap() {
                    FilterEndpointData::OtherEndServer { .. } => {
                        return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                            mode:         self.mode,
                            invalid_data: "ResponseCodes are not sent to servers".to_owned(),
                        }));
                    }
                    FilterEndpointData::OtherEndClient {
                        last_response_sequence_sent,
                        last_response_sent_timestamp,
                        last_request_sequence_seen,
                        ..
                    } => {
                        *last_response_sent_timestamp = Some(Instant::now());

                        if let Some(sn) = last_response_sequence_sent {
                            *sn += Wrapping(1u64);
                        } else {
                            *last_response_sequence_sent = Some(Wrapping(1));
                        }

                        // Unwrap ok b/c the immediate check above guarantees Some(..)
                        let sequence = last_response_sequence_sent.unwrap().0;

                        let request_ack = last_request_sequence_seen.map(|request_sn| request_sn.0);

                        let packets = vec![Packet::Response {
                            code,
                            sequence,
                            request_ack,
                        }];

                        let packet_infos = vec![PacketSettings {
                            tid: ProcessUniqueId::new(),
                            retry_interval,
                        }];

                        self.transport_cmd_tx
                            .send(TransportCmd::SendPackets {
                                endpoint,
                                packet_infos,
                                packets,
                            })
                            .await?;
                    }
                }
            }
            FilterCmd::SendChats { endpoints, messages } => {}
            FilterCmd::SendGameUpdates { endpoints, messages } => {}
            FilterCmd::Authenticated { endpoint } => {}
            FilterCmd::SendGenStateDiff { endpoints, diff } => {}
            FilterCmd::AddPingEndpoints { endpoints } => {}
            FilterCmd::ClearPingEndpoints => {}
            FilterCmd::DropEndpoint { endpoint } => {}
            FilterCmd::Shutdown { graceful } => {
                return Err(anyhow!(FilterCommandError::ShutdownRequested{graceful}));
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
pub(crate) fn determine_seq_num_advancement(pkt_sequence: u64, last_seen_sn: &mut Option<SeqNum>) -> SeqNumAdvancement {
    if let Some(last_sn) = last_seen_sn {
        let sequence_wrapped = Wrapping(pkt_sequence);

        if sequence_wrapped == *last_sn + Wrapping(1) {
            return SeqNumAdvancement::Contiguous;
        } else if sequence_wrapped <= *last_sn {
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
