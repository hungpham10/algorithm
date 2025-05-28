use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::algorithm::Variables;

#[pyclass]
pub struct Datastore {
    variables: Arc<Mutex<Variables>>,
}

#[pymethods]
impl Datastore {
    #[new]
    fn new(memory_size: usize) -> Self {
        Datastore {
            variables: Arc::new(Mutex::new(Variables::new(memory_size))),
        }
    }
}

impl Datastore {
    pub fn variables(&self) -> Arc<Mutex<Variables>> {
        self.variables.clone()
    }
}
