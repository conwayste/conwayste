///! Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules

use std::collections::HashMap;

pub mod wrappers;
pub(crate) mod utils;
use wrappers::request::*;
use utils::get_from_dict;

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
    #[args(kwds="**")]
    fn new(variant: String, kwds: Option<HashMap<String,&PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds {
            kwds
        } else {
            HashMap::new()
        };
        let packet = match variant.to_lowercase().as_str() {
            "request" => {
                let sequence: u64 = get_from_dict(&kwds, "sequence")?;
                let response_ack: Option<u64> = get_from_dict(&kwds, "response_ack")?;
                let action_wrapper: RequestActionW = get_from_dict(&kwds, "action")?;
                let cookie: Option<String> = get_from_dict(&kwds, "cookie")?;
                Packet::Request {
                    sequence,
                    response_ack,
                    action: action_wrapper.into(),
                    cookie,
                }
            }
            // TODO: more variants
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(PacketW { inner: packet })
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
