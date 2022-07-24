/// Reference: https://pyo3.rs/v0.16.4/ecosystem/async-await.html#pyo3-native-rust-modules
use pyo3::prelude::*;

use netwaystev2::protocol::Packet;
use netwaystev2::protocol::RequestAction;

#[pyfunction]
fn get_a_packet() -> PyResult<String> {
    let p = Packet::Request{
        sequence:     1,
        response_ack: None, // Next expected  sequence number the Server responds with to the Client.
        // Stated differently, the client has seen Server responses from 0 to response_ack-1.
        cookie:       None,
        action:       RequestAction::Connect{
            name: "Paul".to_owned(),
            client_version: "0.0.0".to_owned(),
        },
    };
    Ok(format!("{:#?}", p))
}

/// A Python module implemented in Rust.
#[pymodule]
fn nwv2_python_wrapper(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_a_packet, m)?)?;
    Ok(())
}
