use std::collections::HashMap;
use std::net::SocketAddr;

use pyo3::exceptions::*;
use pyo3::prelude::*;

use netwaystev2::common::Endpoint;

#[pyclass]
#[derive(Clone)]
pub struct EndpointW {
    inner: Endpoint,
}

impl Into<Endpoint> for EndpointW {
    fn into(self) -> Endpoint {
        self.inner
    }
}

impl From<Endpoint> for EndpointW {
    fn from(other: Endpoint) -> Self {
        EndpointW { inner: other }
    }
}

#[pymethods]
impl EndpointW {
    #[new]
    fn new(host_and_port: String) -> PyResult<Self> {
        let sa: SocketAddr = host_and_port.parse().map_err(|e| {
            return PyValueError::new_err(format!("failed to parse SocketAddr string for Endpoint: {}", e));
        })?;
        Ok(EndpointW { inner: Endpoint(sa) })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

pub(crate) fn get_from_dict<'py, T: FromPyObject<'py>>(d: &HashMap<String, &'py PyAny>, k: &str) -> PyResult<T> {
    if let Some(v) = d.get(k) {
        let val = (*v).extract::<T>()?;
        Ok(val)
    } else {
        Err(PyKeyError::new_err(format!("not in dict: {}", k)))
    }
}

/// Example:
///
/// ```no_run
/// vec_from_py!{let endpoints: Vec<Endpoint> <- [EndpointW] <- get_from_dict(&kwds, "endpoints")?};
/// ```
///
/// Expands to:
///
/// ```no_run
///  let endpoints_py: Vec<&PyAny> = get_from_dict(&kwds, "endpoints")?;
///  let endpoints: Vec<Endpoint> = endpoints_py
///      .into_iter()
///      .map(|ep| {
///          let endpointw: EndpointW = ep.extract()?;
///          Ok(endpointw.into())
///      })
///      .collect::<PyResult<Vec<_>>>()?;
/// ```
///
/// Must use in a function that returns PyResult<T> where T is any type,
macro_rules! vec_from_py {
    (let $result_var:ident: Vec<$final_type:ty> <- [$wrapper_type:ty] <- $expression:expr) => {
        let vec_py: Vec<&PyAny> = $expression;
        let $result_var: Vec<$final_type> = vec_py
            .into_iter()
            .map(|obj_py| {
                let wrapped: $wrapper_type = obj_py.extract()?;
                Ok(wrapped.into())
            })
            .collect::<PyResult<Vec<_>>>()?;
    };
}

macro_rules! impl_from_and_to {
    ($wrapper:ident wraps $typ:ident) => {
        impl Into<$typ> for $wrapper {
            fn into(self) -> $typ {
                self.inner
            }
        }

        impl From<$typ> for $wrapper {
            fn from(other: $typ) -> Self {
                $wrapper { inner: other }
            }
        }
    };
}
