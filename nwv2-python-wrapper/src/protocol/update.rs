use pyo3::prelude::*;

use netwaystev2::protocol::*;

#[pyclass]
#[derive(Clone, Debug)]
pub struct BroadcastChatMessageW {
    inner: BroadcastChatMessage,
}

impl Into<BroadcastChatMessage> for BroadcastChatMessageW {
    fn into(self) -> BroadcastChatMessage {
        self.inner
    }
}

impl From<BroadcastChatMessage> for BroadcastChatMessageW {
    fn from(other: BroadcastChatMessage) -> Self {
        BroadcastChatMessageW { inner: other }
    }
}

#[pymethods]
impl BroadcastChatMessageW {
    #[new]
    fn new(chat_seq: Option<u64>, player_name: String, message: String) -> PyResult<Self> {
        let inner = BroadcastChatMessage {
            chat_seq,
            player_name,
            message,
        };
        Ok(BroadcastChatMessageW { inner })
    }

    #[getter]
    fn get_chat_seq(&self) -> Option<u64> {
        self.inner.chat_seq
    }

    #[getter]
    fn get_player_name(&self) -> String {
        self.inner.player_name
    }

    #[getter]
    fn get_message(&self) -> String {
        self.inner.message
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

impl Into<GenStateDiffPart> for GenStateDiffPartW {
    fn into(self) -> GenStateDiffPart {
        self.inner
    }
}

impl From<GenStateDiffPart> for GenStateDiffPartW {
    fn from(other: GenStateDiffPart) -> Self {
        GenStateDiffPartW { inner: other }
    }
}

#[pymethods]
impl GenStateDiffPartW {
    #[new]
    fn new(part_number: u8, total_parts: u8, gen0: u32, gen1: u32, pattern_part: String) -> PyResult<Self> {
        let inner = GenStateDiffPart {
            part_number,
            total_parts,
            gen0,
            gen1,
            pattern_part,
        };
        Ok(GenStateDiffPartW { inner })
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
    fn get_pattern_part(&self) -> String {
        self.inner.pattern_part
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

impl Into<NetRegion> for NetRegionW {
    fn into(self) -> NetRegion {
        self.inner
    }
}

impl From<NetRegion> for NetRegionW {
    fn from(other: NetRegion) -> Self {
        NetRegionW { inner: other }
    }
}

#[pymethods]
impl NetRegionW {
    #[new]
    fn new(left: i32, top: i32, width: u32, height: u32) -> PyResult<Self> {
        let inner = NetRegion {
            left,
            top,
            width,
            height,
        };
        Ok(NetRegionW { inner })
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

impl Into<GameOptions> for GameOptionsW {
    fn into(self) -> GameOptions {
        self.inner
    }
}

impl From<GameOptions> for GameOptionsW {
    fn from(other: GameOptions) -> Self {
        GameOptionsW { inner: other }
    }
}

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
        let pw = self.inner.player_writable;
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
