use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Weak};

use pyo3::exceptions::*;
use pyo3::gc::PyVisit;
use pyo3::prelude::*;
use pyo3::PyTraverseError;
use tokio::sync::{
    mpsc::error::{SendError, TryRecvError},
    mpsc::{self, Receiver, Sender},
    watch, Mutex,
};

use netwaystev2::filter::{
    Filter, FilterCmd, FilterCmdSend, FilterMode, FilterNotice, FilterNotifyRecv, FilterRsp, FilterRspRecv,
};
use netwaystev2::transport::{TransportCmd, TransportNotice, TransportRsp};

use crate::transport::*;

// All the below Options are to permit taking these and passing them into async blocks
#[pyclass]
pub struct FilterInterface {
    filter:             Option<Filter>,
    cmd_tx:             FilterCmdSend,
    response_rx:        Arc<Mutex<FilterRspRecv>>,
    notify_rx:          FilterNotifyRecv,
    transport_iface:    Option<Arc<PyObject>>, // duck-typed Python object (same methods as TransportInterface)
    transport_channels: Option<TransportChannels>,
    shutdown_tx:        watch::Sender<()>, // sends or closes if FilterInterface is shutdown
    shutdown_rx:        watch::Receiver<()>, // Is cloned and provided to async functions so they don't run forever
}

struct TransportChannels {
    transport_cmd_rx:    Receiver<TransportCmd>,
    transport_rsp_tx:    Sender<TransportRsp>,
    transport_notice_tx: Sender<TransportNotice>,
}

// TODO: reference the one in netw.../src/settings.rs
pub const TRANSPORT_CHANNEL_LEN: usize = 1000;

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
    fn new(transport_iface: PyObject, filter_mode: FilterModeW) -> Self {
        // Channels for communicating between Transport and Filter layers
        let (transport_cmd_tx, transport_cmd_rx) = mpsc::channel::<TransportCmd>(TRANSPORT_CHANNEL_LEN);
        let (transport_rsp_tx, transport_rsp_rx) = mpsc::channel::<TransportRsp>(TRANSPORT_CHANNEL_LEN);
        let (transport_notice_tx, transport_notice_rx) = mpsc::channel::<TransportNotice>(TRANSPORT_CHANNEL_LEN);
        let transport_channels = TransportChannels {
            transport_cmd_rx,
            transport_rsp_tx,
            transport_notice_tx,
        };

        // Create the filter.
        let (filter, filter_cmd_tx, filter_rsp_rx, filter_notice_rx) = Filter::new(
            transport_cmd_tx,
            transport_rsp_rx,
            transport_notice_rx,
            filter_mode.into(),
        );

        let (shutdown_tx, shutdown_rx) = watch::channel(());

        FilterInterface {
            filter: Some(filter),
            cmd_tx: filter_cmd_tx,
            response_rx: Arc::new(Mutex::new(filter_rsp_rx)),
            notify_rx: filter_notice_rx,
            transport_iface: Some(Arc::new(transport_iface)),
            transport_channels: Some(transport_channels),
            shutdown_tx,
            shutdown_rx,
        }
    }

    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        take_from_self_or_raise_exc!(mut filter <- self.filter);
        take_from_self_or_raise_exc!(t_channels <- self.transport_channels);
        take_from_self_or_raise_exc!(t_iface <- self.transport_iface);

        tokio::spawn(handle_transport_cmd_resp(
            t_channels,
            Arc::downgrade(&t_iface),
            self.shutdown_rx.clone(),
        ));
        //XXX handle_transport_notification

        let run_fut = async move { Ok(filter.run().await) };

        pyo3_asyncio::tokio::future_into_py(py, run_fut)
    }

    // Python GC methods - https://pyo3.rs/v0.16.4/class/protocols.html#garbage-collector-integration
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(t_iface) = &self.transport_iface {
            let ti: &PyObject = &*t_iface;
            visit.call(ti)?
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        let _ = self.shutdown_tx.send(()); // Shutdown async worker functions.
                                           // Clear reference, this decrements PyObject ref counter.
        self.transport_iface = None;
    }

    //XXX command_response

    //XXX get_notifications
}

impl Drop for FilterInterface {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(()); // Shutdown async worker functions.
    }
}

/// Responsible for passing Transport layer commands and responses between the channels to the
/// Filter layer and the Python TransportInterface wrapper.
///
/// There is a weak reference to `transport_iface` so that Python GC can operate correctly.
async fn handle_transport_cmd_resp(
    mut t_channels: TransportChannels,
    transport_iface: Weak<PyObject>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    loop {
        // Receive transport command from Filter layer. Return if Filter layer was shutdown.
        let transport_cmd = tokio::select! {
            maybe_transport_cmd = t_channels.transport_cmd_rx.recv() => {
                if let Some(c) = maybe_transport_cmd {
                    c
                } else {
                    return;
                }
            }
            _ = shutdown_rx.changed() => {
                return;
            }
        };
        let transport_cmdw: TransportCmdW = transport_cmd.into();

        // Call command_response, passing in the TransportCmd and getting a Python Future
        // that returns a TransportRsp.

        let t_iface = if let Some(ti) = transport_iface.upgrade() {
            ti
        } else {
            // Cannot upgrade from weak reference due to Python GC having us break a reference
            // cycle. Should be rare case.
            return;
        };
        let transport_rspw_fut = Python::with_gil(|py| {
            let py_retval = t_iface
                .call_method(py, "command_response", (transport_cmdw,), None)
                .expect("unreachable");
            pyo3_asyncio::tokio::into_future(py_retval.as_ref(py)).expect("should return future")
        });
        let transport_rspw: PyObject = transport_rspw_fut.await.expect("unreachable2");
        let transport_rspw: TransportRspW =
            Python::with_gil(|py| transport_rspw.extract(py)).expect("command_response must return TransportRspW"); // ToDo: handle better
        let transport_rsp: TransportRsp = transport_rspw.into();
        match t_channels.transport_rsp_tx.send(transport_rsp).await {
            Err(SendError(_)) => {
                // A failure to send indicates the Filter layer was shutdown
                return;
            }
            _ => {} // Continue with loop
        }
    }
}

#[pyclass]
#[derive(Clone, Copy, Debug)]
pub struct FilterModeW {
    inner: FilterMode,
}

impl Into<FilterMode> for FilterModeW {
    fn into(self) -> FilterMode {
        self.inner
    }
}

#[pymethods]
impl FilterModeW {
    #[new]
    fn new(mode: String) -> PyResult<Self> {
        let mode = match mode.as_str() {
            "client" => FilterMode::Client,
            "server" => FilterMode::Server,
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
