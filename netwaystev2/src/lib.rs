extern crate anyhow;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate bincode;
extern crate serde;

pub mod common;
mod settings;
pub mod transport;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
