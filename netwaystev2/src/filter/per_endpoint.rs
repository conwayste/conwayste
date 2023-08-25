use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::Wrapping,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail};
use bincode::serialized_size;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::{error::SendError, Sender};

use super::server_update::*;
use crate::common::{Endpoint, UDP_MTU_SIZE};
use crate::filter::FilterNotice;
use crate::protocol::{
    BroadcastChatMessage, GameUpdate, GenPartInfo, GenStateDiffPart, Packet, RequestAction, ResponseCode, UniUpdate,
};
use crate::transport::{PacketSettings, TransportCmd, TransportCmdSend};
#[allow(unused)]
use crate::{nwdebug, nwerror, nwinfo, nwtrace, nwwarn};

use super::client_update::ClientRoom;
use super::interface::{FilterMode, SeqNum};
use super::sortedbuffer::SequencedMinHeap;
use super::{FilterError, FilterNotifySend, PingPong};

/// Maximum size of an update packet containing GameUpdates; 90% of MTU size to account for overhead
const MAX_GU_SIZE: u64 = UDP_MTU_SIZE as u64 * 90 / 100;

const GAME_UPDATE_RETRY_INTERVAL: Duration = Duration::from_millis(200);
const GEN_STATE_DIFF_RETRY_INTERVAL: Duration = Duration::from_millis(30);

pub(crate) enum FilterEndpointData {
    OtherEndClient(OtherEndClient),
    OtherEndServer(OtherEndServer),
}

/// This is the server's representation of clients connected to it.
pub(crate) struct OtherEndClient {
    endpoint: Endpoint,
    pub request_actions: SequencedMinHeap<RequestAction>,
    pub last_request_sequence_seen: Option<SeqNum>,
    pub last_response_sequence_sent: Option<SeqNum>,
    pub unacked_response_codes: VecDeque<ResponseCode>, // the back has sequence `last_response_sequence_sent`
    gen_state_packet_ids: Vec<ProcessUniqueId>,
    pub cookie: Option<String>,
    // Update/UpdateReply below
    room: Option<ServerRoom>,
    //lobby_game_updates: UnackedQueue<GameUpdate>,
    old_room_game_updates: UnackedQueue<GameUpdate>, // If player in lobby and this isn't empty, send only this first
    game_update_packet_ids: Vec<ProcessUniqueId>,
    pub auto_response_seqs: VecDeque<u64>, // Response sequences for replying to client KeepAlives with OK
    pub app_response_seqs: VecDeque<u64>,  // Response sequences waiting on App layer to provide ResponseCodes for
}

impl OtherEndClient {
    pub fn new(endpoint: Endpoint) -> Self {
        OtherEndClient {
            endpoint,
            request_actions: SequencedMinHeap::<RequestAction>::new(),
            last_request_sequence_seen: None,
            last_response_sequence_sent: None,
            unacked_response_codes: VecDeque::new(),
            gen_state_packet_ids: vec![],
            cookie: None,
            room: None,
            //lobby_game_updates: UnackedQueue::new(),
            old_room_game_updates: UnackedQueue::new(),
            game_update_packet_ids: Vec::new(),
            auto_response_seqs: VecDeque::new(),
            app_response_seqs: VecDeque::new(),
        }
    }

    /// Update whatever client state the server-side Filter layer needs to keep track of.
    pub fn process_request_action(&mut self, action: &RequestAction) {
        match action {
            RequestAction::LeaveRoom => {
                // ToDo: more cleanup
                self.room = None;
            }
            _ => {}
        }
    }

    //XXX add comment like send_game_updates
    pub async fn send_chats(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        new_chats: &[BroadcastChatMessage],
    ) -> anyhow::Result<()> {
        //XXX
        Ok(())
    }

