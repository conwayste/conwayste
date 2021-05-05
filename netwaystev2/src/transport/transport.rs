use super::endpoint::EndpointData;
use super::interface::{
    Packet, PacketInfo,
    TransportCmd::{self, *},
    TransportNotice, TransportQueueKind, TransportRsp,
};
use super::udp_codec::{LinesCodec, NetwaystePacketCodec};
use crate::common::Endpoint;
use crate::settings::*;

use std::time::Duration;
use std::{net::SocketAddr, pin::Pin};

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

// https://serverfault.com/questions/645890/tcpdump-truncates-to-1472-bytes-useful-data-in-udp-packets-during-the-capture/645892#645892
const UDP_MTU_SIZE: usize = 1472;

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

    pub async fn run(&mut self) -> Result<()> {
        let udp_stream_recv = &mut self.udp_stream_recv;
        let udp_stream_send = &mut self.udp_stream_send;
        tokio::pin!(udp_stream_recv);
        tokio::pin!(udp_stream_send);

        let transmit_interval = TokioTime::interval(Duration::from_millis(10));
        let mut transmit_interval_stream = IntervalStream::new(transmit_interval).fuse();

        loop {
            tokio::select! {
                Some(cmd) = self.requests.recv() => {
                    trace!("Filter Request: {:?}", cmd);
                    for response in process_transport_command(&mut self.endpoints, cmd, &mut udp_stream_send).await {
                        self.responses.send(response).await?;
                    }
                }
                item_address_result = udp_stream_recv.select_next_some() => {
                    if let Ok((item, address)) = item_address_result {
                        trace!("LinesCodec data: {:?}", item);

                        if let Err(e) = self.endpoints.push_receive_queue(Endpoint(address), item) {
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
                    let (retry_packets, packet_timeouts) = self.endpoints.separate_into_retriable_and_timed_out();

                    for (data_ref, endpoint) in retry_packets {
                        udp_stream_send.send((data_ref.to_owned(), endpoint.0)).await?;
                    }

                    for (tid, endpoint) in packet_timeouts {
                        self.notifications.send(TransportNotice::PacketTimeout {
                            endpoint, tid
                        }).await?;
                    }

                    // Notify filter of any endpoints that have timed-out
                    for endpoint in  self.endpoints.timed_out_endpoints() {
                        self.notifications.send(TransportNotice::EndpointTimeout {
                            endpoint
                        }).await?;
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

async fn process_transport_command(
    endpoints: &mut EndpointData<String>,
    command: TransportCmd,
    udp_send: &mut Pin<&mut &mut SplitSink<UdpFramed<LinesCodec>, (std::string::String, std::net::SocketAddr)>>,
) -> Vec<TransportRsp> {
    match command {
        NewEndpoint { endpoint, timeout } => match endpoints.new_endpoint(endpoint, timeout) {
            Ok(()) => {
                return vec![TransportRsp::Accepted];
            }
            Err(e) => {
                error!("{}", e);
                return vec![TransportRsp::EndpointNotFound { endpoint }];
            }
        },
        GetQueueCount { endpoint, kind } => {
            if let Some(count) = endpoints.queue_count(endpoint, kind) {
                return vec![TransportRsp::QueueCount { endpoint, kind, count }];
            } else {
                return vec![TransportRsp::EndpointNotFound { endpoint }];
            }
        }
        TakeReceivePackets { endpoint } => match endpoints.drain_receive_queue(endpoint) {
            Ok(packets) if !packets.is_empty() => {
                return vec![TransportRsp::TakenPackets { packets }];
            }
            Ok(_) => {
                return vec![];
            }
            Err(e) => {
                error!("{}", e.to_string());
                return vec![TransportRsp::EndpointNotFound { endpoint }];
            }
        },
        SendPackets {
            endpoint,
            packet_infos,
            packets,
        } => {
            if packets.len() != packet_infos.len() {
                return vec![TransportRsp::SendPacketsLengthMismatch];
            }

            let mut errors = vec![];

            for (i, p) in packets.iter().enumerate() {
                let pi = packet_infos.get(i).unwrap(); // Unwrap safe b/c of length check above

                if std::mem::size_of_val(p) < UDP_MTU_SIZE {
                    let _result = udp_send.send((p.clone(), endpoint.0)).await.and_then(|_| {
                        match endpoints.push_transmit_queue(
                            endpoint,
                            pi.tid,
                            p.to_owned(),
                            pi.retry_interval,
                            pi.retry_limit,
                        ) {
                            Ok(()) => {}
                            Err(e) => {
                                error!("{}", e.to_string());
                                errors.push(TransportRsp::EndpointNotFound { endpoint })
                            }
                        }
                        Ok(())
                    });
                } else {
                    errors.push(TransportRsp::ExceedsMtu { tid: pi.tid });
                }
            }

            if errors.is_empty() {
                return vec![TransportRsp::Accepted];
            } else {
                return errors;
            }
        }
        DropEndpoint { endpoint } => match endpoints.drop_endpoint(endpoint) {
            Ok(()) => return vec![TransportRsp::Accepted],
            Err(e) => {
                error!("{}", e.to_string());
                return vec![TransportRsp::EndpointNotFound { endpoint }];
            }
        },
        CancelTransmitQueue { endpoint } => match endpoints.clear_queue(endpoint, TransportQueueKind::Transmit) {
            Ok(()) => return vec![TransportRsp::Accepted],
            Err(e) => {
                error!("{}", e.to_string());
                return vec![TransportRsp::EndpointNotFound { endpoint }];
            }
        },
    }
}
