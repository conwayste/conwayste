use std::time::Duration;

pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 2016;
pub const TRANSPORT_CHANNEL_LEN: usize = 1000;
pub const FILTER_CHANNEL_LEN: usize = 1000;
pub const APP_CHANNEL_LEN: usize = 100;
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_RETRY_INTERVAL: Duration = Duration::from_millis(250);
pub const DEFAULT_ENDPOINT_TIMEOUT_INTERVAL: Duration = Duration::from_secs(5);
pub const TRANSPORT_RETRY_COUNT_LOG_THRESHOLD: usize = 10;
