#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyDict;

use super::rule::Expression;

pub trait Input {
    fn as_json(&self) -> Option<&String> {
        None
    }
    fn as_expression(&self) -> Option<&Expression> {
        None
    }

    #[cfg(feature = "python")]
    fn as_python(&self) -> Option<&Py<PyDict>> {
        None
    }
}

impl Input for String {
    fn as_json(&self) -> Option<&String> {
        Some(self)
    }
}

impl Input for Expression {
    fn as_expression(&self) -> Option<&Expression> {
        Some(self)
    }
}

#[cfg(feature = "python")]
impl Input for Py<PyDict> {
    fn as_python(&self) -> Option<&Py<PyDict>> {
        Some(self)
    }
}
