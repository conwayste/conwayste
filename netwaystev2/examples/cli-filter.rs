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

    transport_cmd_tx
        .send(TransportCmd::SendPackets {
            endpoint:     Endpoint("127.0.0.1:2017".parse().unwrap()),
            packets:      vec!["fire-metal".to_owned(), "fight".to_owned(), "frunk".to_owned()],
            packet_infos: (0..3)
                .map(|i| PacketInfo {
                    tid:            i,
                    retry_limit:    5,
                    retry_interval: Duration::new(10, 0),
                })
                .collect::<Vec<PacketInfo>>(),
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
                            transport_cmd_tx.send(TransportCmd::TakeReceivePackets{
                                endpoint,
                            }).await?;
                        }
                        TransportRsp::TakenPackets{packets} => {
                            for p in packets {
                                trace!("Took packet: {:?}", p);
                            }
                        }
                        TransportRsp::UnknownPacketTid => {
                            // XXX
                        }
                        TransportRsp::BufferFull => {
                            // XXX
                            error!("Transmit buffer is full");
                        }
                        TransportRsp::ExceedsMtu => {
                            // XXX
                            error!("Packet exceeds MTU size");
                        }
                        TransportRsp::EndpointNotFound => {
                            error!("Endpoint not found for previous Transport Command");
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
                            // XXX Take received packets
                            transport_cmd_tx.send(TransportCmd::GetQueueCount{
                                endpoint,
                                kind: TransportQueueKind::Receive
                            }).await?
                        }
                        TransportNotice::EndpointTimeout {
                            endpoint,
                        } => {
                            info!("Endpoint {:?} Timed-out. Dropping.", endpoint);
                            transport_cmd_tx.send(TransportCmd::DropEndpoint{endpoint}).await?;
                        }
                        TransportNotice::PacketTimeout {
                            endpoint,
                            tid,
                        } => {
                            // XXX drop the packet
                        }
                    }
                }
            }
        }
    }
}