    /// Returns error if there is a RoomDeleted but it is not the only update in the array!
    ///
    /// This can be called with an empty slice of GameUpdates, which ensures any old
    /// GameUpdate-containing Update packets are dropped, and that new packets are sent containing
    /// any unacked GameUpdates.
    pub async fn send_game_updates(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        new_updates: &[GameUpdate],
    ) -> anyhow::Result<()> {
        if new_updates.iter().any(|game_update| game_update.is_game_finish()) {
            // Clear game-specific state.
            self.finish_game(transport_cmd_tx).await?;
        }

        // This complicated logic sucks but hopefully we don't hit any crazy edge cases...
        if new_updates.contains(&GameUpdate::RoomDeleted) {
            if new_updates.len() == 1 {
                // Move any remaining game updates in the room to a high priority holding place,
                // delete the room, and add the room deletion as the last game update for this room.
                if let Some(ref mut room) = self.room.as_mut() {
                    std::mem::swap(&mut self.old_room_game_updates, &mut room.game_updates);
                }
                self.room = None;
                self.old_room_game_updates.push(GameUpdate::RoomDeleted);
            } else {
                // Not ever going to implement. Not worth it
                bail!("called with RoomDeleted and one or more other GameUpdates! Not implemented.");
            }
        } else {
            // ToDo: implement support for GameUpdates in a lobby (not from old room)
            if let Some(ref mut room) = self.room.as_mut() {
                for update in new_updates {
                    room.game_updates.push(update.clone());
                }
            } else if !new_updates.is_empty() {
                bail!("in-lobby game updates not implemented"); // ToDo
            }
        }

        // Drop old packet(s)
        for packet_id in self.game_update_packet_ids.drain(..) {
            transport_cmd_tx
                .send(TransportCmd::DropPacket {
                    endpoint: self.endpoint,
                    tid:      packet_id,
                })
                .await?;
        }

        let unacked = if self.room.is_none() && !self.old_room_game_updates.is_empty() {
            self.old_room_game_updates.get() // Higher priority UnackedQueue
        } else if let Some(ref room) = self.room.as_ref() {
            if !self.old_room_game_updates.is_empty() {
                warn!("Entered a room with unacked game updates from previous room :(");
            }
            // Happy path
            room.game_updates.get()
        } else {
            // In lobby, and no updates from old room
            return Ok(()); // Nothing to do
        };

        // Split up the unacked updates into multiple packets according to max. size.
        let mut size_of_last_vec = 0;
        let mut groups: Vec<(u64, Vec<GameUpdate>)> = vec![];
        for (seq, update) in unacked.into_iter() {
            let size = serialized_size(&update)?;
            if groups.is_empty() || size_of_last_vec + size > MAX_GU_SIZE {
                size_of_last_vec = size;
                groups.push((seq, vec![update]));
            } else {
                size_of_last_vec += size;
                groups.last_mut().unwrap().1.push(update);
            }
        }

        if groups.is_empty() {
            return Ok(());
        }

        // Send the new packet(s) and save IDs for later dropping.
        let mut packets = vec![];
        for group in groups.into_iter() {
            packets.push(Packet::Update {
                chats:           vec![],
                game_update_seq: Some(group.0),
                game_updates:    group.1,
                universe_update: UniUpdate::NoChange,
                ping:            PingPong::ping(),
            });
        }
        let mut packet_infos = vec![];
        for _ in 0..packets.len() {
            let tid = ProcessUniqueId::new();
            packet_infos.push(PacketSettings {
                tid,
                retry_interval: GAME_UPDATE_RETRY_INTERVAL,
            });
            self.game_update_packet_ids.push(tid);
        }
        transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint: self.endpoint,
                packet_infos,
                packets,
            })
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn process_update_reply(
        &mut self,
        last_chat_seq: Option<u64>,
        last_game_update_seq: Option<u64>,
        last_full_gen: Option<u64>,
        partial_gen: Option<&GenPartInfo>,
        transport_cmd_tx: &Sender<TransportCmd>,
        filter_notice_tx: &Sender<FilterNotice>,
    ) -> anyhow::Result<()> {
        // Process all of the UpdateReply components
        self.process_chat_ack(last_chat_seq).await?;
        self.process_game_update_ack(last_game_update_seq, transport_cmd_tx)
            .await?;
        self.process_gen_ack(last_full_gen, partial_gen, transport_cmd_tx, filter_notice_tx)
            .await?;
        Ok(())
    }

    async fn process_chat_ack(&mut self, last_chat_seq: Option<u64>) -> anyhow::Result<()> {
        if last_chat_seq.is_none() {
            return Ok(());
        }
        //XXX maybe remove outgoing chats from "unacked" data structure, drop Update packet (if any), and
        // potentially send new Update packet
        Ok(())
    }

    /// Using game update sequence received from client, potentially drop and resend Update
    /// packet(s) containing GameUpdate(s).
    async fn process_game_update_ack(
        &mut self,
        last_game_update_seq: Option<u64>,
        transport_cmd_tx: &Sender<TransportCmd>,
    ) -> anyhow::Result<()> {
        if last_game_update_seq.is_none() {
            return Ok(());
        }

        if let Some(ref mut room) = self.room.as_mut() {
            if !self.old_room_game_updates.is_empty() {
                warn!(
                    "In room, but {} game updates from old room",
                    self.old_room_game_updates.len()
                );
            }
            room.game_updates.ack(last_game_update_seq);
        } else {
            // In lobby, but unacked from old room -- maybe room was deleted?
            self.old_room_game_updates.ack(last_game_update_seq);
        }

        // Calling this with empty slice to ensure any old GameUpdate-containing Update packets
        // were dropped, and new packets are sent containing any unacked GameUpdates.
        self.send_game_updates(transport_cmd_tx, &[]).await
    }

    pub async fn send_gen_state_diff(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        gen0: usize,
        gen1: usize,
        diff: Vec<Option<Arc<GenStateDiffPart>>>,
    ) -> anyhow::Result<()> {
        if let Some(ref mut room) = self.room.as_mut() {
            // Insert into parts, drop old packets (if any), and send new.
            room.unacked_gsd_parts.insert((gen0, gen1), diff);
            self.drop_all_gen_state_diff_packets(transport_cmd_tx).await?;
            self.send_gen_state_diffs(transport_cmd_tx).await?;
        } else {
            warn!(
                "s[F<-A] attempted to send GenStateDiff to client ({:?}) not in room",
                self.endpoint
            );
        }
        Ok(())
    }

    async fn process_gen_ack(
        &mut self,
        last_full_gen: Option<u64>,
        partial_gen: Option<&GenPartInfo>,
        transport_cmd_tx: &Sender<TransportCmd>,
        filter_notice_tx: &Sender<FilterNotice>,
    ) -> anyhow::Result<()> {
        if let Some(ref mut room) = self.room.as_mut() {
            if last_full_gen.is_none() && partial_gen.is_none() {
                return Ok(());
            }

            if let Some(last_full_gen) = last_full_gen {
                if last_full_gen as usize > room.latest_gen {
                    bail!(
                        "Outdated packet or client misbehaving: client reports last full gen {} but server's is {}",
                        last_full_gen,
                        room.latest_gen
                    );
                }
                if last_full_gen as usize > room.latest_gen_client_has {
                    filter_notice_tx
                        .send(FilterNotice::HasGeneration {
                            endpoints: vec![self.endpoint],
                            gen_num:   last_full_gen,
                        })
                        .await?;
                    room.latest_gen_client_has = last_full_gen as usize;
                    // Remove any diffs where the "to" generation is the highest the client has, or lower.
                    room.unacked_gsd_parts
                        .retain(|(_gen0, gen1), _parts| *gen1 > last_full_gen as usize);
                    self.drop_all_gen_state_diff_packets(transport_cmd_tx).await?;
                    // Return because no point continuing on to process a partial gen update
                    return self.send_gen_state_diffs(transport_cmd_tx).await;
                } else if (last_full_gen as usize) < room.latest_gen_client_has {
                    return Ok(()); // Old packet; don't process partial gen updates
                }
            }

            if let Some(partial_gen) = partial_gen {
                let (gen0, gen1) = (partial_gen.gen0 as usize, partial_gen.gen1 as usize);
                if let Some(parts) = room.unacked_gsd_parts.get_mut(&(gen0, gen1)) {
                    let mut changed = false;
                    let mut some_count = 0;
                    for i in 0..parts.len() {
                        if partial_gen.have_bitmask & (1 << i) != 0 {
                            // Client has this part
                            parts[i] = None;
                            changed = true;
                        }
                        if parts[i].is_some() {
                            some_count += 1;
                        }
                    }
                    if some_count == 0 {
                        room.unacked_gsd_parts.remove(&(gen0, gen1));
                        changed = true;
                    }
                    if changed {
                        // ToDo: consider skipping these two calls if we sent packets recently
                        // enough that the missing GSDP packet(s) could still be in transit.
                        // However, it might be even better to keep track of the tid for each
                        // packet and only dropping and resending the acked parts -- this would
                        // avoid resending packets too often.
                        self.drop_all_gen_state_diff_packets(transport_cmd_tx).await?;
                        self.send_gen_state_diffs(transport_cmd_tx).await?;
                    }
                }
            }
            Ok(())
        } else {
            self.drop_all_gen_state_diff_packets(transport_cmd_tx).await
        }
    }

    async fn drop_all_gen_state_diff_packets(&mut self, transport_cmd_tx: &Sender<TransportCmd>) -> anyhow::Result<()> {
        for packet_id in self.gen_state_packet_ids.drain(..) {
            transport_cmd_tx
                .send(TransportCmd::DropPacket {
                    endpoint: self.endpoint,
                    tid:      packet_id,
                })
                .await?;
        }
        Ok(())
    }

    /// Send all GenStateDiffParts that haven't been acked yet -- with retry
    async fn send_gen_state_diffs(&mut self, transport_cmd_tx: &Sender<TransportCmd>) -> anyhow::Result<()> {
        let room = if let Some(ref room) = self.room {
            room
        } else {
            return Ok(());
        };
        let mut packets = vec![];
        let mut packet_infos = vec![];
        for ((_gen0, _gen1), parts) in room.unacked_gsd_parts.iter() {
            for part in parts.iter() {
                if part.is_none() {
                    continue;
                }
                let part = part.as_ref().unwrap(); // unwrap OK because of above check

                let tid = ProcessUniqueId::new();
                self.gen_state_packet_ids.push(tid);
                packet_infos.push(PacketSettings {
                    tid,
                    retry_interval: GEN_STATE_DIFF_RETRY_INTERVAL,
                });
                let diff = GenStateDiffPart::clone(part);
                packets.push(Packet::Update {
                    chats:           vec![],
                    game_update_seq: None,
                    game_updates:    vec![],
                    universe_update: UniUpdate::Diff { diff },
                    ping:            PingPong::ping(),
                });
            }
        }
        transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint: self.endpoint,
                packet_infos,
                packets,
            })
            .await
            .map_err(|e| anyhow!(e))
    }

    pub fn set_latest_gen(&mut self, latest_gen: usize) {
        if let Some(ref mut room) = self.room.as_mut() {
            room.latest_gen = latest_gen;
        }
    }

    /// The only error this can return is a send error on `transport_cmd_tx`.
    pub async fn resend_and_drop_enqueued_response_codes(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        acked_response_seq: Option<u64>,
    ) -> Result<(), SendError<TransportCmd>> {
        // Check if there's anything to do first; otherwise, exit early
        if self.unacked_response_codes.is_empty() {
            return Ok(());
        } else if self.last_response_sequence_sent.is_none() {
            warn!("[F] Never sent a response code before?!? Possible logic bug!");
            return Ok(());
        }

        let last_response_sequence_sent = self.last_response_sequence_sent.unwrap(); // unwrap OK b/c above

        // Get the response sequence of 0th element of unacked_response_codes
        let mut cur_response_seq = last_response_sequence_sent - Wrapping(self.unacked_response_codes.len() as u64 - 1);

        // Somewhat dumbly written check to see if acked_response_seq in range; not _too_
        // inefficient I guess.
        let mut included = false;
        {
            let mut seq = cur_response_seq;
            for _ in 0..self.unacked_response_codes.len() {
                if let Some(acked_response_seq) = acked_response_seq {
                    if seq.0 == acked_response_seq {
                        included = true;
                        break;
                    }
                }
                seq += Wrapping(1);
            }
        }
        if included {
            let acked_response_seq = acked_response_seq.unwrap(); // unwrap OK because of check above

            // Discard the acknowledged response codes
            loop {
                if self.unacked_response_codes.is_empty() {
                    return Ok(());
                }
                self.unacked_response_codes.pop_front();
                if cur_response_seq.0 == acked_response_seq {
                    // The element just popped had a sequence of `cur_response_seq`. If these are
                    // equal, then it was successfully acknowledged.

                    cur_response_seq += Wrapping(1); // Now it matches response code at front.
                    break;
                }
                cur_response_seq += Wrapping(1);
            }
        }

        if self.unacked_response_codes.is_empty() {
            return Ok(());
        }

        // Resend everything left
        let mut packets = vec![];
        let request_ack = self.last_request_sequence_seen.map(|request_sn| request_sn.0);
        for code in &self.unacked_response_codes {
            packets.push(Packet::Response {
                code: code.clone(),
                sequence: cur_response_seq.0,
                request_ack,
            });
            cur_response_seq += Wrapping(1);
        }

        transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint: self.endpoint,
                packet_infos: no_retry_packet_info_vec(packets.len()),
                packets,
            })
            .await
    }

    /// Send a ResponseCode::OK for a KeepAlive; calling this at the wrong time b0rks everything.
    pub async fn send_keep_alive_response(&mut self, transport_cmd_tx: &Sender<TransportCmd>) -> anyhow::Result<()> {
        if let Some(ref mut sn) = self.last_response_sequence_sent {
            *sn += Wrapping(1u64);
        } else {
            self.last_response_sequence_sent = Some(Wrapping(1));
        }

        // Unwrap ok b/c the immediate check above guarantees Some(..)
        let sequence = self.last_response_sequence_sent.unwrap().0;

        self.do_send_response_code(transport_cmd_tx, ResponseCode::OK, sequence)
            .await
    }

    /// Send a new ResponseCode to the client. Not to be used for re-sends!
    pub async fn send_response_code(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        code: ResponseCode,
    ) -> anyhow::Result<()> {
        if let Some(ref mut sn) = self.last_response_sequence_sent {
            *sn += Wrapping(1u64);
        } else {
            self.last_response_sequence_sent = Some(Wrapping(1));
        }

        // Unwrap ok b/c the immediate check above guarantees Some(..)
        let sequence = self.last_response_sequence_sent.unwrap().0;

        self.do_send_response_code(transport_cmd_tx, code, sequence).await?;
        while !self.auto_response_seqs.is_empty() {
            // unwrap OK because of logic at top of function
            let next_seq = *self.last_response_sequence_sent.as_mut().unwrap() + Wrapping(1u64);
            if Some(&next_seq.0) == self.app_response_seqs.back() {
                // Must wait for App layer reply
                break;
            }
            self.last_response_sequence_sent = Some(next_seq);
            // Unwrap ok b/c the immediate check above guarantees Some(..)
            let sequence = self.last_response_sequence_sent.unwrap().0;
            let expected_seq = self.auto_response_seqs.pop_back().unwrap(); // unwrap OK because of while cond.
            if expected_seq != sequence {
                error!(
                    "s[F] Sending OK response to KeepAlive but sequence mismatch; expected {}, actual {}",
                    expected_seq, sequence
                );
            }
            self.do_send_response_code(transport_cmd_tx, ResponseCode::OK, sequence)
                .await?;
        }
        Ok(())
    }

    async fn do_send_response_code(
        &mut self,
        transport_cmd_tx: &Sender<TransportCmd>,
        code: ResponseCode,
        sequence: u64,
    ) -> anyhow::Result<()> {
        match &code {
            ResponseCode::JoinedRoom { room_name } => {
                self.join_room(room_name);
            }
            _ => {}
        }

        // Save ResponseCode on self for possible re-sending
        self.unacked_response_codes.push_back(code.clone());

        let request_ack = self.last_request_sequence_seen.map(|request_sn| request_sn.0);

        let packets = vec![Packet::Response {
            code,
            sequence,
            request_ack,
        }];

        transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint: self.endpoint,
                packet_infos: no_retry_packet_info_vec(packets.len()),
                packets,
            })
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Create ServerRoom now that player has joined
    pub fn join_room(&mut self, room_name: &str) {
        let room = ServerRoom::new(room_name.into());
        self.room = Some(room);
    }

    pub async fn finish_game(&mut self, transport_cmd_tx: &Sender<TransportCmd>) -> anyhow::Result<()> {
        if let Some(ref mut room) = self.room {
            room.finish_game();
        } else {
            error!("s[F] Tried to finish a game but player in lobby");
        }

        // Drop any GenStateDiffParts sent previously
        self.drop_all_gen_state_diff_packets(transport_cmd_tx).await
    }
}

