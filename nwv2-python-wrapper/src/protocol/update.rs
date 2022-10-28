use pyo3::exceptions::*;
use pyo3::prelude::*;

use netwaystev2::protocol::BroadcastChatMessage;

#[pyclass]
#[derive(Clone, Debug)]
pub struct BroadcastChatMessageW {
    inner: BroadcastChatMessage,
}

impl Into<BroadcastChatMessage> for BroadcastChatMessageW {
    fn into(self) -> BroadcastChatMessage {
        self.inner
    }
}

impl From<BroadcastChatMessage> for BroadcastChatMessageW {
    fn from(other: BroadcastChatMessage) -> Self {
        BroadcastChatMessageW { inner: other }
    }
}

#[pymethods]
impl BroadcastChatMessageW {
    #[new]
    fn new(chat_seq: Option<u64>, player_name: String, message: String) -> PyResult<Self> {
        let inner = BroadcastChatMessage {
            chat_seq,
            player_name,
            message,
        };
        Ok(BroadcastChatMessageW { inner })
    }

    #[getter]
    fn get_chat_seq(&self) -> Option<u64> {
        self.inner.chat_seq
    }

    #[getter]
    fn get_player_name(&self) -> String {
        self.inner.player_name
    }

    #[getter]
    fn get_message(&self) -> String {
        self.inner.message
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}
