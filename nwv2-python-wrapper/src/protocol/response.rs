use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::common::*;
use netwaystev2::protocol::{ResponseCode, RoomList};

#[pyclass]
#[derive(Clone, Debug)]
pub struct ResponseCodeW {
    inner: ResponseCode,
}

impl_from_and_to!(ResponseCodeW wraps ResponseCode);

#[pymethods]
impl ResponseCodeW {
    #[new]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let inner = match variant.to_lowercase().as_str() {
            "ok" => ResponseCode::OK,
            "loggedin" => {
                let cookie: String = get_from_dict(&kwds, "cookie")?;
                let server_version: String = get_from_dict(&kwds, "server_version")?;
                ResponseCode::LoggedIn { cookie, server_version }
            }
            "joinedroom" => {
                let room_name: String = get_from_dict(&kwds, "room_name")?;
                ResponseCode::JoinedRoom { room_name }
            }
            "leaveroom" => ResponseCode::LeaveRoom,
            "playerlist" => {
                let players: Vec<String> = get_from_dict(&kwds, "players")?;
                ResponseCode::PlayerList { players }
            }
            "roomlist" => {
                vec_from_py! {let rooms: Vec<RoomList> <- [RoomListW] <- get_from_dict(&kwds, "players")?};
                ResponseCode::RoomList { rooms }
            }
            "badrequest" => {
                let error_msg: String = get_from_dict(&kwds, "error_msg")?;
                ResponseCode::BadRequest { error_msg }
            }
            "unauthorized" => {
                let error_msg: String = get_from_dict(&kwds, "error_msg")?;
                ResponseCode::Unauthorized { error_msg }
            }
            "toomanyrequests" => {
                let error_msg: String = get_from_dict(&kwds, "error_msg")?;
                ResponseCode::TooManyRequests { error_msg }
            }
            "servererror" => {
                let error_msg: String = get_from_dict(&kwds, "error_msg")?;
                ResponseCode::ServerError { error_msg }
            }
            "notconnected" => {
                let error_msg: String = get_from_dict(&kwds, "error_msg")?;
                ResponseCode::NotConnected { error_msg }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(ResponseCodeW { inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct RoomListW {
    inner: RoomList,
}

impl Into<RoomList> for RoomListW {
    fn into(self) -> RoomList {
        self.inner
    }
}

impl From<RoomList> for RoomListW {
    fn from(other: RoomList) -> Self {
        RoomListW { inner: other }
    }
}

#[pymethods]
impl RoomListW {
    #[new]
    fn new(room_name: String, player_count: u8, in_progress: bool) -> PyResult<Self> {
        let inner = RoomList {
            room_name,
            player_count,
            in_progress,
        };
        Ok(RoomListW { inner })
    }

    #[getter]
    fn get_room_name(&self) -> &str {
        &self.inner.room_name
    }

    #[getter]
    fn get_player_count(&self) -> u8 {
        self.inner.player_count
    }

    #[getter]
    fn get_in_progress(&self) -> bool {
        self.inner.in_progress
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}
