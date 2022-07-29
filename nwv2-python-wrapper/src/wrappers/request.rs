use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::*;

use netwaystev2::protocol::RequestAction;

#[pyclass]
pub struct RequestActionW {
    inner: RequestAction,
}

#[pymethods]
impl RequestActionW {
    #[new]
    #[args(kwds="**")]
    fn new(variant: String, kwds: Option<HashMap<String,&PyAny>>) -> PyResult<Self> {
        //XXX replace the following with a big match like the enum definition, taking from `kwds`
        //as needed
        if let Some(kwds) = kwds {
            kwds.get("haha");
        }
        Ok(RequestActionW{inner:RequestAction::None})
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// TODO: ClientOptionValue
