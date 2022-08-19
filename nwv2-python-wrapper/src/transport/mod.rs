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

#[pymethods]
impl TransportInterface  {
    #[new]
    #[args(kwds="**")]
    fn new(opt_host: Option<&str>, opt_port: Option<u16>) -> PyResult<Self> {
        let err_mapper = |e| {
            PyException::new_err(format!("failed to create Transport: {}", e))
        };
        let (transport, cmd_tx, response_rx, notify_rx) = Transport::new(opt_host, opt_port).map_err(err_mapper)?;
        Ok(TransportInterface{
            transport,
            cmd_tx,
            response_rx,
            notify_rx,
        })
    }
}
