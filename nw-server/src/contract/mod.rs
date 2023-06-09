use serde::{Serialize, Deserialize};

pub const MAX_CONTROL_MESSAGE_LEN: usize = 128;

#[derive(Debug, Serialize, Deserialize)]
pub enum DaemonStatus {
    Failure,
    Success,
}

#[derive(Serialize, Deserialize)]
pub struct DaemonResponse {
    pub status: DaemonStatus,
    pub message: String,
}


// Not used by netwaystectl
#[allow(unused)]
impl DaemonResponse {
    pub fn success(message: &str) -> DaemonResponse {
        DaemonResponse {
            status: DaemonStatus::Success,
            message: message.to_owned(),
        }
    }

    pub fn failure(message: &str) -> DaemonResponse {
        DaemonResponse {
            status: DaemonStatus::Failure,
            message: message.to_owned(),
        }
    }
}
