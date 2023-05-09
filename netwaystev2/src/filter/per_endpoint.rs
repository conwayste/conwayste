use std::{
    collections::{HashMap, VecDeque},
    num::Wrapping,
    time::Duration,
};

use anyhow::anyhow;
use snowflake::ProcessUniqueId;
use tokio::sync::mpsc::{error::SendError, Sender};

use crate::common::Endpoint;
use crate::protocol::{GameUpdate, Packet, RequestAction, ResponseCode};
use crate::transport::{PacketSettings, TransportCmd, TransportCmdSend};
#[allow(unused)]
use crate::{nwdebug, nwerror, nwinfo, nwtrace, nwwarn};

use super::client_update::ClientRoom;
use super::interface::{FilterMode, SeqNum};
use super::sortedbuffer::SequencedMinHeap;
use super::{FilterError, FilterNotifySend, PingPong};

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
    pub cookie: Option<String>,
}

impl OtherEndClient {
    pub fn new(endpoint: Endpoint) -> Self {
        OtherEndClient {
            endpoint,
            request_actions: SequencedMinHeap::<RequestAction>::new(),
            last_request_sequence_seen: None,
            last_response_sequence_sent: None,
            unacked_response_codes: VecDeque::new(),
            cookie: None,
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
    // Update/UpdateReply below
    pub room: Option<ClientRoom>,
    pub game_update_seq: Option<u64>, // When a player enters or leaves a room, this gets reset to None
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
        let mut start_idx = None;
        // We are comparing game update sequence number to what the server just sent us to decide
        // what game updates have we already processed, what game updates we can process now, and
        // what updates are too far ahead to be processed.
        match (self.game_update_seq, game_update_seq) {
            (None, None) => {} // No-op
            (Some(_), None) => {
                // We previously had Some(...), but the server just sent None -- reset!
                debug!("c[F] reset game_update_seq");
                self.game_update_seq = None;
            }
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
        if let Some(_start_idx) = start_idx {
            if self.room.is_none() {
                if game_updates.len() == 1 {
                    let game_update = &game_updates[0];
                    match game_update {
                        GameUpdate::Match { room, expire_secs } => {
                            self.process_match(&room, *expire_secs)?;
                            self.game_update_seq.as_mut().map(|seq| *seq += 1);
                            // Increment
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
            for i in (_start_idx as usize)..game_updates.len() {
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
                            warn!(
                                "c[F] got a RoomDeleted but it wasn't the last game update; the rest will be ignored"
                            );
                            break;
                        }
                    }
                }
                self.game_update_seq.as_mut().map(|seq| *seq += 1); // Increment by 1 because we just handled a game update
            }

            if room_deleted {
                self.room = None;
                self.game_update_seq = None;
            }
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
