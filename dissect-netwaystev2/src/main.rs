use etherparse::{SlicedPacket, TransportSlice::Udp};
use netwaystev2::DEFAULT_PORT;
use pcap;
use tracing::*;
use tracing_subscriber::FmtSubscriber;

fn main() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.) will be written to stdout.
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // Verify we can find a device
    let _ = pcap::Device::list().expect("device lookup failed");

    // get the default Device
    let device = pcap::Device::lookup()
        .expect("device lookup failed")
        .expect("no device available");

    info!("Using device '{}'", device.name);

    // Setup Capture
    let mut cap = pcap::Capture::from_device(device)
        .unwrap()
        .immediate_mode(true)
        .open()
        .unwrap();

    cap.filter(format!("udp port {:?}", DEFAULT_PORT).as_str(), true)
        .expect("failed to filter for netwayste packets");

    while let Ok(packet) = cap.next_packet() {
        debug!("{:?}", packet);
        match SlicedPacket::from_ethernet(packet.data) {
            Err(err) => {
                panic!("deserializing EthernetII packet: {}", err);
            }
            Ok(ethernet) => {
                match ethernet.transport {
                    Some(Udp(udp)) => info!("src={} dst={} data={:?}", udp.source_port(), udp.destination_port(), ethernet.payload),
                    _ => (),
                }

                // TODO: Deserialize 'ethernet.payload' into a Netwayste Packet
            }
        }
    }
}
