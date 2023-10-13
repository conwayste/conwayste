pub mod interface;
pub use interface::*;

use std::sync::Arc;

use pyo3::exceptions::*;
use pyo3::prelude::*;
use tokio::sync::{mpsc::error::TryRecvError, Mutex};

use netwaystev2::filter::{
    Filter, FilterCmd, FilterCmdSend, FilterMode, FilterNotifyRecv, FilterRspRecv, ServerStatus,
};

use crate::transport::*;

// All the below Options are to permit taking these and passing them into async blocks
#[pyclass]
pub struct FilterInterface {
    filter:      Option<Filter>,
    cmd_tx:      FilterCmdSend,
    response_rx: Arc<Mutex<FilterRspRecv>>,
    notify_rx:   FilterNotifyRecv,
}

/// Ex:
///
/// ```
/// take_from_self_or_raise_exc!(mut t_iface <- self.transport_iface);
/// ```
///
/// Expands to:
///
/// ```
/// let mut t_iface = self
///     .transport_iface
///     .take()
///     .ok_or_else(|| PyException::new_err("cannot call run() more than once - transport_iface"))?;
/// ```
///
/// The `mut` is optional.
///
/// Must be used in a function that returns `PyResult<T>`, where `T` can be any type.
macro_rules! take_from_self_or_raise_exc {
    (mut $var:ident <- $self:ident.$field:ident) => {
        let mut $var = $self.$field.take().ok_or_else(|| {
            PyException::new_err(format!("cannot call run() more than once - {}", stringify!($field)))
        })?;
    };
    ($var:ident <- $self:ident.$field:ident) => {
        let $var = $self.$field.take().ok_or_else(|| {
            PyException::new_err(format!("cannot call run() more than once - {}", stringify!($field)))
        })?;
    };
}

#[pymethods]
impl FilterInterface {
    /// The argument can be a TransportInterface, but any object that has the same methods with the
    /// same signatures, including the "async" on the `command_response` method and the lack of
    /// "async" on `get_notifications`, will work here.
    #[new]
    fn new(transport_iface: &mut TransportInterface, filter_mode: FilterModeW) -> Self {
        // This causes the TransportInterface to no longer be usable, but that's OK.
        let transport_response_rx = Arc::try_unwrap(transport_iface.response_rx.take().expect("T.I. usable"))
            .expect("singly held Arc")
            .into_inner();
        let transport_notify_rx = transport_iface.notify_rx.take().expect("T.I. usable");

        // Create the filter.
        let (filter, filter_cmd_tx, filter_rsp_rx, filter_notice_rx) = Filter::new(
            transport_iface.cmd_tx.clone(),
            transport_response_rx,
            transport_notify_rx,
            filter_mode.into(),
        );

        FilterInterface {
            filter:      Some(filter),
            cmd_tx:      filter_cmd_tx,
            response_rx: Arc::new(Mutex::new(filter_rsp_rx)),
            notify_rx:   filter_notice_rx,
        }
    }

    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        take_from_self_or_raise_exc!(mut filter <- self.filter);

        let rust_fut = async move {
            filter.run().await;
            Ok(())
        };

        pyo3_asyncio::tokio::future_into_py(py, rust_fut) // Returns a Python future
    }

    /// Send a command and get a response. This is essentially a copy-paste job from TransportInterface.
    fn command_response<'p>(&mut self, py: Python<'p>, filter_cmd: FilterCmdW) -> PyResult<&'p PyAny> {
        let cmd_tx = self.cmd_tx.clone();
        let response_rx = self.response_rx.clone();
        let send_recv_fut = async move {
            let filter_cmd = filter_cmd.into();
            cmd_tx
                .send(filter_cmd)
                .await
                .map_err(|e| PyException::new_err(format!("failed to send FilterCmd: {}", e)))?;
            let mut response_rx = response_rx
                .try_lock()
                .map_err(|e| PyException::new_err(format!("failed to unlock filter response receiver: {}", e)))?;
            Ok(response_rx.recv().await.map(|resp| FilterRspW::from(resp)))
        };
        pyo3_asyncio::tokio::future_into_py(py, send_recv_fut)
    }

    /// Send an individual command.
    fn command<'p>(&mut self, py: Python<'p>, filter_cmd: FilterCmdW) -> PyResult<&'p PyAny> {
        let cmd_tx = self.cmd_tx.clone();
        let send_fut = async move {
            let filter_cmd = filter_cmd.into();
            cmd_tx
                .send(filter_cmd)
                .await
                .map_err(|e| PyException::new_err(format!("failed to send FilterCmd: {}", e)))?;
            Ok(())
        };
        pyo3_asyncio::tokio::future_into_py(py, send_fut)
    }

    /// Receive an individual response.
    fn response<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let response_rx = self.response_rx.clone();
        let recv_fut = async move {
            let mut response_rx = response_rx
                .try_lock()
                .map_err(|e| PyException::new_err(format!("failed to unlock filter response receiver: {}", e)))?;
            Ok(response_rx.recv().await.map(|resp| FilterRspW::from(resp)))
        };
        pyo3_asyncio::tokio::future_into_py(py, recv_fut)
    }

    /// Get a Vec of Filter notifications.
    /// Note: Not Python async, unlike other methods!
    /// This is essentially a copy-paste job from TransportInterface.
    fn get_notifications(&mut self) -> PyResult<Vec<FilterNoticeW>> {
        let mut notifications = vec![];
        loop {
            match self.notify_rx.try_recv() {
                Ok(notification) => {
                    notifications.push(notification.into());
                    continue;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return Err(PyException::new_err("filter notify channel was disconnected"))
                }
            }
        }
        Ok(notifications)
    }
}

impl Drop for FilterInterface {
    fn drop(&mut self) {
        let _ = self.cmd_tx.try_send(FilterCmd::Shutdown { graceful: false });
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct FilterModeW {
    inner: FilterMode,
}

impl_from_and_to!(FilterModeW wraps FilterMode);

#[pymethods]
impl FilterModeW {
    #[new]
    fn new(mode: String) -> PyResult<Self> {
        let mode = match mode.as_str() {
            "client" => FilterMode::Client,
            "server" => FilterMode::Server(ServerStatus::default()), // TODO: allow populating server mode
            _ => {
                return Err(PyValueError::new_err(format!(
                    "invalid mode: {}, must be client or server",
                    mode
                )));
            }
        };
        Ok(FilterModeW { inner: mode })
    }
}
