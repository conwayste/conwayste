use anyhow::{anyhow, Result};
use std::collections::{hash_map::Entry, HashMap};

use crate::protocol::{GameUpdate, GenPartInfo, GenStateDiffPart, UniUpdate};
use conway::{
    rle::Pattern,
    universe::{GenStateDiff, Universe},
};

pub struct ClientRoom {
    player_name:       String, // Duplicate of player_name from OtherEndServer (parent) struct
    pub game:          Option<ClientGame>,
    pub last_chat_seq: Option<u64>, // sequence number of latest chat msg. received from server
}

pub struct ClientGame {
    player_id:         usize,
    diff_parts:        HashMap<(u32, u32), Vec<Option<String>>>,
    universe:          Universe,
    pub last_full_gen: Option<u64>, // generation number client is currently at
    pub partial_gen:   Option<GenPartInfo>,
}

impl ClientRoom {
    pub fn process_game_update(&mut self, game_update: &GameUpdate) -> Result<()> {
        Ok(()) //XXX
    }
}

impl ClientGame {
    pub fn process_genstate_diff_part(&mut self, universe_update: UniUpdate) -> Result<Option<GenStateDiff>> {
        let genstate_diff_part;
        match universe_update {
            UniUpdate::NoChange => return Ok(None),
            UniUpdate::Diff { diff } => {
                genstate_diff_part = diff;
            }
        }

        const MAX_DIFF_PARTS: u8 = 32;

        if genstate_diff_part.total_parts > MAX_DIFF_PARTS {
            // TODO: Add a real error code via thisError
            return Err(anyhow!("Total parts exceeds limit"));
        }

        let gen0 = genstate_diff_part.gen0;
        let gen1 = genstate_diff_part.gen1;
        match self.diff_parts.entry((gen0, gen1)) {
            Entry::Vacant(entry) => {
                let mut new_parts = vec![];
                for i in 0..genstate_diff_part.total_parts {
                    if i == genstate_diff_part.part_number {
                        new_parts.push(Some(genstate_diff_part.pattern_part.clone()));
                    } else {
                        new_parts.push(None);
                    }
                }
                entry.insert(new_parts);
            }
            Entry::Occupied(mut entry) => {
                let current_parts = entry.get_mut();
                if current_parts.len() != genstate_diff_part.total_parts as usize {
                    // TODO: Add a real error code via thisError
                    return Err(anyhow!("Total parts do not match"));
                }
                if current_parts.len() <= genstate_diff_part.part_number as usize {
                    // TODO: Add a real error code via thisError
                    return Err(anyhow!("Part number out of range"));
                }
                current_parts[genstate_diff_part.part_number as usize] = Some(genstate_diff_part.pattern_part.clone());
            }
        }

        let mut diff = "".to_owned();
        let mut all_parts_are_some = true;
        if let Some(entry) = self.diff_parts.get(&(gen0, gen1)) {
            for part in entry {
                if let Some(part_string) = part {
                    diff.push_str(part_string);
                } else {
                    all_parts_are_some = false;
                    break;
                }
            }
        }

        if all_parts_are_some {
            let genstatediff = GenStateDiff {
                gen0:    gen0 as usize,
                gen1:    gen1 as usize,
                pattern: Pattern(diff),
            };
            let opt_gen = self.universe.apply(&genstatediff, Some(self.player_id))?;
            if let Some(latest_gen) = opt_gen {
                //XXX store this
            }
            //XXX error handling

            self.diff_parts.remove(&(gen0, gen1));
            //XXX delete stuff from diff_parts

            return Ok(Some(genstatediff));
        }

        Ok(None)
    }
}
