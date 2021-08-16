use super::interface::{FilterCmd, FilterMode, FilterNotice, FilterRsp, SeqNum};
use super::sortedbuffer::SequencedMinHeap;
use crate::common::Endpoint;
use crate::protocol::{Packet, RequestAction, ResponseCode};
use crate::settings::FILTER_CHANNEL_LEN;
use crate::transport::{
    TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp, TransportRspRecv,
};
use anyhow::anyhow;
use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::watch;

use std::{collections::HashMap, future::Future, num::Wrapping, time::Instant};

enum SeqNumAdvancement {
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
    #[error("Filter is shutting down")]
    ShutdownRequested,
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
    ShutdownRequested,
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
    phase_watch_rx:      watch::Receiver<Phase>, // XXX gets cloned
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

    pub async fn run(&mut self) -> Result<()> {
        let transport_cmd_tx = self.transport_cmd_tx.clone();
        let transport_rsp_rx = self.transport_rsp_rx.take().unwrap();
        let transport_notice_rx = self.transport_notice_rx.take().unwrap();
        let mut phase = Phase::Running;
        let mut phase_watch_tx = self.phase_watch_tx.take().unwrap();
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
                                if let Err(e) = self.process_transport_packet(endpoint, packet) {
                                    error!("[FILTER] error processing incoming packet: {:?}", e);
                                }
                            }
                            TransportNotice::EndpointTimeout {
                                endpoint,
                            } => {
                                info!("[FILTER] Endpoint {:?} timed-out. Dropping.", endpoint);
                                self.per_endpoint.remove(&endpoint);
                                transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await?;
                            }
                        }
                    }
                }
                command = filter_cmd_rx.recv() => {
                    if let Some(command) = command {
                        trace!("[FILTER] New command: {:?}", command);

                        if let Err(e) = self.process_filter_command(command) {
                            if let Some(err) = e.downcast_ref::<FilterCommandError>() {
                                match err {
                                    FilterCommandError::ShutdownRequested => {
                                        info!("[FILTER] shutting down");
                                        phase = Phase::ShutdownComplete; // TODO: can do ShutdownRequested if it's a graceful shutdown
                                        phase_watch_tx.send(phase).unwrap();
                                        return Ok(());
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

    fn process_transport_packet(&mut self, endpoint: Endpoint, packet: Packet) -> anyhow::Result<()> {
        // TODO: also create endpoint data entry on a filter command to initiate a connection
        // (client mode only).
        if !self.per_endpoint.contains_key(&endpoint) {
            if self.mode == FilterMode::Server {
                if let Packet::Request { .. } = packet {
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
                } else {
                    // wrong but don't spam logs
                    return Ok(());
                }
            } else {
                // wrong but don't spam logs
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

                    match advance_sequence_number(sequence, last_request_sequence_seen) {
                        SeqNumAdvancement::Duplicate => {
                            return Err(anyhow!(FilterEndpointDataError::DuplicateRequest {
                                sequence: sequence,
                            }));
                        }
                        _ => {
                            request_actions.add(sequence, action);
                        }
                    }

                    // Determine how many contiguous requests are available to send to the app layer
                    let mut taken = false;
                    loop {
                        if let Some(sn) = request_actions.peek_sequence_number() {
                            if let Some(ref mut lrsn) = last_request_sequence_seen {
                                if lrsn.0 == sn {
                                    /* TODO: Send to application layer */
                                    // NewRequestAction(endpoint, request_actions.take())
                                    *lrsn += Wrapping(1);
                                    taken = true;
                                }
                            }
                        }
                        if !taken {
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

                    match advance_sequence_number(sequence, last_response_sequence_seen) {
                        SeqNumAdvancement::Duplicate => {
                            return Err(anyhow!(FilterEndpointDataError::DuplicateResponse {
                                sequence: sequence,
                            }));
                        }
                        _ => {
                            response_codes.add(sequence, code);
                        }
                    }

                    // Determine how many contiguous responses are available to send to the app layer
                    let mut taken = false;
                    loop {
                        if let Some(sn) = response_codes.peek_sequence_number() {
                            if let Some(ref mut lrsn) = last_response_sequence_seen {
                                if lrsn.0 == sn {
                                    /* TODO: Send to application layer */
                                    // NewRequestAction(endpoint, request_actions.take())
                                    *lrsn += Wrapping(1);
                                    taken = true;
                                }
                            }
                        }
                        if !taken {
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

    fn process_filter_command(&mut self, command: FilterCmd) -> anyhow::Result<()> {
        match command {
            FilterCmd::SendRequestAction { endpoint, action } => {
                // PR_GATE: This will currently fail because the endpoint has not been created on an connect() event
                check_endpoint_exists(&self.per_endpoint, endpoint)?;
            }
            FilterCmd::SendResponseCode { endpoint, code } => {}
            FilterCmd::SendChats { endpoints, messages } => {}
            FilterCmd::SendGameUpdates { endpoints, messages } => {}
            FilterCmd::Authenticated { endpoint } => {}
            FilterCmd::SendGenStateDiff { endpoints, diff } => {}
            FilterCmd::AddPingEndpoints { endpoints } => {}
            FilterCmd::ClearPingEndpoints => {}
            FilterCmd::DropEndpoint { endpoint } => {}
            FilterCmd::Shutdown { graceful } => {
                // TODO: graceful
                return Err(anyhow!(FilterCommandError::ShutdownRequested));
            }
        }

        Ok(())
    }

    pub fn get_shutdown_watcher(&mut self) -> impl Future<Output=()> + 'static {
        let mut phase_watch_rx = self.phase_watch_rx.clone();
        async move {
            loop {
                let phase = *phase_watch_rx.borrow();
                match phase {
                    Phase::ShutdownComplete => {
                        return;
                    }
                    _ => {}
                }
                if phase_watch_rx.changed().await.is_err() {
                    // channel closed
                    trace!("[FILTER] phase watch channel was dropped");
                    return;
                }
            }
        }
    }
}

/// True if the sequence number is contiguous to the previously seen value
/// False if the sequence number is out-of-order
fn advance_sequence_number(sequence: u64, last_seen: &mut Option<SeqNum>) -> SeqNumAdvancement {
    // Buffer the packet if it is received out-of-order, otherwise send it up to the app layer directly
    // for immediate processing
    if let Some(last_sn) = last_seen {
        let sequence_wrapped = Wrapping(sequence);

        // TODO: Need to handle wrapping cases for when comparing sequence numbers on both sides 0

        if sequence_wrapped == *last_sn + Wrapping(1) {
            *last_seen = Some(sequence_wrapped);
            return SeqNumAdvancement::Contiguous;
        } else if sequence_wrapped <= *last_sn {
            return SeqNumAdvancement::Duplicate;
        } else {
            return SeqNumAdvancement::OutOfOrder;
        }
    } else {
        *last_seen = Some(Wrapping(sequence));
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
