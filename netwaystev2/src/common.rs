use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

// https://serverfault.com/questions/645890/tcpdump-truncates-to-1472-bytes-useful-data-in-udp-packets-during-the-capture/645892#645892
pub const UDP_MTU_SIZE: usize = 1440;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint(pub SocketAddr);

/// A future that completes when the layer has been shutdown. This is returned by the
/// get_shutdown_watcher() method of various layers.
pub type ShutdownWatcher = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