/// This is the client's representation of the server it is connected to.
pub(crate) struct OtherEndServer {
    pub player_name: String,
    pub cookie: Option<String>,
    // Request/Response below
    pub response_codes: SequencedMinHeap<ResponseCode>,
    pub last_request_sequence_sent: Option<SeqNum>,
    pub last_response_sequence_seen: Option<SeqNum>,
    pub unacked_outgoing_packet_tids: VecDeque<(SeqNum, ProcessUniqueId)>, // Tracks outgoing Requests
    pub auto_request_seqs: HashSet<u64>, // Request sequences for auto-generated RequestActions (KeepAlives)
    // Update/UpdateReply below
    pub room: Option<ClientRoom>,
    pub game_update_seq: Option<u64>,
    pub server_ping: PingPong,
}

impl OtherEndServer {
    pub fn new(player_name: String) -> Self {
        OtherEndServer {
            player_name,
            cookie: None,
            response_codes: SequencedMinHeap::<ResponseCode>::new(),
            last_request_sequence_sent: None,
            last_response_sequence_seen: None,
            unacked_outgoing_packet_tids: VecDeque::new(),
            auto_request_seqs: HashSet::new(),
            room: None,
            game_update_seq: None,
            server_ping: PingPong::pong(0),
        }
    }

    fn process_match(&mut self, _room: &str, _expire_secs: u32) -> anyhow::Result<()> {
        // TODO
        unimplemented!();
    }

