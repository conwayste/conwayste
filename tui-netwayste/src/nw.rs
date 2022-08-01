use crate::nw_protocol::*;
use crate::StatefulList;
use mimicry::MimicMetadata;
use netwaystev2::filter::FilterMode;
use strum::IntoEnumIterator;

const MIMIC_CREATION_FAILURE: &'static str = "Mimic creation failed";

pub fn create_packet_selection_lists(mode: FilterMode) -> Vec<StatefulList<String>> {
    match mode {
        FilterMode::Client => {
            let client_packets = StatefulList::with_items(vec!["RequestAction".to_owned(), "ResponseCode".to_owned()]);

            let ra_list = StatefulList::with_items(RequestAction::iter().map(|ra| ra.to_string()).collect());
            let rc_list = StatefulList::with_items(ResponseCode::iter().map(|rc| rc.to_string()).collect());

            vec![client_packets, ra_list, rc_list]
        }
        FilterMode::Server => {
            // TODO
            vec![]
        }
    }
}

pub fn get_mimic_meta_from(mra: &MimicRequestAction) -> Option<&MimicMetadata> {
    match mra {
        MimicRequestAction::RequestActionNone {} => return None,
        MimicRequestAction::RequestActionConnect {inner} => return Some(&inner.meta),
        MimicRequestAction::RequestActionDisconnect {} => return None,
        MimicRequestAction::RequestActionKeepAlive {inner} => return Some(&inner.meta),
        _ => unimplemented!(),
    }
}

pub fn create_request_action_data() -> Vec<MimicRequestAction> {
    vec![
        make_request_action_none(),
        make_request_action_connect(),
        make_request_action_disconnect(),
        make_request_action_keep_alive(),
    ]
}

pub fn make_request_action_none() -> MimicRequestAction {
    MimicRequestAction::RequestActionNone
}

pub fn make_request_action_connect() -> MimicRequestAction {
    MimicRequestAction::RequestActionConnect {
        inner: RequestActionConnect::try_from(vec!["".to_owned(), "".to_owned()]).expect(MIMIC_CREATION_FAILURE),
    }
}

pub fn make_request_action_disconnect() -> MimicRequestAction {
    MimicRequestAction::RequestActionDisconnect
}

pub fn make_request_action_keep_alive() -> MimicRequestAction {
    MimicRequestAction::RequestActionKeepAlive {
        inner: RequestActionKeepAlive::try_from(vec!["0".to_owned()]).expect(MIMIC_CREATION_FAILURE),
    }
}
