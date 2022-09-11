///! Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules

mod protocol;
mod transport;
pub(crate) mod common;
pub(crate) mod utils;
use protocol::request::RequestActionW;
use protocol::packet::PacketW;
use transport::*;
use common::EndpointW;

use pyo3_asyncio;
use pyo3::prelude::*;


/// A Python module implemented in Rust.
#[pymodule]
fn nwv2_python_wrapper(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(rust_delayed_value, m)?)?;
    m.add_class::<RequestActionW>()?;
    m.add_class::<PacketW>()?;
    m.add_class::<EndpointW>()?;
    m.add_class::<ProcessUniqueIdW>()?;
    m.add_class::<PacketSettingsW>()?;
    m.add_class::<TransportInterface>()?;
    m.add_class::<TransportCmdW>()?;
    m.add_class::<TransportRspW>()?;
    m.add_class::<TransportNoticeW>()?;
    m.add_function(wrap_pyfunction!(new_transport_interface, m)?)?;
    Ok(())
}
