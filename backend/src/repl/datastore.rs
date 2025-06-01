use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::algorithm::Variables;

#[pyclass]
pub struct Datastore {
    vps_vars: Arc<Mutex<Variables>>,
    tcbs_vars: Arc<Mutex<Variables>>,
}

#[pymethods]
impl Datastore {
    #[new]
    fn new(vps_memory_size: usize, tcbs_memory_size: usize) -> Self {
        Datastore {
            vps_vars: Arc::new(Mutex::new(Variables::new(vps_memory_size))),
            tcbs_vars: Arc::new(Mutex::new(Variables::new(tcbs_memory_size))),
        }
    }
}

impl Datastore {
    pub fn tcbs(&self) -> Arc<Mutex<Variables>> {
        self.tcbs_vars.clone()
    }

    pub fn vps(&self) -> Arc<Mutex<Variables>> {
        self.vps_vars.clone()
    }
}
