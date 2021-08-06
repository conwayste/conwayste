use std::num::Wrapping;

pub type SeqNum = Wrapping<u64>;

/// App layer sends these commands to the Filter layer.
#[derive(Debug)]
pub enum FilterCmd {
    // PR_GATE TODO: many more commands go here: https://github.com/conwayste/conwayste/issues/153

    Shutdown, //XXX
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum FilterMode {
    Client,
    Server,
}