    pub async fn send_update_reply(
        &mut self,
        server_endpoint: Endpoint,
        transport_cmd_tx: &mut TransportCmdSend,
    ) -> anyhow::Result<()> {
        let cookie = self
            .cookie
            .as_ref()
            .ok_or_else(|| anyhow!("No cookie so cannot send UpdateReply -- not logged in?"))?
            .clone();
        let mut last_chat_seq = None;
        let mut last_full_gen: Option<u64> = None;
        let mut partial_gen = None;
        if let Some(ref room) = self.room {
            last_chat_seq = room.last_chat_seq;
            if let Some(ref game) = room.game {
                last_full_gen = game.last_full_gen.map(|gen| gen as u64);
                partial_gen = game.partial_gen.clone();
            }
        }
        let packets = vec![Packet::UpdateReply {
            cookie,
            last_chat_seq,
            last_game_update_seq: self.game_update_seq,
            last_full_gen,
            partial_gen,
            pong: self.server_ping,
        }];

        // Only send once
        let packet_infos = vec![PacketSettings {
            tid:            ProcessUniqueId::new(),
            retry_interval: Duration::ZERO,
        }];
        transport_cmd_tx
            .send(TransportCmd::SendPackets {
                endpoint: server_endpoint,
                packet_infos,
                packets,
            })
            .await?;
        Ok(())
    }

