use anyhow::anyhow;
use std::collections::HashMap;
use super::{
    SequencedMinHeap,
    EndpointData,
    interface::{FilterMode, Packet, RequestAction},
};
use crate::common::Endpoint;
use crate::transport::{
    TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportRsp,
    TransportRspRecv,
};
use anyhow::Result;

pub struct Filter {
    transport_cmd_tx:    TransportCmdSend,
    transport_rsp_rx:    Option<TransportRspRecv>, // TODO no option
    transport_notice_rx: Option<TransportNotifyRecv>, // TODO no option
    mode:                FilterMode,
    per_endpoint: HashMap<Endpoint, EndpointData>,
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
                                if let Err(e) = self.process_incoming_packet(endpoint, packet) {
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

    fn process_incoming_packet(&mut self, endpoint: Endpoint, packet: Packet) -> anyhow::Result<()> {
        // TODO: also create endpoint data entry on a filter command to initiate a connection
        // (client mode only).
        if !self.per_endpoint.contains_key(&endpoint) {
            if self.mode == FilterMode::Server {
                self.per_endpoint.insert(endpoint, EndpointData::OtherEndClient{
                    request_actions: SequencedMinHeap::<RequestAction>::new(),
                });
            } else {
                // wrong but don't spam logs
                return Ok(())
            }
        }
        let endpoint_data = self.per_endpoint.get_mut(&endpoint).unwrap();
        match packet {
            Packet::Request{sequence, action, ..} => {
                match endpoint_data {
                    EndpointData::OtherEndClient{request_actions} => {
                        request_actions.add(sequence, action);
                    }
                    EndpointData::OtherEndServer{..} => {
                        return Err(anyhow!("wrong mode")); // TODO: thiserror
                    }
                }
            }
            Packet::Response{sequence, code, ..} => {
                match endpoint_data {
                    EndpointData::OtherEndClient{..} => {
                        return Err(anyhow!("wrong mode")); // TODO: thiserror
                    }
                    EndpointData::OtherEndServer{response_codes} => {
                        response_codes.add(sequence, code);
                    }
                }
            }
            _ => {} // TODO!!!!!!!!!!!!1
        }
        Ok(())
    }
}
