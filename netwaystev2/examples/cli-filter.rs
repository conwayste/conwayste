extern crate color_backtrace;
extern crate env_logger;
#[macro_use]
extern crate log;

use netwaystev2::common::Endpoint;
use netwaystev2::filter::{Filter, FilterMode};
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

    let mut filter = Filter::new(
        transport_cmd_tx,
        transport_rsp_rx,
        transport_notice_rx,
        FilterMode::Server,
    );

    tokio::spawn(async move { filter.run().await });
    info!("Filter initialized!");

    loop {}
}