    pub async fn process_game_updates(
        &mut self,
        endpoint: Endpoint,
        game_update_seq: Option<u64>,
        game_updates: &[GameUpdate],
        filter_notice_tx: &FilterNotifySend,
    ) -> anyhow::Result<()> {
        let increment_game_update_seq = |_self: &mut Self| {
            if let Some(seq) = _self.game_update_seq.as_mut() {
                *seq += 1;
            } else {
                _self.game_update_seq = Some(1);
            }
        };
        let mut start_idx = None;
        // We are comparing game update sequence number to what the server just sent us to decide
        // what game updates have we already processed, what game updates we can process now, and
        // what updates are too far ahead to be processed.
        match (self.game_update_seq, game_update_seq) {
            (_, None) => {} // No-op
            (None, Some(_)) => {
                start_idx = Some(0);
            }
            (Some(seen_seq), Some(recvd_seq)) => {
                // recvd_seq is the offset of `game_updates` in the sequence that's shared
                // between client and server.
                // seen_seq  |  recvd_seq | meaning
                //    5            7          can't do anything with this -- missing GameUpdate #6
                //    5            6          start processing at index 0 in game_updates
                //    5            5          overlap -- already got GameUpdate #5; start processing at index 1
                //    5            1          overlap -- already got GameUpdate #5; start processing at index 5
                if seen_seq + 1 >= recvd_seq {
                    let i = seen_seq + 1 - recvd_seq;
                    start_idx = if i as usize >= game_updates.len() {
                        // All of these updates were already processed
                        None
                    } else {
                        Some(i)
                    };
                } else {
                    // The start of the `game_updates` server just sent us is missing one
                    // or more that we need next -- in other words, it's too far ahead.
                    start_idx = None;
                }
            }
        }

        let start_idx = if let Some(si) = start_idx {
            si
        } else {
            return Ok(());
        };
        if self.room.is_none() {
            if game_updates.len() == 1 {
                let game_update = &game_updates[0];
                match game_update {
                    GameUpdate::Match { room, expire_secs } => {
                        self.process_match(&room, *expire_secs)?;
                        increment_game_update_seq(self); // Just handled a game update
                    }
                    _ => {
                        return Err(anyhow!("we are in the lobby and got a non-Match game update"));
                    }
                }
            } else {
                return Err(anyhow!(
                    "we are in the lobby and getting more than one game update at a time"
                ));
            }
        }

        // Out of the game updates we got from the server, process the ones we haven't already
        // processed.
        let mut room_deleted = false;
        for i in (start_idx as usize)..game_updates.len() {
            if let Some(ref mut room) = self.room {
                if let Err(e) = room
                    .process_game_update(endpoint, &self.player_name, &game_updates[i], filter_notice_tx)
                    .await
                {
                    error!("c[F] failed to process game update {:?}: {}", game_updates[i], e);
                }

                if game_updates[i].room_was_deleted() {
                    room_deleted = true;
                    if i != game_updates.len() {
                        warn!("c[F] got a RoomDeleted but it wasn't the last game update; the rest will be ignored");
                        break;
                    }
                }
            }
            increment_game_update_seq(self); // Just handled a game update
        }

        if room_deleted {
            self.room = None;
            self.game_update_seq = None;
        }
        Ok(())
    }
}

