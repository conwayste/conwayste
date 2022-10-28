use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::common::*;
use crate::utils::get_from_dict;
use crate::{BroadcastChatMessageW, RequestActionW, ResponseCodeW};
use netwaystev2::common::Endpoint;
use netwaystev2::filter::{FilterCmd, FilterNotice, FilterRsp};
use netwaystev2::protocol::BroadcastChatMessage;

#[pyclass]
#[derive(Debug, Clone)]
pub struct FilterCmdW {
    inner: FilterCmd,
}

impl Into<FilterCmd> for FilterCmdW {
    fn into(self) -> FilterCmd {
        self.inner
    }
}

impl From<FilterCmd> for FilterCmdW {
    fn from(other: FilterCmd) -> Self {
        FilterCmdW { inner: other }
    }
}

#[pymethods]
impl FilterCmdW {
    #[new]
    #[args(kwds = "**")]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let tc = match variant.to_lowercase().as_str() {
            "sendrequestaction" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let req_actionw: RequestActionW = get_from_dict(&kwds, "action")?;
                FilterCmd::SendRequestAction {
                    endpoint: endpointw.into(),
                    action:   req_actionw.into(),
                }
            }
            "sendresponsecode" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let resp_codew: ResponseCodeW = get_from_dict(&kwds, "code")?;
                FilterCmd::SendResponseCode {
                    endpoint: endpointw.into(),
                    code:     resp_codew.into(),
                }
            }
            "sendchats" => {
                vec_from_py!{let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                vec_from_py!{let messages: Vec<BroadcastChatMessage> <- [BroadcastChatMessageW] <- get_from_dict(&kwds, "messages")?};
                FilterCmd::SendChats { endpoints, messages }
            }
            "sendgameupdates" => {
                vec_from_py!{let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                //XXX messages
                FilterCmd::SendGameUpdates { endpoints, messages }
            }
            "authenticated" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                FilterCmd::Authenticated { endpoint: endpointw.into() }
            }
            "sendgenstatediff" => {
                vec_from_py!{let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                //XXX diff
                FilterCmd::SendGenStateDiff { endpoints, diff }
            }
            "addpingendpoints" => {
                vec_from_py!{let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                FilterCmd::AddPingEndpoints { endpoints }
            }
            "clearpingendpoints" => FilterCmd::ClearPingEndpoints,
            "dropendpoint" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                FilterCmd::DropEndpoint { endpoint: endpointw.into() }
            }
            "shutdown" => {
                let graceful: bool = get_from_dict(&kwds, "graceful")?;
                FilterCmd::Shutdown { graceful }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(FilterCmdW { inner: tc })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}
