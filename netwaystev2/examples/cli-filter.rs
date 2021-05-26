extern crate color_backtrace;
extern crate env_logger;
#[macro_use]
extern crate log;

use netwaystev2::common::Endpoint;
use netwaystev2::filter::Filter;
use netwaystev2::transport::Transport;
use netwaystev2::transport::{PacketSettings, TransportCmd};

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

    let (mut transport, transport_cmd_tx, transport_rsp_rx, transport_notice_rx) = Transport::new(None, None)?;

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
                .map(|i| PacketSettings {
                    tid:            i,
                    retry_limit:    2,
                    retry_interval: Duration::new(1, 0),
                })
                .collect::<Vec<PacketSettings>>(),
            packets:      packets_to_send,
        })
        .await?;

    let mut filter = Filter::new(transport_cmd_tx, transport_rsp_rx, transport_notice_rx);

    tokio::spawn(async move { filter.run().await });
    info!("Filter initialized!");

    loop {}
}
