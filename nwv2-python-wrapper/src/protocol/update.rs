use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::common::*;
use netwaystev2::protocol::*;

#[pyclass]
#[derive(Clone, Debug)]
pub struct BroadcastChatMessageW {
    inner: BroadcastChatMessage,
}

impl_from_and_to!(BroadcastChatMessageW wraps BroadcastChatMessage);

#[pymethods]
impl BroadcastChatMessageW {
    #[new]
    fn new(chat_seq: Option<u64>, player_name: String, message: String) -> Self {
        let inner = BroadcastChatMessage {
            chat_seq,
            player_name,
            message,
        };
        BroadcastChatMessageW { inner }
    }

    #[getter]
    fn get_chat_seq(&self) -> Option<u64> {
        self.inner.chat_seq
    }

    #[getter]
    fn get_player_name(&self) -> &str {
        &self.inner.player_name
    }

    #[getter]
    fn get_message(&self) -> &str {
        &self.inner.message
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct GenStateDiffPartW {
    inner: GenStateDiffPart,
}

impl_from_and_to!(GenStateDiffPartW wraps GenStateDiffPart);

#[pymethods]
impl GenStateDiffPartW {
    #[new]
    fn new(part_number: u8, total_parts: u8, gen0: u32, gen1: u32, pattern_part: String) -> Self {
        let inner = GenStateDiffPart {
            part_number,
            total_parts,
            gen0,
            gen1,
            pattern_part,
        };
        GenStateDiffPartW { inner }
    }

    #[getter]
    fn get_part_number(&self) -> u8 {
        self.inner.part_number
    }

    #[getter]
    fn get_total_parts(&self) -> u8 {
        self.inner.total_parts
    }

    #[getter]
    fn get_gen0(&self) -> u32 {
        self.inner.gen0
    }

    #[getter]
    fn get_gen1(&self) -> u32 {
        self.inner.gen1
    }

    #[getter]
    fn get_pattern_part(&self) -> &str {
        &self.inner.pattern_part
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct NetRegionW {
    inner: NetRegion,
}

impl_from_and_to!(NetRegionW wraps NetRegion);

#[pymethods]
impl NetRegionW {
    #[new]
    fn new(left: i32, top: i32, width: u32, height: u32) -> Self {
        let inner = NetRegion {
            left,
            top,
            width,
            height,
        };
        NetRegionW { inner }
    }

    #[getter]
    fn get_left(&self) -> i32 {
        self.inner.left
    }

    #[getter]
    fn get_top(&self) -> i32 {
        self.inner.top
    }

    #[getter]
    fn get_width(&self) -> u32 {
        self.inner.width
    }

    #[getter]
    fn get_height(&self) -> u32 {
        self.inner.height
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct GameOptionsW {
    inner: GameOptions,
}

impl_from_and_to!(GameOptionsW wraps GameOptions);

#[pymethods]
impl GameOptionsW {
    #[new]
    fn new(width: u32, height: u32, history: u16, player_writable_py: Vec<&PyAny>, fog_radius: u32) -> PyResult<Self> {
        vec_from_py! {let player_writable: Vec<NetRegion> <- [NetRegionW] <- player_writable_py};
        let inner = GameOptions {
            width,
            height,
            history,
            player_writable,
            fog_radius,
        };
        Ok(GameOptionsW { inner })
    }

    #[getter]
    fn get_width(&self) -> u32 {
        self.inner.width
    }

    #[getter]
    fn get_height(&self) -> u32 {
        self.inner.height
    }

    #[getter]
    fn get_history(&self) -> u16 {
        self.inner.history
    }

    #[getter]
    fn get_player_writable(&self) -> Vec<NetRegionW> {
        let pw = &self.inner.player_writable;
        pw.iter()
            .cloned()
            .map(|net_region| NetRegionW { inner: net_region })
            .collect()
    }

    #[getter]
    fn get_fog_radius(&self) -> u32 {
        self.inner.fog_radius
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PlayerInfoW {
    inner: PlayerInfo,
}

impl_from_and_to!(PlayerInfoW wraps PlayerInfo);

#[pymethods]
impl PlayerInfoW {
    #[new]
    fn new(name: String, index: Option<u64>) -> Self {
        let inner = PlayerInfo { name, index };
        PlayerInfoW { inner }
    }

    #[getter]
    fn get_name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn get_index(&self) -> Option<u64> {
        self.inner.index
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct GameOutcomeW {
    inner: GameOutcome,
}

impl_from_and_to!(GameOutcomeW wraps GameOutcome);

#[pymethods]
impl GameOutcomeW {
    #[new]
    fn new(winner: Option<String>) -> Self {
        let inner = GameOutcome { winner };
        GameOutcomeW { inner }
    }

    #[getter]
    fn get_winner(&self) -> Option<&String> {
        self.inner.winner.as_ref()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct GameUpdateW {
    inner: GameUpdate,
}

impl_from_and_to!(GameUpdateW wraps GameUpdate);

#[pymethods]
impl GameUpdateW {
    #[new]
    #[args(kwds = "**")]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        use GameUpdate::*;
        let inner = match variant.to_lowercase().as_str() {
            "gamenotification" => {
                let msg: String = get_from_dict(&kwds, "msg")?;
                GameNotification { msg }
            }
            "gamestart" => {
                let optionsw: GameOptionsW = get_from_dict(&kwds, "options")?;
                GameStart {
                    options: optionsw.into(),
                }
            }
            "playerlist" => {
                vec_from_py! {let players: Vec<PlayerInfo> <- [PlayerInfoW] <- get_from_dict(&kwds, "players")?};
                PlayerList { players }
            }
            "playerchange" => {
                let playerw: PlayerInfoW = get_from_dict(&kwds, "player")?;
                let old_name: Option<String> = get_from_dict(&kwds, "old_name")?;
                PlayerChange {
                    player: playerw.into(),
                    old_name,
                }
            }
            "playerjoin" => {
                let playerw: PlayerInfoW = get_from_dict(&kwds, "player")?;
                PlayerJoin { player: playerw.into() }
            }
            "playerleave" => {
                let name: String = get_from_dict(&kwds, "name")?;
                PlayerLeave { name }
            }
            "gamefinish" => {
                let outcomew: GameOutcomeW = get_from_dict(&kwds, "outcome")?;
                GameFinish {
                    outcome: outcomew.into(),
                }
            }
            "roomdeleted" => RoomDeleted,
            "match" => {
                let room: String = get_from_dict(&kwds, "room")?;
                let expire_secs: u32 = get_from_dict(&kwds, "expire_secs")?;
                Match { room, expire_secs }
            }
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(GameUpdateW { inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}
