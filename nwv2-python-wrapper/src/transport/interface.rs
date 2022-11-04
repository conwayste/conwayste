use std::collections::HashMap;
use std::time::Duration;

use pyo3::exceptions::*;
use pyo3::prelude::*;
use snowflake::ProcessUniqueId;

use crate::common::*;
use crate::protocol::PacketW;
use crate::utils::get_from_dict;
use netwaystev2::protocol::Packet;
use netwaystev2::transport::{PacketSettings, TransportCmd, TransportNotice, TransportRsp};

#[pyclass]
#[derive(Clone, Debug)]
pub struct ProcessUniqueIdW {
    pub inner: ProcessUniqueId,
}

impl Into<ProcessUniqueId> for ProcessUniqueIdW {
    fn into(self) -> ProcessUniqueId {
        self.inner
    }
}

impl From<ProcessUniqueId> for ProcessUniqueIdW {
    fn from(other: ProcessUniqueId) -> Self {
        ProcessUniqueIdW { inner: other }
    }
}

#[pymethods]
impl ProcessUniqueIdW {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(ProcessUniqueIdW {
            inner: ProcessUniqueId::new(),
        })
    }

    fn __repr__(&self) -> String {
        format!("ProcessUniqueID{{<{}>}}", self.inner)
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PacketSettingsW {
    pub inner: PacketSettings,
}

impl Into<PacketSettings> for PacketSettingsW {
    fn into(self) -> PacketSettings {
        self.inner
    }
}

#[pymethods]
impl PacketSettingsW {
    // ToDo: take PyDelta and convert to std::time::Duration, instead of accepting integer milliseconds
    #[new]
    fn new(retry_interval_ms: u64, tid: Option<&ProcessUniqueIdW>) -> PyResult<Self> {
        let retry_interval = Duration::from_millis(retry_interval_ms);
        let tid = tid.map(|w| w.inner).unwrap_or_else(|| ProcessUniqueId::new()); // Generate new ID if none was specified
        let inner = PacketSettings { tid, retry_interval };
        Ok(PacketSettingsW { inner })
    }

    #[getter]
    fn tid(&self) -> PyResult<ProcessUniqueIdW> {
        Ok(self.inner.tid.into())
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TransportCmdW {
    pub inner: TransportCmd,
}

impl Into<TransportCmd> for TransportCmdW {
    fn into(self) -> TransportCmd {
        self.inner
    }
}

impl From<TransportCmd> for TransportCmdW {
    fn from(other: TransportCmd) -> Self {
        TransportCmdW { inner: other }
    }
}

#[pymethods]
impl TransportCmdW {
    #[new]
    #[args(kwds = "**")]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let tc = match variant.to_lowercase().as_str() {
            "newendpoint" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                // ToDo: take PyDelta and convert to std::time::Duration, instead of accepting integer milliseconds
                let timeout_ms: u64 = get_from_dict(&kwds, "timeout")?;
                let timeout = Duration::from_millis(timeout_ms);
                TransportCmd::NewEndpoint {
                    endpoint: endpointw.into(),
                    timeout,
                }
            }
            "sendpackets" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                vec_from_py! {let packet_infos: Vec<PacketSettings> <- [PacketSettingsW] <- get_from_dict(&kwds, "packet_infos")?};

                vec_from_py! {let packets: Vec<Packet> <- [PacketW] <- get_from_dict(&kwds, "packets")?};

                TransportCmd::SendPackets {
                    endpoint: endpointw.into(),
                    packet_infos,
                    packets,
                }
            }
            "dropendpoint" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                TransportCmd::DropEndpoint {
                    endpoint: endpointw.into(),
                }
            }
            "droppacket" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let tidw: ProcessUniqueIdW = get_from_dict(&kwds, "tid")?;
                TransportCmd::DropPacket {
                    endpoint: endpointw.into(),
                    tid:      tidw.into(),
                }
            }
            "canceltransmitqueue" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                TransportCmd::CancelTransmitQueue {
                    endpoint: endpointw.into(),
                }
            }
            "shutdown" => TransportCmd::Shutdown,
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(TransportCmdW { inner: tc })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TransportRspW {
    pub inner: TransportRsp,
}

impl From<TransportRsp> for TransportRspW {
    fn from(inner: TransportRsp) -> Self {
        TransportRspW { inner }
    }
}

impl Into<TransportRsp> for TransportRspW {
    fn into(self) -> TransportRsp {
        self.inner
    }
}

#[pymethods]
impl TransportRspW {
    // ToDo: new
    fn variant(&self) -> String {
        match self.inner {
            TransportRsp::Accepted => "Accepted",
            TransportRsp::BufferFull => "BufferFull",
            TransportRsp::ExceedsMtu { .. } => "ExceedsMtu",
            TransportRsp::EndpointError { .. } => "EndpointError",
            TransportRsp::SendPacketsLengthMismatch => "SendPacketsLengthMismatch",
        }
        .to_owned()
    }

    fn __repr__(&self) -> String {
        format!("TransportRsp::{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct TransportNoticeW {
    pub inner: TransportNotice,
}

impl From<TransportNotice> for TransportNoticeW {
    fn from(inner: TransportNotice) -> Self {
        TransportNoticeW { inner }
    }
}

impl Into<TransportNotice> for TransportNoticeW {
    fn into(self) -> TransportNotice {
        self.inner
    }
}

#[pymethods]
impl TransportNoticeW {
    // ToDo: new
    fn variant(&self) -> String {
        match self.inner {
            TransportNotice::PacketDelivery { .. } => "PacketDelivery",
            TransportNotice::EndpointTimeout { .. } => "EndpointTimeout",
        }
        .to_owned()
    }

    fn __repr__(&self) -> String {
        format!("TransportNotice::{:?}", self.inner)
    }
}
