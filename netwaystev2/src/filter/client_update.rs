pub struct ClientRoom {
    game: Option<ClientGame>,
}

pub struct ClientGame {
    oldest_have_gen: u32,
    newest_have_gen: u32,
    //XXX not sure about the below
    seen_gen_state_diffs: HashSet<(u32, u32)>, // Nearly eliminates dupes from going to app layer; elements: (gen0, gen1) from received GenStateDiffParts
    diff_parts: HashMap<(u32, u32), Vec<Option<GenStateDiffPart>>>,
}
