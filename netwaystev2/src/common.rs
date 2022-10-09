use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint(pub SocketAddr);

pub type ShutdownWatcher = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
