use super::endpoint::TransportEndpointData;
use super::interface::{
    TransportCmd::{self, *},
    TransportNotice, TransportRsp, UDP_MTU_SIZE,
};
use super::udp_codec::NetwaystePacketCodec;
use crate::common::{Endpoint, ShutdownWatcher};
use crate::protocol::Packet;
use crate::settings::*;

use std::sync::Arc;
use std::time::Duration;
use std::{net::SocketAddr, pin::Pin};

use anyhow::anyhow;
use anyhow::Result;
use futures::prelude::*;
use futures::stream::Fuse;
use futures::StreamExt;
use stream::{SplitSink, SplitStream};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::watch;
use tokio::time as TokioTime;
use tokio_stream::wrappers::IntervalStream;
use tokio_util::udp::UdpFramed;

#[derive(Debug, thiserror::Error)]
pub enum TransportCommandError {
    #[error("Transport is shutting down")]
    ShutdownRequested,
}

pub type TransportCmdSend = Sender<TransportCmd>;
pub type TransportCmdRecv = Receiver<TransportCmd>;

type TransportRspSend = Sender<TransportRsp>;
pub type TransportRspRecv = Receiver<TransportRsp>;

pub type TransportNotifySend = Sender<TransportNotice>;
pub type TransportNotifyRecv = Receiver<TransportNotice>;

pub type TransportInit = (Transport, TransportCmdSend, TransportRspRecv, TransportNotifyRecv);

type TransportItem = (Packet, SocketAddr);

#[derive(Copy, Clone, Debug)]
enum Phase {
    Running,
    ShutdownComplete,
}

pub struct Transport {
    requests:        TransportCmdRecv,
    responses:       TransportRspSend,
    notifications:   TransportNotifySend,
    udp_stream_send: SplitSink<UdpFramed<NetwaystePacketCodec>, TransportItem>,
    udp_stream_recv: Fuse<SplitStream<UdpFramed<NetwaystePacketCodec>>>,
    phase_watch_tx:  Option<watch::Sender<Phase>>, // Temp. holding place. This is only Some(...) between new() and run() calls
    phase_watch_rx:  watch::Receiver<Phase>,

    endpoints: TransportEndpointData<Packet>,
}

impl Transport {
    pub async fn new(opt_host: Option<String>, opt_port: Option<u16>) -> Result<TransportInit> {
        // Bind socket to UDP
        let udp_socket = bind(opt_host, opt_port).await?;

        // Split the socket into a two-part stream
        let udp_stream = UdpFramed::new(udp_socket, NetwaystePacketCodec);
        let (udp_stream_send, udp_stream_recv) = udp_stream.split();
        let udp_stream_recv = udp_stream_recv.fuse();

        // Build the Transport
        let (cmd_tx, cmd_rx): (TransportCmdSend, TransportCmdRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
        let (rsp_tx, rsp_rx): (TransportRspSend, TransportRspRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
        let (notice_tx, notice_rx): (TransportNotifySend, TransportNotifyRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);

        let (phase_watch_tx, phase_watch_rx) = watch::channel(Phase::Running);

        Ok((
            Transport {
                requests: cmd_rx,
                responses: rsp_tx,
                notifications: notice_tx,
                udp_stream_send,
                udp_stream_recv,
                phase_watch_tx: Some(phase_watch_tx),
                phase_watch_rx,
                endpoints: TransportEndpointData::new(),
            },
            cmd_tx,
            rsp_rx,
            notice_rx,
        ))
    }

    pub async fn run(&mut self) -> Result<()> {
        let udp_stream_recv = &mut self.udp_stream_recv;
        let udp_stream_send = &mut self.udp_stream_send;
        let phase_watch_tx = self.phase_watch_tx.take().unwrap();
        tokio::pin!(udp_stream_recv);
        tokio::pin!(udp_stream_send);

        let transmit_interval = TokioTime::interval(Duration::from_millis(10));
        let mut transmit_interval_stream = IntervalStream::new(transmit_interval).fuse();

        loop {
            tokio::select! {
                Some(cmd) = self.requests.recv() => {
                    trace!("[TRANSPORT] Filter Request: {:?}", cmd);
                    match process_transport_command(&mut self.endpoints, cmd, &mut udp_stream_send).await {
                        Ok(responses) => {
                            for response in responses {
                                self.responses.send(response).await?;
                            }
                        }
                        Err(e) => {
                            if let Some(err) = e.downcast_ref::<TransportCommandError>() {
                                match err {
                                    TransportCommandError::ShutdownRequested => {
                                        info!("[TRANSPORT] shutting down");
                                        let phase = Phase::ShutdownComplete;
                                        phase_watch_tx.send(phase).unwrap();
                                        return Ok(());
                                    }
                                }
                            }
                            error!("[TRANSPORT] Transport command processing failed: {}", e);
                            return Ok(());
                        }
                    }
                }
                item_address_result = udp_stream_recv.select_next_some() => {
                    if let Ok((item, address)) = item_address_result {
                        trace!("[TRANSPORT] UDP Codec data: {:?}", item);

                        if let Err(e) = self.endpoints.update_last_received(Endpoint(address)) {
                            warn!("[TRANSPORT] {}", e);
                        } else {
                            self.notifications.send(TransportNotice::PacketDelivery{
                                endpoint: Endpoint(address),
                                packet: item,
                            }).await?;
                        }
                    }
                }
                _ = transmit_interval_stream.select_next_some() => {
                    // Resend any packets in the transmit queue at their retry interval or send PacketTimeout
                    let retry_packets = self.endpoints.retriable_packets();

                    for (packet_ref, endpoint) in retry_packets {
                        udp_stream_send.send((packet_ref.to_owned(), endpoint.0)).await?;
                    }

                    // Notify filter of any endpoints that have timed-out
                    for endpoint in self.endpoints.timed_out_endpoints_needing_notify() {
                        // FIXME: need to limit how often this is sent to avoid spamming Filter layer
                        self.notifications.send(TransportNotice::EndpointTimeout {
                            endpoint
                        }).await?;
                        self.endpoints.mark_endpoint_as_timeout_notified(endpoint);
                    }
                }
            }
        }
    }

    pub fn get_shutdown_watcher(&mut self) -> ShutdownWatcher {
        let mut phase_watch_rx = self.phase_watch_rx.clone();
        Box::pin(async move {
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
                    trace!("[TRANSPORT] phase watch channel was dropped");
                    return;
                }
            }
        })
    }
}

