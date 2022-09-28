use std::sync::Arc;

use pyo3::exceptions::*;
use pyo3::prelude::*;
use tokio::sync::{
    mpsc::error::TryRecvError,
    mpsc::{self, Receiver, Sender},
    Mutex,
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
    transport_iface:    Option<PyObject>, // duck-typed Python object (same methods as TransportInterface)
    transport_channels: Option<TransportChannels>,
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
/// take_from_self_or_raise_exc!(t_iface <- self.transport_iface);
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
macro_rules! take_from_self_or_raise_exc {
    ($var:ident <- $self:ident.$field:ident) => {
        let mut $var = $self.$field.take().ok_or_else(|| {
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

        let (filter, filter_cmd_tx, filter_rsp_rx, filter_notice_rx) = Filter::new(
            transport_cmd_tx,
            transport_rsp_rx,
            transport_notice_rx,
            filter_mode.into(),
        );

        FilterInterface {
            filter:             Some(filter),
            cmd_tx:             filter_cmd_tx,
            response_rx:        Arc::new(Mutex::new(filter_rsp_rx)),
            notify_rx:          filter_notice_rx,
            transport_iface:    Some(transport_iface),
            transport_channels: Some(transport_channels),
        }
    }

    fn run<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        take_from_self_or_raise_exc!(filter <- self.filter);
        take_from_self_or_raise_exc!(t_channels <- self.transport_channels);
        take_from_self_or_raise_exc!(t_iface <- self.transport_iface);
        let run_fut = async move { Ok(filter.run().await) };
        //XXX why did I do a tokio spawn here? This is already async...
        tokio::spawn(async move {
            loop {
                // Receive transport command from Filter layer
                let transport_cmd = t_channels.transport_cmd_rx.recv().await.unwrap(); //XXX break if error
                let transport_cmdw: TransportCmdW = transport_cmd.into();

                // Call command_response, passing in the TransportCmd and getting a Python Future
                // that returns a TransportRsp.

                let transport_rspw: PyObject = Python::with_gil(|py| {
                    let py_retval = t_iface
                        .call_method(py, "command_response", (transport_cmdw,), None)
                        .expect("unreachable");
                    pyo3_asyncio::tokio::into_future(py_retval.as_ref(py)).expect("should return future")
                })
                .await
                .expect("unreachable2");
                let transport_rspw: TransportRspW =
                    Python::with_gil(|py| transport_rspw.extract(py)).expect("bad command_response retval"); // TODO: handle better
                let transport_rsp: TransportRsp = transport_rspw.into();
                //XXX send TransportRsp on channel
            }
            //XXX turn channel send/recvs into calls to transport_iface methods
        });
        pyo3_asyncio::tokio::future_into_py(py, run_fut)
    }

    //XXX more methods
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
