use anyhow::{anyhow, Result};
use reqwest;
use serde::Serialize;
use std::time::Duration;
use thiserror;
use tokio::time as TokioTime;

pub const REGISTER_INTERVAL: Duration = Duration::from_millis(7 * 3600 * 1000);
pub const REGISTER_RETRIES: usize = 3;
pub const REGISTER_RETRY_SLEEP: Duration = Duration::from_millis(5000);
pub const REGISTRY_DEFAULT_URL: &str = "https://registry.conwayste.rs/addServer";

#[derive(Debug, thiserror::Error)]
pub enum ServerRegistrationError {
    #[error("failed to register server with registrar: StatusCode {status}")]
    RegistrationFailed { status: reqwest::StatusCode },
}

#[derive(Debug, Clone)]
pub struct RegistryParams {
    /// The value sent to the registrar
    pub public_addr: String,

    /// The URL to POST our public address to
    pub registry_url: String,
}

impl RegistryParams {
    fn new(public_addr: String) -> Self {
        RegistryParams {
            public_addr,
            registry_url: REGISTRY_DEFAULT_URL.to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
struct RegisterRequestBody {
    host_and_port: String,
}

async fn register(reg_params: &RegistryParams) -> anyhow::Result<()> {
    let req_body = RegisterRequestBody {
        host_and_port: reg_params.public_addr.clone(),
    };
    let response = reqwest::Client::new()
        .post(reg_params.registry_url.clone())
        .json(&req_body)
        .send()
        .await?;
    debug!("Response from registration attempt: {:?}", response);
    if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!(ServerRegistrationError::RegistrationFailed {
            status: response.status(),
        }));
    }
    Ok(())
}

pub async fn try_register(reg_params: RegistryParams) {
    debug!("attempting to register server with {:?}", reg_params.registry_url);
    for attempt in 1..=REGISTER_RETRIES {
        match register(&reg_params).await {
            Ok(_) => {
                debug!("registration success!");
                break;
            }
            Err(e) => {
                warn!(
                    "Failed to register server (was attempt {} of {}): {:?}",
                    attempt, REGISTER_RETRIES, e
                );
            }
        }
        TokioTime::sleep(REGISTER_RETRY_SLEEP).await;
    }
}
