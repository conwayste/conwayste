use std::collections::HashMap;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use crate::utils::get_from_dict;
use netwaystev2::protocol::ResponseCode;

#[pyclass]
#[derive(Clone, Debug)]
pub struct ResponseCodeW {
    inner: ResponseCode,
}

impl Into<ResponseCode> for ResponseCodeW {
    fn into(self) -> ResponseCode {
        self.inner
    }
}

impl From<ResponseCode> for ResponseCodeW {
    fn from(other: ResponseCode) -> Self {
        ResponseCodeW { inner: other }
    }
}

#[pymethods]
impl ResponseCodeW {
    #[new]
    fn new(variant: String, kwds: Option<HashMap<String, &PyAny>>) -> PyResult<Self> {
        let kwds = if let Some(kwds) = kwds { kwds } else { HashMap::new() };
        let inner = match variant.to_lowercase().as_str() {
            "ok" => ResponseCode::OK,
            "loggedin" => {
                let cookie: String = get_from_dict(&kwds, "cookie")?;
                let server_version: String = get_from_dict(&kwds, "server_version")?;
                ResponseCode::LoggedIn { cookie, server_version }
            }
            //XXX more
            _ => {
                return Err(PyValueError::new_err(format!("invalid variant type: {}", variant)));
            }
        };
        Ok(ResponseCodeW { inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}
