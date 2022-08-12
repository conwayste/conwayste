///! Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules

pub mod protocol;
pub(crate) mod utils;
use protocol::request::RequestActionW;
use protocol::packet::PacketW;

use pyo3_asyncio;
use pyo3::prelude::*;


/// A Python module implemented in Rust.
#[pymodule]
fn nwv2_python_wrapper(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(rust_delayed_value, m)?)?;
    m.add_class::<PacketW>()?;
    m.add_class::<RequestActionW>()?;
    Ok(())
}