pub(crate) struct PerEndpoint(HashMap<Endpoint, FilterEndpointData>);

/// Wraps HashMap to avoid borrow error that would occur if `other_end_client_ref_mut` and
/// `other_end_server_ref_mut` were methods of Filter.
#[allow(unused)]
impl PerEndpoint {
    pub fn new() -> Self {
        PerEndpoint(HashMap::new())
    }

    /// Returns a mutable OtherEndClient reference for the specified endpoint. Two possible errors:
    ///
    /// * If it's a server at the other end, return `UnexpectedData` error variant.
    ///
    /// * If there is no record of this endpoint, return `EndpointNotFound` error variant.
    pub fn other_end_client_ref_mut(
        &mut self,
        endpoint: &Endpoint,
        filter_mode: &FilterMode,
        invalid_data: Option<&str>,
    ) -> Result<&mut OtherEndClient, FilterError> {
        let endpoint_data = self.0.get_mut(endpoint).ok_or_else(|| FilterError::EndpointNotFound {
            endpoint: endpoint.clone(),
        })?;
        match endpoint_data {
            FilterEndpointData::OtherEndServer { .. } => {
                return Err(FilterError::UnexpectedData {
                    mode:         filter_mode.clone(),
                    invalid_data: invalid_data
                        .map(|msg| msg.into())
                        .unwrap_or_else(|| "expected other end to be client".into()),
                });
            }
            FilterEndpointData::OtherEndClient(ref mut client) => Ok(client),
        }
    }

