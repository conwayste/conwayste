use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint(pub SocketAddr);

/// A future that completes when the layer has been shutdown. This is returned by the
/// get_shutdown_watcher() method of various layers.
pub type ShutdownWatcher = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