async fn bind(opt_host: Option<String>, opt_port: Option<u16>) -> Result<UdpSocket> {
    let host = if let Some(host) = opt_host {
        host
    } else {
        DEFAULT_HOST.to_owned()
    };
    let port = if let Some(port) = opt_port { port } else { DEFAULT_PORT };
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!("[TRANSPORT] Attempting to bind to {}", addr);

    let sock = UdpSocket::bind(&addr).await?;

    Ok(sock)
}

async fn process_transport_command(
    endpoints: &mut TransportEndpointData<Packet>,
    command: TransportCmd,
    udp_send: &mut Pin<&mut &mut SplitSink<UdpFramed<NetwaystePacketCodec>, (Packet, std::net::SocketAddr)>>,
) -> anyhow::Result<Vec<TransportRsp>> {
    let mut cmd_responses = vec![];
    match command {
        NewEndpoint { endpoint, timeout } => cmd_responses.push(endpoints.new_endpoint(endpoint, timeout).map_or_else(
            |error| TransportRsp::EndpointError { error: Arc::new(error) },
            |()| TransportRsp::Accepted,
        )),
        SendPackets {
            endpoint,
            packet_infos,
            packets,
        } => {
            if packets.len() != packet_infos.len() {
                return Ok(vec![TransportRsp::SendPacketsLengthMismatch]);
            }
            for (i, p) in packets.iter().enumerate() {
                let pi = packet_infos.get(i).unwrap(); // Unwrap safe b/c of length check above

                let size = match bincode::serialized_size(&p) {
                    Ok(size) => size as usize,
                    Err(error) => {
                        cmd_responses.push(TransportRsp::EndpointError {
                            error: Arc::new(error.into()),
                        });
                        continue;
                    }
                };
                if size > UDP_MTU_SIZE {
                    cmd_responses.push(TransportRsp::ExceedsMtu {
                        tid: pi.tid,
                        size,
                        mtu: UDP_MTU_SIZE,
                    });
                    continue;
                }
                if let Err(error) = udp_send.send((p.clone(), endpoint.0)).await {
                    continue;
                }
                cmd_responses.push(
                    endpoints
                        .push_transmit_queue(endpoint, pi.tid, p.to_owned(), pi.retry_interval)
                        .map_or_else(
                            |error| TransportRsp::EndpointError { error: Arc::new(error) },
                            |()| TransportRsp::Accepted,
                        ),
                );
            }
        }
        DropEndpoint { endpoint } => cmd_responses.push(endpoints.drop_endpoint(endpoint).map_or_else(
            |error| TransportRsp::EndpointError { error: Arc::new(error) },
            |()| TransportRsp::Accepted,
        )),
        DropPacket { endpoint, tid } => cmd_responses.push(endpoints.drop_packet(endpoint, tid).map_or_else(
            |error| TransportRsp::EndpointError { error: Arc::new(error) },
            |()| TransportRsp::Accepted,
        )),
        CancelTransmitQueue { endpoint } => cmd_responses.push(endpoints.clear_queue(endpoint).map_or_else(
            |error| TransportRsp::EndpointError { error: Arc::new(error) },
            |()| TransportRsp::Accepted,
        )),
        Shutdown => return Err(anyhow!(TransportCommandError::ShutdownRequested)),
    }

    Ok(cmd_responses)
}
