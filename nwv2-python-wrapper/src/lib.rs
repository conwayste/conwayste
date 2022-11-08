#[macro_use]
pub(crate) mod common;
mod filter;
///! Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules
mod protocol;
mod transport;
use common::EndpointW;
use filter::*;
use protocol::*;
use transport::*;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn nwv2_python_wrapper(_py: Python, m: &PyModule) -> PyResult<()> {
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
    m.add_class::<FilterModeW>()?;
    m.add_class::<FilterInterface>()?;
    m.add_class::<FilterCmdW>()?;
    m.add_class::<FilterRspW>()?;
    m.add_class::<FilterNoticeW>()?;
    m.add_class::<BroadcastChatMessageW>()?;
    m.add_class::<ResponseCodeW>()?;
    m.add_class::<RoomListW>()?;
    m.add_class::<GenStateDiffW>()?;
    m.add_class::<GenStateDiffPartW>()?;
    m.add_class::<NetRegionW>()?;
    m.add_class::<GameOptionsW>()?;
    m.add_class::<PlayerInfoW>()?;
    m.add_class::<GameOutcomeW>()?;
    m.add_class::<GameUpdateW>()?;
    Ok(())
}
