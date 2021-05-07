extern crate color_backtrace;
extern crate env_logger;
#[macro_use]
extern crate log;

use netwaystev2::common::Endpoint;
use netwaystev2::transport::Transport;
use netwaystev2::transport::{PacketInfo, TransportCmd, TransportNotice, TransportQueueKind, TransportRsp};

use anyhow::Result;
use std::io::Write;
use std::time::Duration;

use chrono::Local;

#[tokio::main]
async fn main() -> Result<()> {
    color_backtrace::install();

    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{:5}] - {}",
                Local::now().format("%a %Y-%m-%d %H:%M:%S%.6f"),
                record.level(),
                record.args(),
            )
        })
        .filter_level(log::LevelFilter::max())
        .target(env_logger::Target::Stdout)
        .init();

    let (mut transport, transport_cmd_tx, mut transport_rsp_rx, mut transport_notice_rx) = Transport::new(None, None)?;

    tokio::spawn(async move { transport.run().await });
    info!("Transport initialized!");

    transport_cmd_tx
        .send(TransportCmd::NewEndpoint {
            endpoint: Endpoint("127.0.0.1:2017".parse().unwrap()),
            timeout:  Duration::new(5, 0), // PR_GATE make this configurable and reasonably valued
        })
        .await?;

    let packets_to_send = vec!["fire-metal".to_owned(), "fight".to_owned(), "frunk".to_owned()];
    transport_cmd_tx
        .send(TransportCmd::SendPackets {
            endpoint:     Endpoint("127.0.0.1:2017".parse().unwrap()),
            packet_infos: (0..packets_to_send.len())
                .map(|i| PacketInfo {
                    tid:            i,
                    retry_limit:    2,
                    retry_interval: Duration::new(1, 0),
                })
                .collect::<Vec<PacketInfo>>(),
            packets:      packets_to_send,
        })
        .await?;

    loop {
        tokio::select! {
            response = transport_rsp_rx.recv() => {
                // trace!("Transport Response: {:?}", response);

                if let Some(response) = response {
                    match response {
                        TransportRsp::Accepted => {
                            trace!("Transport Command Accepted");
                        }
                        TransportRsp::QueueCount{endpoint, kind: _, count: _} => {
                            // XXX Take received packets
                            transport_cmd_tx.send(TransportCmd::TakeReceivePackets{
                                endpoint,
                            }).await?;
                        }
                        TransportRsp::TakenPackets{packets} => {
                            for p in packets {
                                trace!("Took packet: {:?}", p);
                            }
                        }
                        TransportRsp::SendPacketsLengthMismatch => {
                            error!("Packet and PacketInfo data did not align")
                        }
                        TransportRsp::BufferFull => {
                            // XXX
                            error!("Transmit buffer is full");
                        }
                        TransportRsp::ExceedsMtu {tid} => {
                            // XXX
                            error!("Packet exceeds MTU size. Tid={}", tid);
                        }
                        TransportRsp::EndpointNotFound {endpoint} => {
                            error!("Endpoint not found for previous Transport Command: {:?}", endpoint);
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
                            info!("Packets Available for Endpoint {:?}.", endpoint);
                            transport_cmd_tx.send(TransportCmd::GetQueueCount{
                                endpoint,
                                kind: TransportQueueKind::Receive
                            }).await?
                        }
                        TransportNotice::EndpointTimeout {
                            endpoint,
                        } => {
                            info!("Endpoint {:?} timed-out. Dropping.", endpoint);
                            transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await?;
                        }
                        TransportNotice::PacketTimeout {
                            endpoint,
                            tid,
                        } => {
                            info!("Packet (tid = {}) timed-out for {:?}. Dropping.", tid, endpoint);
                            transport_cmd_tx.send(TransportCmd::DropPacket{endpoint, tid}).await?;
                        }
                    }
                }
            }
        }
    }
}
