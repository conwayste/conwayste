use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::common::*;
use crate::{BroadcastChatMessageW, GameUpdateW, GenStateDiffW, RequestActionW, ResponseCodeW};
use netwaystev2::common::Endpoint;
use netwaystev2::filter::{FilterCmd, FilterNotice, FilterRsp};
use netwaystev2::protocol::{BroadcastChatMessage, GameUpdate};

#[pyclass]
#[derive(Debug, Clone)]
pub struct FilterCmdW {
    inner: FilterCmd,
}

impl_from_and_to!(FilterCmdW wraps FilterCmd);

#[pymethods]
impl FilterCmdW {
    #[new]
    #[pyo3(signature = (variant, **kwds))]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let fc = match variant.to_lowercase().as_str() {
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
                vec_from_py! {let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                vec_from_py! {let messages: Vec<BroadcastChatMessage> <- [BroadcastChatMessageW] <- get_from_dict(&kwds, "messages")?};
                FilterCmd::SendChats { endpoints, messages }
            }
            "sendgameupdates" => {
                vec_from_py! {let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                vec_from_py! {let updates: Vec<GameUpdate> <- [GameUpdateW] <- get_from_dict(&kwds, "updates")?};
                FilterCmd::SendGameUpdates { endpoints, updates }
            }
            "authenticated" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                FilterCmd::Authenticated {
                    endpoint: endpointw.into(),
                }
            }
            "changeserverstatus" => {
                let server_version: Option<String> = get_from_dict(&kwds, "server_version")?;
                let player_count: Option<u64> = get_from_dict(&kwds, "player_count")?;
                let room_count: Option<u64> = get_from_dict(&kwds, "room_count")?;
                let server_name: Option<String> = get_from_dict(&kwds, "server_name")?;
                FilterCmd::ChangeServerStatus {
                    server_version,
                    player_count,
                    room_count,
                    server_name,
                }
            }
            "sendgenstatediff" => {
                vec_from_py! {let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                let diffw: GenStateDiffW = get_from_dict(&kwds, "diff")?;
                FilterCmd::SendGenStateDiff {
                    endpoints,
                    diff: diffw.into(),
                }
            }
            "addpingendpoints" => {
                vec_from_py! {let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                FilterCmd::AddPingEndpoints { endpoints }
            }
            "clearpingendpoints" => FilterCmd::ClearPingEndpoints,
            "dropendpoint" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                FilterCmd::DropEndpoint {
                    endpoint: endpointw.into(),
                }
            }
            "shutdown" => {
                let graceful: bool = get_from_dict(&kwds, "graceful")?;
                FilterCmd::Shutdown { graceful }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(FilterCmdW { inner: fc })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct FilterRspW {
    inner: FilterRsp,
}

impl_from_and_to!(FilterRspW wraps FilterRsp);

#[pymethods]
impl FilterRspW {
    #[new]
    #[pyo3(signature = (variant, **kwds))]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        use FilterRsp::*;
        let fc = match variant.to_lowercase().as_str() {
            "accepted" => Accepted,
            "nosuchendpoint" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                NoSuchEndpoint {
                    endpoint: endpointw.into(),
                }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(FilterRspW { inner: fc })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct FilterNoticeW {
    inner: FilterNotice,
}

impl_from_and_to!(FilterNoticeW wraps FilterNotice);

#[pymethods]
impl FilterNoticeW {
    #[new]
    #[pyo3(signature = (variant, **kwds))]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        use FilterNotice::*;
        let fc = match variant.to_lowercase().as_str() {
            "hasgeneration" => {
                vec_from_py! {let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
                let gen_num: u64 = get_from_dict(&kwds, "gen_num")?;
                HasGeneration { endpoints, gen_num }
            }
            "newgenstatediff" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let diffw: GenStateDiffW = get_from_dict(&kwds, "diff")?;
                NewGenStateDiff {
                    endpoint: endpointw.into(),
                    diff:     diffw.into(),
                }
            }
            "pingresult" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let latency = get_from_dict(&kwds, "latency")?;
                let server_name = get_from_dict(&kwds, "server_name")?;
                let server_version = get_from_dict(&kwds, "server_version")?;
                let room_count = get_from_dict(&kwds, "room_count")?;
                let player_count = get_from_dict(&kwds, "player_count")?;
                PingResult {
                    endpoint: endpointw.into(),
                    latency,
                    server_name,
                    server_version,
                    room_count,
                    player_count,
                }
            }
            "newgameupdates" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                vec_from_py! {let updates: Vec<GameUpdate> <- [GameUpdateW] <- get_from_dict(&kwds, "updates")?};
                NewGameUpdates {
                    endpoint: endpointw.into(),
                    updates,
                }
            }
            "newchats" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                vec_from_py! {
                    let messages: Vec<BroadcastChatMessage> <- [BroadcastChatMessageW]
                        <- get_from_dict(&kwds, "messages")?
                };
                NewChats {
                    endpoint: endpointw.into(),
                    messages,
                }
            }
            "newrequestaction" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let actionw: RequestActionW = get_from_dict(&kwds, "action")?;
                NewRequestAction {
                    endpoint: endpointw.into(),
                    action:   actionw.into(),
                }
            }
            "newresponsecode" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                let codew: ResponseCodeW = get_from_dict(&kwds, "code")?;
                NewResponseCode {
                    endpoint: endpointw.into(),
                    code:     codew.into(),
                }
            }
            "endpointtimeout" => {
                let endpointw: EndpointW = get_from_dict(&kwds, "endpoint")?;
                EndpointTimeout {
                    endpoint: endpointw.into(),
                }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(FilterNoticeW { inner: fc })
    }

    #[getter]
    fn get_variant(&self) -> &str {
        use FilterNotice::*;
        match self.inner {
            HasGeneration { .. } => "HasGeneration",
            NewGenStateDiff { .. } => "NewGenStateDiff",
            PingResult { .. } => "PingResult",
            NewGameUpdates { .. } => "NewGameUpdates",
            NewChats { .. } => "NewChats",
            NewRequestAction { .. } => "NewRequestAction",
            NewResponseCode { .. } => "NewResponseCode",
            EndpointTimeout { .. } => "EndpointTimeout",
        }
    }

    #[getter]
    fn get_latency(&self) -> Option<u64> {
        match self.inner {
            FilterNotice::PingResult { latency, .. } => latency,
            _ => None,
        }
    }

    #[getter]
    fn get_server_name(&self) -> Option<&str> {
        match self.inner {
            FilterNotice::PingResult { ref server_name, .. } => Some(server_name),
            _ => None,
        }
    }

    #[getter]
    fn get_room_count(&self) -> Option<u64> {
        match self.inner {
            FilterNotice::PingResult { room_count, .. } => Some(room_count),
            _ => None,
        }
    }

    #[getter]
    fn get_request_action(&self) -> Option<RequestActionW> {
        match self {
            FilterNoticeW {
                inner: FilterNotice::NewRequestAction { action, .. },
            } => Some(action.clone().into()),
            _ => None,
        }
    }

    #[getter]
    fn get_response_code(&self) -> Option<ResponseCodeW> {
        match self {
            FilterNoticeW {
                inner: FilterNotice::NewResponseCode { code, .. },
            } => Some(code.clone().into()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}
