pub mod interface;
pub use interface::*;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};
use std::time::Duration;

use pyo3::exceptions::*;
use pyo3::gc::PyVisit;
use pyo3::prelude::*;
use pyo3::PyTraverseError;
use tokio::sync::{
    mpsc::error::{SendError, TryRecvError},
    mpsc::{self, Receiver, Sender},
    watch, Mutex,
};

use tokio::time::sleep;

use netwaystev2::transport::{TransportCmd, TransportNotice, TransportRsp};
use netwaystev2::{
    filter::{Filter, FilterCmd, FilterCmdSend, FilterMode, FilterNotice, FilterNotifyRecv, FilterRsp, FilterRspRecv},
    protocol::ResponseCode,
};

use crate::{filter, transport::*};

// All the below Options are to permit taking these and passing them into async blocks
#[pyclass]
pub struct FilterInterface {
    filter:             Option<Filter>,
    cmd_tx:             FilterCmdSend,
    response_rx:        Arc<Mutex<FilterRspRecv>>,
    notify_rx:          Arc<Mutex<FilterNotifyRecv>>,
    notif_poll_ms:      Arc<AtomicUsize>, // Controls how frequently we poll the TransportInterface for transport notifications
    transport_iface:    Option<Arc<PyObject>>, // duck-typed Python object (same methods as TransportInterface)
    transport_channels: Option<TransportChannels>,
    shutdown_tx:        watch::Sender<()>, // sends or closes if FilterInterface is shutdown
    shutdown_rx:        watch::Receiver<()>, // Is cloned and provided to async functions so they don't run forever
    filter_mode:        FilterMode,
}

struct TransportChannels {
    transport_cmd_rx:    Receiver<TransportCmd>,
    transport_rsp_tx:    Sender<TransportRsp>,
    transport_notice_tx: Sender<TransportNotice>,
}

// TODO: reference the one in netw.../src/settings.rs
pub const TRANSPORT_CHANNEL_LEN: usize = 1000;

