use anyhow::{anyhow, Result};

use crate::app::server::rooms::ROOMS_PER_SERVER;
use crate::Endpoint;

// TODO: Move to config
// Sized to scale with the number of rooms the server supports. Each room supports a number of players and spectators
// that can watch them. Additionally, there are spots reserved in the lobby so that prospective players may queue up for
// a match even if all rooms are full.
const SPECTATORS_PER_ROOM: usize = 2;
const PLAYERS_PER_ROOM: usize = 2;
const LOBBY_SPARING: usize = 20;
pub(crate) const PLAYERS_PER_SERVER: usize =
    ROOMS_PER_SERVER * (SPECTATORS_PER_ROOM + PLAYERS_PER_ROOM) + LOBBY_SPARING;
pub(crate) const MAX_PLAYER_NAME_LEN: usize = 32;

pub struct Player {
    pub name:     String,
    pub endpoint: Endpoint,
}

impl Player {
    pub fn new(name: String, endpoint: Endpoint) -> Self {
        Self { name, endpoint }
    }
}

pub struct LobbyPlayers {
    players: Vec<Player>,
}

impl LobbyPlayers {
    pub fn new() -> Self {
        Self {
            players: Vec::with_capacity(LOBBY_SPARING),
        }
    }

    pub fn add_player(&mut self, name: String, endpoint: Endpoint) -> Result<()> {
        self.players.push(Player::new(name, endpoint));
        Ok(())
    }

    pub fn remove_player(&mut self, name: String) -> Result<()> {
        self.players.retain(|p| p.name != name);

        Ok(())
    }
}
