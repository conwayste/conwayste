///! Unlike the other top-level modules here, this is not a layer. Instead, it describes the
///! UDP protocol (serialized with bincode).

mod packet;
mod request;
mod response;
mod update;


pub use packet::Packet;
pub use request::RequestAction;
pub use response::ResponseCode;
