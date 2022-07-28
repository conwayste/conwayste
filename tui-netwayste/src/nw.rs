use crate::StatefulList;
use crate::nw_protocol::*;
use netwaystev2::{
    filter::FilterMode,
};
use strum::IntoEnumIterator;

pub fn create_packet_selection_lists(mode: FilterMode) -> Vec<StatefulList<String>> {
    match mode {
        FilterMode::Client => {
            let client_packets = StatefulList::with_items(vec!["RequestAction".to_owned(), "ResponseCode".to_owned()]);

            let mimic = MimicRequestAction::RequestActionConnect {
                inner: RequestActionConnect::try_from(vec!["hi".to_owned(), "hello".to_owned()]).expect("fail"),
            };

            let ra_list = StatefulList::with_items(RequestAction::iter().map(|ra| ra.to_string()).collect());
            let rc_list = StatefulList::with_items(vec!["RC_one".to_owned(), "RC_two".to_owned()]);

            vec![client_packets, ra_list, rc_list]
        }
        FilterMode::Server => {
            // TODO
            vec![]
        }
    }
}
