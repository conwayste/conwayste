extern crate anyhow;
extern crate bincode;
extern crate conway;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde;
extern crate snowflake;
extern crate thiserror;

pub mod common;
pub mod filter;
pub mod protocol;
pub mod app;
mod settings;
pub mod transport;

#[cfg(test)]
mod tests {

    // TODO: put tests here :)
}
