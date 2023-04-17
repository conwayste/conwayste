use super::FilterMode;
use crate::common::Endpoint;

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("Filter mode ({mode:?}) is not configured to receive {invalid_data}")]
    UnexpectedData {
        mode:         FilterMode,
        invalid_data: String,
    },
    #[error("Internal Filter layer error: {problem}")]
    InternalError { problem: String },
    #[error("Filter does not contain an entry for the endpoint: {endpoint:?}")]
    EndpointNotFound { endpoint: Endpoint },
    #[error("Filter is shutting down. Graceful: {graceful}")]
    ShutdownRequested { graceful: bool },
}
