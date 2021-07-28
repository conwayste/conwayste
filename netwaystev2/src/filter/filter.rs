use super::interface::{FilterMode, SeqNum};
use super::sortedbuffer::SequencedMinHeap;
use crate::protocol::{RequestAction, ResponseCode, Packet};
use crate::common::Endpoint;
use crate::transport::{
    TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp, TransportRspRecv,
};
use anyhow::anyhow;
use anyhow::Result;
use std::{collections::HashMap, num::Wrapping, time::Instant};

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
}

pub struct Filter {
    transport_cmd_tx:    TransportCmdSend,
    transport_rsp_rx:    Option<TransportRspRecv>,    // TODO no option
    transport_notice_rx: Option<TransportNotifyRecv>, // TODO no option
    mode:                FilterMode,
    per_endpoint:        HashMap<Endpoint, FilterEndpointData>,
}

impl Filter {
    pub fn new(
        transport_cmd_tx: TransportCmdSend,
        transport_rsp_rx: TransportRspRecv,
        transport_notice_rx: TransportNotifyRecv,
        mode: FilterMode,
    ) -> Self {
        let per_endpoint = HashMap::new();
        Filter {
            transport_cmd_tx,
            transport_rsp_rx: Some(transport_rsp_rx),
            transport_notice_rx: Some(transport_notice_rx),
            mode,
            per_endpoint,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let transport_cmd_tx = self.transport_cmd_tx.clone();
        let transport_rsp_rx = self.transport_rsp_rx.take().unwrap();
        let transport_notice_rx = self.transport_notice_rx.take().unwrap();
        tokio::pin!(transport_cmd_tx);
        tokio::pin!(transport_rsp_rx);
        tokio::pin!(transport_notice_rx);

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
                FilterEndpointData::OtherEndServer { .. } => {
                    return Err(anyhow!(FilterEndpointDataError::UnexpectedData {
                        mode:         self.mode,
                        invalid_data: "RequestAction".to_owned(),
                    }));
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
