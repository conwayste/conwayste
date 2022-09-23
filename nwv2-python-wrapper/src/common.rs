use std::net::SocketAddr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use netwaystev2::common::Endpoint;

#[pyclass]
#[derive(Clone)]
pub struct EndpointW {
    inner: Endpoint,
}

impl Into<Endpoint> for EndpointW {
    fn into(self) -> Endpoint {
        self.inner
    }
}

#[pymethods]
impl EndpointW {
    #[new]
    fn new(host_and_port: String) -> PyResult<Self> {
        let sa: SocketAddr = host_and_port.parse().map_err(|e| {
            return PyValueError::new_err(format!("failed to parse SocketAddr string for Endpoint: {}", e));
        })?;
        Ok(EndpointW { inner: Endpoint(sa) })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}
