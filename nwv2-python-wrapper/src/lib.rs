///! Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules

pub mod wrappers;
pub(crate) mod utils;
use wrappers::request::*;

use pyo3_asyncio;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use netwaystev2::protocol::Packet;
use netwaystev2::protocol::RequestAction;

/// A wrapped netwaystev2 Packet
///
/// Example usage:
///
/// ```python
/// p = PacketW("request", "fakecookie")
/// ```
#[pyclass]
struct PacketW {
    inner: Packet,
}

#[pymethods]
impl PacketW {
    #[new]
    fn new(variant: String, cookie: Option<String>) -> PyResult<Self> {
        let opt_packet = match variant.as_str() {
            "request" => {
                Ok(Packet::Request {
                    sequence: 0,
                    response_ack: None,
                    action: RequestAction::None,
                    cookie,
                })
            }
            // TODO: more variants
            _ => Err(PyValueError::new_err(format!("invalid variant type: {}", variant)))
        };
        opt_packet.map(|packet| PacketW { inner: packet })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    // TODO: methods for getting/setting stuff in a packet
}


/// A Python module implemented in Rust.
#[pymodule]
fn nwv2_python_wrapper(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(rust_delayed_value, m)?)?;
    m.add_class::<PacketW>()?;
    m.add_class::<RequestActionW>()?;
    Ok(())
}
