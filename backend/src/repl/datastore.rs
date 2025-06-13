use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::algorithm::fuzzy::Variables;

#[pyclass]
pub struct Datastore {
    vps_vars: Arc<Mutex<Variables>>,
    tcbs_vars: Arc<Mutex<Variables>>,
}

#[pymethods]
impl Datastore {
    #[new]
    /// Creates a new `Datastore` with separate variable stores for VPS and TCBS, each initialized with the specified memory size and an initial value of zero.
    ///
    /// # Parameters
    /// - `vps_memory_size`: The memory size to allocate for the VPS variable store.
    /// - `tcbs_memory_size`: The memory size to allocate for the TCBS variable store.
    ///
    /// # Examples
    ///
    /// ```
    /// let ds = Datastore::new(1024, 2048);
    /// let vps = ds.vps();
    /// let tcbs = ds.tcbs();
    /// ```
    fn new(vps_memory_size: usize, tcbs_memory_size: usize) -> Self {
        Datastore {
            vps_vars: Arc::new(Mutex::new(Variables::new(vps_memory_size, 0))),
            tcbs_vars: Arc::new(Mutex::new(Variables::new(tcbs_memory_size, 0))),
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
