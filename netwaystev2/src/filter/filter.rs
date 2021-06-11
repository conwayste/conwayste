use super::interface::{FilterMode, Packet};
use super::sortedbuffer::SortedBuffer;
use crate::transport::{
    TransportCmd, TransportCmdSend, TransportNotice, TransportNotifyRecv, TransportQueueKind, TransportRsp,
    TransportRspRecv,
};
use anyhow::Result;

pub struct Filter {
    transport_cmd_tx:    TransportCmdSend,
    transport_rsp_rx:    TransportRspRecv,
    transport_notice_rx: TransportNotifyRecv,
    mode:                FilterMode,
}

impl Filter {
    pub fn new(
        transport_cmd_tx: TransportCmdSend,
        transport_rsp_rx: TransportRspRecv,
        transport_notice_rx: TransportNotifyRecv,
        mode: FilterMode,
    ) -> Self {
        Filter {
            transport_cmd_tx,
            transport_rsp_rx,
            transport_notice_rx,
            mode,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let transport_cmd_tx = &mut self.transport_cmd_tx;
        let transport_rsp_rx = &mut self.transport_rsp_rx;
        let transport_notice_rx = &mut self.transport_notice_rx;
        tokio::pin!(transport_cmd_tx);
        tokio::pin!(transport_rsp_rx);
        tokio::pin!(transport_notice_rx);

        let mut sorted_buffer = SortedBuffer::new(self.mode);

        loop {
            tokio::select! {
                response = transport_rsp_rx.recv() => {
                    // trace!("[FILTER] Transport Response: {:?}", response);

                    if let Some(response) = response {
                        match response {
                            TransportRsp::Accepted => {
                                trace!("[FILTER] Transport Command Accepted");
                            }
                            TransportRsp::QueueCount{endpoint, kind: _, count: _} => {
                                // XXX Take received packets
                                transport_cmd_tx.send(TransportCmd::TakeReceivePackets{
                                    endpoint,
                                }).await?;
                            }
                            TransportRsp::TakenPackets{packets} => {
                                for p in packets {
                                    trace!("[FILTER] Took packet: {:?}", p);
                                    sorted_buffer.incoming_push(p);
                                }
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
                            TransportNotice::PacketsAvailable {
                                endpoint,
                            } => {
                                info!("[FILTER] Packets Available for Endpoint {:?}.", endpoint);
                                transport_cmd_tx.send(TransportCmd::GetQueueCount{
                                    endpoint,
                                    kind: TransportQueueKind::Receive
                                }).await?
                            }
                            TransportNotice::EndpointTimeout {
                                endpoint,
                            } => {
                                info!("[FILTER] Endpoint {:?} timed-out. Dropping.", endpoint);
                                transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await?;
                            }
                            TransportNotice::PacketTimeout {
                                endpoint,
                                tid,
                            } => {
                                info!("[FILTER] Packet (tid = {}) timed-out for {:?}. Dropping.", tid, endpoint);
                                transport_cmd_tx.send(TransportCmd::DropPacket{endpoint, tid}).await?;
                            }
                        }
                    }
                }
            }
        }
    }
}
