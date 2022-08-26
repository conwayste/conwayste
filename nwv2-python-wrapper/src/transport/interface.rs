use std::time::Duration;

use pyo3::prelude::*;
use pyo3::exceptions::*;
use snowflake::ProcessUniqueId;

use netwaystev2::transport::{PacketSettings, TransportCmd, TransportRsp};

#[pyclass]
pub struct ProcessUniqueIdW {
    pub inner: ProcessUniqueId,
}

#[pymethods]
impl ProcessUniqueIdW {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(ProcessUniqueIdW { inner: ProcessUniqueId::new() })
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }
}

#[pyclass]
pub struct PacketSettingsW {
    pub inner: PacketSettings,
}

#[pymethods]
impl PacketSettingsW {
    // ToDo: take PyDelta and convert to std::time::Duration, instead of accepting integer milliseconds
    #[new]
    fn new(retry_interval_ms: u64, tid: Option<&ProcessUniqueIdW>) -> PyResult<Self> {
        let retry_interval = Duration::from_millis(retry_interval_ms);
        let tid = tid.map(|w| w.inner).unwrap_or_else(|| ProcessUniqueId::new());
        let inner = PacketSettings{ tid, retry_interval };
        Ok(PacketSettingsW{ inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

//XXX wrappers for cmd and resp