pub const DEFAULT_NOTIFY_POLL_MS: usize = 30; // Milliseconds to wait between calling get_notifications()

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
            notify_rx: Arc::new(Mutex::new(filter_notice_rx)),
            notif_poll_ms: Arc::new(AtomicUsize::new(DEFAULT_NOTIFY_POLL_MS)),
            transport_iface: Some(Arc::new(transport_iface)),
            transport_channels: Some(transport_channels),
            shutdown_tx,
            shutdown_rx,
            filter_mode: filter_mode.into(),
        }
    }

    #[getter]
    fn get_notif_poll_ms(&self) -> PyResult<usize> {
        Ok(self.notif_poll_ms.load(Ordering::SeqCst))
    }

    #[setter]
    fn set_notif_poll_ms(&mut self, notif_poll_ms: usize) -> PyResult<()> {
        self.notif_poll_ms.store(notif_poll_ms, Ordering::SeqCst);
        Ok(())
    }

    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        take_from_self_or_raise_exc!(mut filter <- self.filter);
        take_from_self_or_raise_exc!(t_channels <- self.transport_channels);
        take_from_self_or_raise_exc!(t_iface <- self.transport_iface);

        let transport_notice_tx = t_channels.transport_notice_tx.clone();

        let mut filter_cmd_tx = self.cmd_tx.clone();
        let mut filter_notify_rx = self.notify_rx.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let shutdown_rx2 = self.shutdown_rx.clone();
        let shutdown_rx3 = self.shutdown_rx.clone();
        let notif_poll_ms = self.notif_poll_ms.clone();
        let notif_poll_ms2 = self.notif_poll_ms.clone();
        let filter_mode = self.filter_mode;

        let rust_fut = async move {
            tokio::join!(
                filter.run(),
                handle_transport_cmd_resp(t_channels, Arc::downgrade(&t_iface), shutdown_rx),
                handle_transport_notification(
                    transport_notice_tx,
                    Arc::downgrade(&t_iface),
                    notif_poll_ms,
                    shutdown_rx2
                ),
                handle_filter_notification(&mut filter_cmd_tx, filter_notify_rx, notif_poll_ms2, shutdown_rx3, filter_mode),
            );
            Ok(())
        };

        pyo3_asyncio::tokio::future_into_py(py, rust_fut) // Returns a Python future
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
        let _ = self.cmd_tx.try_send(FilterCmd::Shutdown { graceful: false });
        let _ = self.shutdown_tx.send(()); // Shutdown async worker functions.
        self.transport_iface = None; // Clear reference, this decrements PyObject ref counter.
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

    /// Get a Vec of Filter notifications.
    /// Note: Not Python async, unlike other methods!
    /// This is essentially a copy-paste job from TransportInterface.
    fn get_notifications(&mut self) -> PyResult<Vec<FilterNoticeW>> {
        let mut notifications = vec![];
        loop {
            match self.notify_rx.try_lock().expect("failed to acquire notify rx lock").try_recv() {
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
        let transport_rspw: TransportRspW = Python::with_gil(|py| {
            if transport_rspw.is_none(py) {
                panic!("Return value from transport interface .command_response() is None");
            }
            transport_rspw.extract(py)
        })
        .expect("command_response must return TransportRspW"); // ToDo: handle better
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

async fn handle_transport_notification(
    transport_notice_tx: Sender<TransportNotice>,
    transport_iface: Weak<PyObject>,
    notif_poll_ms: Arc<AtomicUsize>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    loop {
        let t_iface = if let Some(ti) = transport_iface.upgrade() {
            ti
        } else {
            // Cannot upgrade from weak reference due to Python GC having us break a reference
            // cycle. Should be rare case.
            return;
        };

        // Python call "get_notifications" on transport_iface. Note: not Python async!
        let py_retval = Python::with_gil(|py| t_iface.call_method0(py, "get_notifications"))
            .expect("TransportInterface get_notifications should not raise exception");
        let py_obj_vec: Vec<Py<PyAny>> = Python::with_gil(|py| py_retval.extract(py))
            .expect("TransportInterface get_notifications must return list");
        let transport_notif_wrappers: Vec<TransportNoticeW> = Python::with_gil(|py| {
            py_obj_vec
                .into_iter()
                .map(|obj| {
                    obj.extract(py)
                        .expect("TransportInterface get_notifications must return list of TransportNoticeW")
                })
                .collect()
        });
        drop(t_iface); // Now we only have a weak ref

        // Send any notifications we received above to `transport_notice_tx` so that Filter can handle them.
        for tnoticew in transport_notif_wrappers {
            let transport_notice: TransportNotice = tnoticew.into();
            if let Err(_) = transport_notice_tx.send(transport_notice).await {
                // Filter layer must have been dropped
                return;
            }
        }

        let poll_interval = Duration::from_millis(
            notif_poll_ms
                .load(Ordering::SeqCst)
                .try_into()
                .expect("notif_poll_ms too big"),
        );
        tokio::select! {
            _ = sleep(poll_interval) => {}
            _ = shutdown_rx.changed() => {
                return;
            }
        };
    }
}

async fn handle_filter_notification(
    cmd_tx: &mut Sender<FilterCmd>,
    notify_rx: Arc<Mutex<Receiver<FilterNotice>>>,
    notif_poll_ms: Arc<AtomicUsize>,
    mut shutdown_rx: watch::Receiver<()>,
    filter_mode: FilterMode,
) {
    // Only run the rest in server mode
    if filter_mode == FilterMode::Client {
        return;
    }

    loop {
        // Python call "get_notifications" on filter_notifications. Note: not Python async!
        /*
        let py_retval = Python::with_gil(|py| filter_iface
            .call_method0(py, "get_notifications"))
            .expect("TransportInterface get_notifications should not raise exception");
        let py_obj_vec: Vec<Py<PyAny>> = Python::with_gil(|py| py_retval.extract(py))
            .expect("TransportInterface get_notifications must return list");
        let transport_notif_wrappers: Vec<TransportNoticeW> = Python::with_gil(|py| {
            py_obj_vec
                .into_iter()
                .map(|obj| {
                    obj.extract(py)
                        .expect("TransportInterface get_notifications must return list of TransportNoticeW")
                })
                .collect()
        });
        drop(t_iface); // Now we only have a weak ref
        */
        let mut notice = None;

        let mut notify_rx = notify_rx
            .try_lock()
            .expect("Failed to acquire notify rx lock. Why?");

        while let Ok(message) = notify_rx.try_recv() {
            notice = Some(message);
            break;
        }

        if let Some(message) = notice {
            match message {
                FilterNotice::NewRequestAction { endpoint, action } => {
                        cmd_tx
                        .try_send(FilterCmd::SendResponseCode {
                            endpoint,
                            code: ResponseCode::OK,
                        })
                        .expect("Channel closed?");
                }
                FilterNotice::EndpointTimeout { endpoint } => {
                    //XXX
                    info!("received FilterNotice::EndpointTimeout in handle_filter_notification");
                }
                _ => panic!("Unhandled filter notice in handle_filter_notification: {:?}", message),
            }
        }

        let poll_interval = Duration::from_millis(
            notif_poll_ms
                .load(Ordering::SeqCst)
                .try_into()
                .expect("notif_poll_ms too big"),
        );
        tokio::select! {
            _ = sleep(poll_interval) => {}
            _ = shutdown_rx.changed() => {
                return;
            }
        };
    }
}

#[pyclass]
#[derive(Clone, Copy, Debug)]
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
