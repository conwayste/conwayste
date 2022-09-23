use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::utils::get_from_dict;
use netwaystev2::protocol::RequestAction;

#[pyclass]
#[derive(Clone)]
pub struct RequestActionW {
    inner: RequestAction,
}

impl Into<RequestAction> for RequestActionW {
    fn into(self) -> RequestAction {
        self.inner
    }
}

#[pymethods]
impl RequestActionW {
    #[new]
    #[args(kwds = "**")]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let ra = match variant.to_lowercase().as_str() {
            "none" => RequestAction::None,
            "connect" => {
                let name: String = get_from_dict(&kwds, "name")?;
                let client_version: String = get_from_dict(&kwds, "client_version")?;
                RequestAction::Connect { name, client_version }
            }
            /* All actions below require a log-in via a Connect request */
            "disconnect" => RequestAction::Disconnect,
            // Send latest response ack on each heartbeat
            "keepalive" => {
                let latest_response_ack = get_from_dict(&kwds, "latest_response_ack")?;
                RequestAction::KeepAlive { latest_response_ack }
            }
            "listplayers" => RequestAction::ListPlayers,
            "chatmessage" => {
                let message = get_from_dict(&kwds, "message")?;
                RequestAction::ChatMessage { message }
            }
            "listrooms" => RequestAction::ListRooms,
            "newroom" => {
                let room_name = get_from_dict(&kwds, "room_name")?;
                RequestAction::NewRoom { room_name }
            }
            "joinroom" => {
                let room_name = get_from_dict(&kwds, "room_name")?;
                RequestAction::JoinRoom { room_name }
            }
            "leaveroom" => RequestAction::LeaveRoom,
            // TODO SetClientOptions (requires a ClientOptionValue)
            // Draw the specified RLE Pattern with upper-left cell at position x, y.
            "droppattern" => {
                let x = get_from_dict(&kwds, "x")?;
                let y = get_from_dict(&kwds, "y")?;
                let pattern = get_from_dict(&kwds, "pattern")?;
                RequestAction::DropPattern { x, y, pattern }
            }
            // Clear all cells in the specified region not belonging to other players. No part of this
            // region may be outside the player's writable region.
            "cleararea" => {
                let x = get_from_dict(&kwds, "x")?;
                let y = get_from_dict(&kwds, "y")?;
                let w = get_from_dict(&kwds, "w")?;
                let h = get_from_dict(&kwds, "h")?;
                RequestAction::ClearArea { x, y, w, h }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(RequestActionW { inner: ra })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// TODO: ClientOptionValue
