use pyo3::prelude::*;
use pyo3::exceptions::*;

use netwaystev2::transport::{
    Transport,
    TransportInit,
    TransportCmdSend,
    TransportRspRecv,
    TransportNotifyRecv,
};

#[pyclass]
pub struct TransportInterface {
    transport: Transport,
    cmd_tx: TransportCmdSend,
    response_rx: TransportRspRecv,
    notify_rx: TransportNotifyRecv,
}

// This can't be a #[new] constructor because it's Python async.
#[pyfunction]
pub fn new_transport_interface<'p>(py: Python<'p>, opt_host: Option<String>, opt_port: Option<u16>) -> PyResult<&'p PyAny> {
    let err_mapper = |e| {
        PyException::new_err(format!("failed to create Transport: {}", e))
    };
    let transport_fut = async move {
        let (transport, cmd_tx, response_rx, notify_rx) = Transport::new(opt_host, opt_port).await.map_err(err_mapper)?;
        Ok(TransportInterface{
            transport,
            cmd_tx,
            response_rx,
            notify_rx,
        })
    };
    pyo3_asyncio::tokio::future_into_py(py, transport_fut)
}

//XXX pyfunction run_transport_interface

#[pymethods]
impl TransportInterface  {
    fn __repr__(&self) -> String {
        "<(Transport, TransportCmdSend, TransportRspRecv, TransportNotifyRecv)>".to_owned()
    }
}
