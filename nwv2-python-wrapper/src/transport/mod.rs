pub mod interface;
pub use interface::*;

use std::net::SocketAddr;
use std::sync::Arc;

use pyo3::exceptions::*;
use pyo3::prelude::*;
use tokio::sync::{mpsc::error::TryRecvError, Mutex};

use netwaystev2::common::Endpoint;
use netwaystev2::transport::{Transport, TransportCmd, TransportCmdSend, TransportNotifyRecv, TransportRspRecv};
use crate::common::EndpointW;

#[pyclass]
pub struct TransportInterface {
    transport:   Option<Transport>,
    cmd_tx:      TransportCmdSend,
    response_rx: Arc<Mutex<TransportRspRecv>>, // Can't clone an MPSC receiver; need to share :(
    notify_rx:   TransportNotifyRecv,          // ... but this one doesn't need that because it's only read in non-async
    local_addr:  SocketAddr,
}

/// Create a TransportInterface.
/// This can't be a #[new] constructor because it's Python async.
#[pyfunction]
pub fn new_transport_interface<'p>(
    py: Python<'p>,
    opt_host: Option<String>,
    opt_port: Option<u16>,
) -> PyResult<&'p PyAny> {
    let err_mapper = |e| PyException::new_err(format!("failed to create Transport: {}", e));
    let transport_fut = async move {
        let (transport, cmd_tx, response_rx, notify_rx) =
            Transport::new(opt_host, opt_port).await.map_err(err_mapper)?;
        let local_addr = transport.local_addr();
        Ok(TransportInterface {
            transport: Some(transport),
            cmd_tx,
            response_rx: Arc::new(Mutex::new(response_rx)),
            notify_rx,
            local_addr,
        })
    };
    pyo3_asyncio::tokio::future_into_py(py, transport_fut)
}

#[pymethods]
impl TransportInterface {
    /// Runs the Transport. The Python Future will complete when the Transport exits.
    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let mut transport = self
            .transport
            .take()
            .ok_or_else(|| PyException::new_err("cannot call run() more than once"))?;
        let run_fut = async move {
            // Use .await.map_err(...) to get rid of the anyhow
            transport
                .run()
                .await
                .map_err(|e| PyException::new_err(format!("error from run(): {}", e)))
        };
        pyo3_asyncio::tokio::future_into_py(py, run_fut)
    }

    /// Send a command and get a response
    fn command_response<'p>(&mut self, py: Python<'p>, transport_cmd: TransportCmdW) -> PyResult<&'p PyAny> {
        let cmd_tx = self.cmd_tx.clone();
        let response_rx = self.response_rx.clone();
        let send_recv_fut = async move {
            let transport_cmd = transport_cmd.into();
            if let TransportCmd::SendPackets {
                ref packet_infos,
                ref packets,
                ..
            } = transport_cmd
            {
                if packet_infos.len() == packets.len() && packets.len() != 1 {
                    // Packet vec lengths other than 1 aren't supported because they would require
                    // support for reading a number of TransportRsps other than 1, which
                    // complicates things. However, mismatched lengths are supported only to allow
                    // testing for the length mismatch error.
                    return Err(PyValueError::new_err(format!(
                        "unsupported TransportCmd::SendPackets - length {}, should be 1",
                        packets.len()
                    )));
                }
            }
            cmd_tx
                .send(transport_cmd)
                .await
                .map_err(|e| PyException::new_err(format!("failed to send TransportCmd: {}", e)))?;
            let mut response_rx = response_rx
                .try_lock()
                .map_err(|e| PyException::new_err(format!("failed to unlock transport response receiver: {}", e)))?;
            Ok(response_rx.recv().await.map(|resp| TransportRspW::from(resp)))
        };
        pyo3_asyncio::tokio::future_into_py(py, send_recv_fut)
    }

    /// Get a Vec of Transport notifications.
    /// Note: Not Python async, unlike other methods!
    fn get_notifications(&mut self) -> PyResult<Vec<TransportNoticeW>> {
        let mut notifications = vec![];
        loop {
            match self.notify_rx.try_recv() {
                Ok(notification) => {
                    notifications.push(notification.into());
                    continue;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return Err(PyException::new_err("transport notify channel was disconnected"))
                }
            }
        }
        Ok(notifications)
    }

    #[getter]
    fn get_local_addr(&self) -> EndpointW {
        Endpoint(self.local_addr).into()
    }

    fn __repr__(&self) -> String {
        format!(
            "TransportInterface{{ transport: {},   cmd_tx: ...,   response_rx: ...,   notify_rx: ... }}",
            if self.transport.is_some() {
                "Some(<Transport>)"
            } else {
                "None"
            }, // run() takes this; keep borrow
               // checker happy
        )
    }
}
