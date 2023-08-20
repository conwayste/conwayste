use tokio::sync::mpsc::{Receiver, Sender};

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

#[derive(Debug, Clone)]
pub enum AppRsp {
    // TODO: Add more commands to retrieve information from the app layer
    RoomsStatus,
}

pub type AppCmdSend = Sender<AppCmd>;
pub type AppCmdRecv = Receiver<AppCmd>;
pub type AppRspSend = Sender<AppRsp>;
pub type AppRspRecv = Receiver<AppRsp>;
