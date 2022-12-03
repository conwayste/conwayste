use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use super::request::RequestActionW;
use crate::common::*;
use netwaystev2::filter::*;
use netwaystev2::protocol::*;

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

impl_from_and_to!(PacketW wraps Packet);

#[pymethods]
impl PacketW {
    #[new]
    #[args(kwds = "**")]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
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
            "getstatus" => {
                // Note: non-standard! The standard way would to accept a `ping` param of type
                // PingPongW, but that seems unnecessary...
                let ping_nonce: u64 = get_from_dict(&kwds, "ping_nonce")?;
                let ping = PingPong { nonce: ping_nonce };
                Packet::GetStatus { ping }
            }
            "status" => {
                let pong_nonce: u64 = get_from_dict(&kwds, "pong_nonce")?;
                let pong = PingPong { nonce: pong_nonce };

                let server_version: String = get_from_dict(&kwds, "server_version")?;
                let player_count: u64 = get_from_dict(&kwds, "player_count")?;
                let room_count: u64 = get_from_dict(&kwds, "room_count")?;
                let server_name: String = get_from_dict(&kwds, "server_name")?;

                Packet::Status {
                    pong,
                    server_version,
                    player_count,
                    room_count,
                    server_name,
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

    #[args(member = "\"ping_nonce\"")]
    fn get_status(&self, py: Python<'_>, member: &str) -> PyResult<Py<PyAny>> {
        match self.inner {
            Packet::GetStatus { ref ping } => match member {
                "ping_nonce" => return Ok(ping.nonce.into_py(py)),
                _ => return Err(PyValueError::new_err(format!("invalid member: {}", member))),
            },
            _ => {
                return Err(PyValueError::new_err(format!("not a Packet::GetStatus data type")));
            }
        };
    }

    #[args(member = "\"pong_nonce\"")]
    fn status(&self, py: Python<'_>, member: &str) -> PyResult<Py<PyAny>> {
        match self.inner {
            Packet::Status {
                ref pong,
                ref server_version,
                player_count,
                room_count,
                ref server_name,
            } => match member {
                "pong_nonce" => return Ok(pong.nonce.into_py(py)),
                "server_version" => return Ok(server_version.into_py(py)),
                "player_count" => return Ok(player_count.into_py(py)),
                "room_count" => return Ok(room_count.into_py(py)),
                "server_name" => return Ok(server_name.into_py(py)),
                _ => return Err(PyValueError::new_err(format!("invalid member: {}", member))),
            },
            _ => {
                return Err(PyValueError::new_err(format!("not a Packet::Status data type")));
            }
        };
    }

    // TODO: methods for getting/setting stuff in a packet
}
