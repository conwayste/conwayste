pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 2016;
pub const TRANSPORT_CHANNEL_LEN: usize = 10;
pub const FILTER_CHANNEL_LEN: usize = 10;
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_RETRY_INTERVAL_US: u32 = 100*1000; // 100us
