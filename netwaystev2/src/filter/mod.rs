mod filter;
mod interface;
mod ping;
mod sortedbuffer;
use interface::{RequestAction, ResponseCode};
use sortedbuffer::SequencedMinHeap;

pub use filter::Filter;
pub use interface::{FilterMode, Packet};

enum FilterEndpointData {
    OtherEndClient {
        request_actions: SequencedMinHeap<RequestAction>,
    },
    OtherEndServer {
        response_codes: SequencedMinHeap<ResponseCode>,
    },
}
