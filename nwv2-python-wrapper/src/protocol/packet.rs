use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use netwaystev2::protocol::Packet;
use super::request::RequestActionW;
use crate::utils::get_from_dict;

/// A wrapped netwaystev2 Packet
///
/// Example usage:
///
/// ```python
/// p = PacketW("request", "fakecookie")
/// ```
#[pyclass]
#[derive(Clone)]
pub struct PacketW {
    inner: Packet,
}

impl Into<Packet> for PacketW {
    fn into(self) -> Packet {
        self.inner
    }
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
