use anyhow::anyhow;
use std::collections::{hash_map::Entry, HashMap};

use super::{FilterNotice, FilterNotifySend};
use crate::common::Endpoint;
#[allow(unused)] // ToDo: need this?
use crate::protocol::{BroadcastChatMessage, GameUpdate, GenPartInfo, GenStateDiffPart, UniUpdate};
#[allow(unused)] // ToDo: need this?
use conway::{BigBang, GenStateDiff, Pattern, PlayerBuilder, PlayerID, Region, Universe};

pub struct ClientRoom {
    player_name:       String,   // Duplicate of player_name from OtherEndServer (parent) struct
    player_id:         PlayerID, // If player is not a lurker, must be Some(...) before the first GenStateDiff
    pub other_players: HashMap<String, PlayerID>, // Other players: player_name => player_id (None means lurker)
    pub game:          Option<ClientGame>,
    pub last_chat_seq: Option<u64>, // sequence number of latest chat msg. received from server
}

pub struct ClientGame {
    player_id:         PlayerID, // Duplicate of player_id from ClientRoom
    diff_parts:        HashMap<(u32, u32), Vec<Option<String>>>,
    universe:          Universe,
    pub last_full_gen: Option<u64>, // generation number client is currently at
    pub partial_gen:   Option<GenPartInfo>,
}

impl ClientRoom {
    pub async fn process_game_update(
        &mut self,
        server_endpoint: Endpoint,
        game_update: &GameUpdate,
        filter_notice_tx: &FilterNotifySend,
    ) -> anyhow::Result<()> {
        use GameUpdate::*;
        // First, special handling for some of these
        match game_update {
            GameStart { options } => {
                let mut big_bang = BigBang::new()
                    .width(options.width as usize)
                    .height(options.height as usize)
                    .server_mode(false)
                    .history(options.history as usize)
                    .fog_radius(options.fog_radius as usize);

                for net_region in options.player_writable.iter() {
                    let region: Region = net_region.clone().into();
                    big_bang = big_bang.add_player(PlayerBuilder::new(region));
                }
                let uni = big_bang.birth()?;
                let game = ClientGame {
                    player_id:     self.player_id,
                    diff_parts:    HashMap::new(),
                    universe:      uni,
                    last_full_gen: None,
                    partial_gen:   None,
                };
                self.game = Some(game);
            }
            GameFinish { .. } => {
                self.game = None;
                self.player_id = None;
                let mut new_other_players = HashMap::new();
                for name in self.other_players.keys() {
                    new_other_players.insert(name.clone(), None);
                }
                self.other_players = new_other_players;
            }

            PlayerList { players } => {
                for player in players {
                    if player.name == self.player_name {
                        // Hey, that's us! The game is starting or finishing, so we are likely
                        // going between None and Some(...).
                        self.change_own_player_id(player.index);
                    }
                }
            }

            PlayerChange { player, old_name } => {
                if old_name.is_some() {
                    unimplemented!("player name changes not implemented"); // TODO
                }
                if player.name == self.player_name {
                    // Hey, that's us! The game is starting or finishing, so we are likely
                    // going between None and Some(...).
                    self.change_own_player_id(player.index);
                }
            }
            PlayerJoin { player } => {
                if player.name == self.player_name {
                    warn!("[F] ignoring GameUpdate::PlayerJoin for ourselves");
                } else {
                    self.other_players
                        .insert(player.name.clone(), player.index.map(|idx| idx as usize));
                }
            }
            PlayerLeave { name } => {
                if *name == self.player_name {
                    warn!("[F] ignoring GameUpdate::PlayerLeave for ourselves");
                } else {
                    self.other_players.remove(name);
                }
            }
            _ => {}
        }

        // Send it on up
        filter_notice_tx
            .send(FilterNotice::NewGameUpdates {
                endpoint: server_endpoint,
                updates:  vec![game_update.clone()],
            })
            .await?;

        Ok(())
    }

    pub async fn process_chats(
        &mut self,
        server_endpoint: Endpoint,
        chats: &[BroadcastChatMessage],
        filter_notice_tx: &FilterNotifySend,
    ) -> anyhow::Result<()> {
        let mut to_send = vec![];

        // Figure out which chats are new
        for chat in chats {
            if chat.chat_seq.is_none() {
                return Err(anyhow!("chat_seq in message is unexpectedly None: {:?}", chat));
            }
            let chat_seq = chat.chat_seq.unwrap(); // unwrap OK because of above check
            if let Some(ref mut last_chat_seq) = self.last_chat_seq {
                if *last_chat_seq < chat_seq {
                    *last_chat_seq = chat_seq;
                    to_send.push(chat.clone());
                }
            } else {
                self.last_chat_seq = Some(chat_seq);
                to_send.push(chat.clone());
            }
        }

        if to_send.is_empty() {
            return Ok(());
        }

        // Send them on up
        filter_notice_tx
            .send(FilterNotice::NewChats {
                endpoint: server_endpoint,
                messages: to_send,
            })
            .await?;
        Ok(())
    }

    // This field is duplicated so should be saved to the ClientGame if one is in progress.
    fn change_own_player_id(&mut self, player_id: Option<u64>) {
        self.player_id = player_id.map(|idx| idx as usize);
        if let Some(ref mut game) = self.game {
            // Copy
            game.player_id = self.player_id;
        }
    }

    pub fn new(player_name: String) -> Self {
        ClientRoom {
            player_name,
            player_id: None,
            other_players: HashMap::new(),
            game: None,
            last_chat_seq: None,
        }
    }
}

impl ClientGame {
    pub fn process_genstate_diff_part(&mut self, universe_update: UniUpdate) -> anyhow::Result<Option<GenStateDiff>> {
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

        // TODO: only build the `diff` string if we already know all_parts_are_some
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
            let opt_gen = self.universe.apply(&genstatediff, self.player_id)?;
            if let Some(latest_gen) = opt_gen {
                // We have a new generation in the Universe
                if latest_gen != gen1 as usize {
                    warn!(
                        "[F] expected latest generation to be {} but it was {}",
                        gen1, latest_gen
                    );
                }

                // Remove all from diff_parts where gen1 <= latest_gen because they're outdated
                self.diff_parts
                    .retain(|&(_gen0, gen1), _current_parts| gen1 as usize > latest_gen);
            } else {
                // * `Ok(None)` if the update is valid but was not applied because either:
                //     - the generation to be applied is already present,
                //     - there is already a greater generation present, or
                //     - the base generation of this diff (that is, `diff.gen0`) could not be found.
                //       A base generation of 0 is a special case -- it is always found.
                self.diff_parts.remove(&(gen0, gen1));
            }

            return Ok(Some(genstatediff));
        }

        Ok(None)
    }
}
