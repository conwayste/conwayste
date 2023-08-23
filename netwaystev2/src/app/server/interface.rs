use anyhow::anyhow;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::protocol::RoomStatus;

#[derive(Debug, PartialEq)]
pub enum UniGenCmd {}

#[derive(Debug, PartialEq)]
pub enum UniGenRsp {}

#[derive(Debug, PartialEq)]
pub enum UniGenNotice {}

#[derive(Debug, Clone)]
pub enum AppCmd {
    // TODO: Add more commands to retrieve information from the app layer
    GetRoomsStatus,
}

impl std::convert::TryFrom<String> for AppCmd {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "status" => Ok(AppCmd::GetRoomsStatus),
            _ => Err(anyhow!("Unknown command")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppRsp {
    // TODO: Add more commands to retrieve information from the app layer
    RoomsStatuses(Vec<RoomStatus>),
}

pub type AppCmdSend = Sender<AppCmd>;
pub type AppCmdRecv = Receiver<AppCmd>;
pub type AppRspSend = Sender<AppRsp>;
pub type AppRspRecv = Receiver<AppRsp>;
