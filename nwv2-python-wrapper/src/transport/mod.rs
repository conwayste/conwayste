pub mod interface;
pub use interface::*;

use futures_util::future::TryFutureExt;
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
    transport: Option<Transport>,
    cmd_tx: TransportCmdSend,
    response_rx: TransportRspRecv,
    notify_rx: TransportNotifyRecv,
}

/// Create a TransportInterface.
/// This can't be a #[new] constructor because it's Python async.
#[pyfunction]
pub fn new_transport_interface<'p>(py: Python<'p>, opt_host: Option<String>, opt_port: Option<u16>) -> PyResult<&'p PyAny> {
    let err_mapper = |e| {
        PyException::new_err(format!("failed to create Transport: {}", e))
    };
    let transport_fut = async move {
        let (transport, cmd_tx, response_rx, notify_rx) = Transport::new(opt_host, opt_port).await.map_err(err_mapper)?;
        Ok(TransportInterface{
            transport: Some(transport),
            cmd_tx,
            response_rx,
            notify_rx,
        })
    };
    pyo3_asyncio::tokio::future_into_py(py, transport_fut)
}

#[pymethods]
impl TransportInterface  {
    /// Runs the Transport. The Python Future will complete when the Transport exits.
    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let mut transport = self.transport.take().ok_or_else(|| PyException::new_err("cannot call run() more than once"))?;
        let run_fut = async move {
            // Use .await.map_err(...) to get rid of the anyhow
            transport.run().await.map_err(|e| PyException::new_err(format!("error from run(): {}", e)))
        };
        pyo3_asyncio::tokio::future_into_py(py, run_fut)
    }

    //fn command_response<'p>(&mut self, py: Python<'p>, transport_cmd: /*XXX impl*/TransportCmdW) -> PyResult<&'p PyAny> {
    //    //XXX send a command and get a response
    //}

    //fn get_notifications<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
    //    //XXX get a Vec of Transport notifications.
    //}

    fn __repr__(&self) -> String {
        format!(
            "TransportInterface{{ transport: {},   cmd_tx: {:?},   response_rx: {:?},   notify_rx: {:?} }}",
            if self.transport.is_some() { "Some(<Transport>)" } else { "None" }, // run() takes this; keep borrow
                                                                                 // checker happy
            self.cmd_tx, self.response_rx, self.notify_rx,
        )
    }
}