    /// Returns a mutable OtherEndServer reference for the specified endpoint. Two possible errors:
    ///
    /// * If it's a client at the other end, return `UnexpectedData` error variant.
    ///
    /// * If there is no record of this endpoint, return `EndpointNotFound` error variant.
    pub fn other_end_server_ref_mut(
        &mut self,
        endpoint: &Endpoint,
        filter_mode: &FilterMode,
        invalid_data: Option<&str>,
    ) -> Result<&mut OtherEndServer, FilterError> {
        let endpoint_data = self.0.get_mut(endpoint).ok_or_else(|| FilterError::EndpointNotFound {
            endpoint: endpoint.clone(),
        })?;
        match endpoint_data {
            FilterEndpointData::OtherEndClient { .. } => {
                return Err(FilterError::UnexpectedData {
                    mode:         filter_mode.clone(),
                    invalid_data: invalid_data
                        .map(|msg| msg.into())
                        .unwrap_or_else(|| "expected other end to be server".into()),
                });
            }
            FilterEndpointData::OtherEndServer(ref mut server) => Ok(server),
        }
    }

    pub fn remove(&mut self, k: &Endpoint) -> Option<FilterEndpointData> {
        self.0.remove(k)
    }

    pub fn contains_key(&self, k: &Endpoint) -> bool {
        self.0.contains_key(k)
    }

    pub fn insert(&mut self, k: Endpoint, v: FilterEndpointData) -> Option<FilterEndpointData> {
        self.0.insert(k, v)
    }

    pub fn get(&self, k: &Endpoint) -> Option<&FilterEndpointData> {
        self.0.get(k)
    }

    pub fn get_mut(&mut self, k: &Endpoint) -> Option<&mut FilterEndpointData> {
        self.0.get_mut(k)
    }

    // More methods from HashMap can be added if needed.
}

/// If no retries are needed for a series of packets, this function can be used to generate
/// packet_infos vec for TransportCmd::SendPackets.
fn no_retry_packet_info_vec(count: usize) -> Vec<PacketSettings> {
    let mut packet_infos = vec![];
    for _ in 0..count {
        packet_infos.push(PacketSettings {
            tid:            ProcessUniqueId::new(),
            retry_interval: Duration::ZERO,
        });
    }
    packet_infos
}
