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

    cap.filter(format!("udp port {:?}", DEFAULT_PORT).as_str(), true).expect("failed to filter for netwayste packets");

    while let Ok(packet) = cap.next_packet() {
        info!("{:?}", packet);
    }
}
