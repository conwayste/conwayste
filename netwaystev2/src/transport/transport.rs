use super::endpoint::EndpointData;
use super::interface::{
    Packet,
    TransportCmd::{self, *},
    TransportNotice, TransportRsp,
};
use super::udp_codec::{LinesCodec, NetwaystePacketCodec};
use crate::common::Endpoint;
use crate::settings::*;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use futures::prelude::*;
use futures::stream::Fuse;
use futures::StreamExt;
use stream::{SplitSink, SplitStream};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time as TokioTime;
use tokio_stream::wrappers::IntervalStream;
use tokio_util::udp::UdpFramed;

type TransportCmdSend = Sender<TransportCmd>;
type TransportCmdRecv = Receiver<TransportCmd>;

type TransportRspSend = Sender<TransportRsp>;
type TransportRspRecv = Receiver<TransportRsp>;

type TransportNotifySend = Sender<TransportNotice>;
type TransportNotifyRecv = Receiver<TransportNotice>;

type TransportInit = (Transport, TransportCmdSend, TransportRspRecv, TransportNotifyRecv);

//type TransportItem = (Packet, SocketAddr);
type TransportItem = (String, SocketAddr);

pub struct Transport {
    requests:        TransportCmdRecv,
    responses:       TransportRspSend,
    notifications:   TransportNotifySend,
    //udp_stream_send: SplitSink<UdpFramed<NetwaystePacketCodec>, (Packet, SocketAddr)>,
    //udp_stream_recv: Fuse<SplitStream<UdpFramed<NetwaystePacketCodec>>>,
    udp_stream_send: SplitSink<UdpFramed<LinesCodec>, TransportItem>,
    udp_stream_recv: Fuse<SplitStream<UdpFramed<LinesCodec>>>,

    //endpoints: EndpointData<PacketInfo>,
    endpoints: EndpointData<String>,
}

impl Transport {
    pub fn new(opt_host: Option<&str>, opt_port: Option<u16>) -> Result<TransportInit> {
        // Bind socket to UDP
        let udp_socket = bind(opt_host, opt_port)?;

        // Split the socket into a two-part stream
        let udp_stream = UdpFramed::new(udp_socket, LinesCodec::new());
        let (udp_stream_send, udp_stream_recv) = udp_stream.split();
        let udp_stream_recv = udp_stream_recv.fuse();

        // Build the Transport
        let (cmd_tx, cmd_rx): (TransportCmdSend, TransportCmdRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
        let (rsp_tx, rsp_rx): (TransportRspSend, TransportRspRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);
        let (notice_tx, notice_rx): (TransportNotifySend, TransportNotifyRecv) = mpsc::channel(TRANSPORT_CHANNEL_LEN);

        Ok((
            Transport {
                requests: cmd_rx,
                responses: rsp_tx,
                notifications: notice_tx,
                udp_stream_send,
                udp_stream_recv,
                endpoints: EndpointData::new(),
            },
            cmd_tx,
            rsp_rx,
            notice_rx,
        ))
    }

    pub async fn monitor(&mut self) -> Result<()> {
        let udp_stream_recv = &mut self.udp_stream_recv;
        tokio::pin!(udp_stream_recv);

        let transmit_interval = TokioTime::interval(Duration::from_millis(10));
        let mut transmit_interval_stream = IntervalStream::new(transmit_interval).fuse();

        loop {
            tokio::select! {
                Some(cmd) = self.requests.recv() => {
                    trace!("Filter Request: {:?}", cmd);
                    let opt_response = process_transport_command(&mut self.endpoints, cmd);
                    if let Some(response) = opt_response {
                        self.responses.send(response).await?;
                    }
                }
                item_address_result = udp_stream_recv.select_next_some() => {
                    if let Ok((item, address)) = item_address_result {
                        trace!("LinesCodec data: {:?}", item);

                        if let Err(e) = self.endpoints.insert_receive_queue(Endpoint(address), item) {
                            warn!("{}", e);
                        } else {
                            self.notifications.send(TransportNotice::PacketsAvailable{
                                endpoint: Endpoint(address)
                            }).await?;
                        }
                    }
                }
                _ = transmit_interval_stream.select_next_some() => {
                    // Resend any packets in the transmit queue at their retry interval or send PacketTimeout
                    // XXX, for all end-points

                    // Check all end-points are still active or send EndpointTimeout
                    if let Ok(endpoints) = self.endpoints.timed_out_endpoints() {
                       for e in endpoints {
                           self.notifications.send(TransportNotice::EndpointTimeout {
                               endpoint: e
                           }).await?;
                       }
                    }
                }
            }
        }
    }
}

fn bind(opt_host: Option<&str>, opt_port: Option<u16>) -> Result<UdpSocket> {
    let host = if let Some(host) = opt_host { host } else { DEFAULT_HOST };
    let port = if let Some(port) = opt_port { port } else { DEFAULT_PORT };
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!("Attempting to bind to {}", addr);

    let sock_fut = UdpSocket::bind(&addr);
    let sock = futures::executor::block_on(sock_fut)?;

    Ok(sock)
}

fn process_transport_command(endpoints: &mut EndpointData<String>, command: TransportCmd) -> Option<TransportRsp> {
    match command {
        NewEndpoint { endpoint, timeout } => match endpoints.new_endpoint(endpoint, timeout) {
            Ok(()) => {
                return Some(TransportRsp::Accepted);
            }
            Err(e) => {
                error!("{}", e);
                return Some(TransportRsp::EndpointNotFound);
            }
        },
        GetQueueCount { endpoint, kind } => {
            if let Some(count) = endpoints.queue_count(endpoint, kind) {
                return Some(TransportRsp::QueueCount { endpoint, kind, count });
            } else {
                return Some(TransportRsp::EndpointNotFound);
            }
        }
        TakeReceivePackets { endpoint } => match endpoints.drain_receive_queue(endpoint) {
            Ok(packets) if !packets.is_empty() => {
                return Some(TransportRsp::TakenPackets { packets });
            }
            Ok(_) => {
                return None;
            }
            Err(e) => {
                error!("{}", e.to_string());
                return Some(TransportRsp::EndpointNotFound);
            }
        },
        SendPackets {
            endpoint,
            packet_infos,
            packets,
        } => {}
        DropEndpoint { endpoint } => {
            if let Ok(_) = endpoints.drop_endpoint(endpoint) {
                return Some(TransportRsp::Accepted);
            } else {
                return Some(TransportRsp::EndpointNotFound);
            }
        }
        CancelTransmitQueue { endpoint } => {}
    }
    None
}
