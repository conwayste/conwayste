use std::num::Wrapping;

pub type SeqNum = Wrapping<u64>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum FilterMode {
    Client,
    Server,
}
