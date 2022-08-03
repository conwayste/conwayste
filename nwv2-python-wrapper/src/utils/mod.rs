use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::exceptions::*;

pub(crate) fn get_from_dict<'py, T: FromPyObject<'py>>(d: &HashMap<String, &'py PyAny>, k: &str) -> PyResult<T> {
    if let Some(v) = d.get(k){
        let val = (*v).extract::<T>()?;
        Ok(val)
    } else {
        Err(PyKeyError::new_err(format!("not in dict: {}", k)))
    }
}

