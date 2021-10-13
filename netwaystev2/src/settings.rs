pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 2016;
pub const TRANSPORT_CHANNEL_LEN: usize = 10;
pub const FILTER_CHANNEL_LEN: usize = 10;
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_RETRY_INTERVAL_NS: u32 = 5 * 1000 * 1000; // 5ms
pub const TRANSPORT_RETRY_COUNT_LOG_THRESHOLD: usize = 10;
