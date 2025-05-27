use pyo3::prelude::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use crate::actors::cron::{CronResolver, ScheduleCommand, TickCommand, connect_to_cron};
use crate::actors::vps::resolve_vps_routes;
use crate::actors::tcbs::resolve_tcbs_routes;

use super::datastore::Datastore;

#[pyclass]
pub struct Monitor {
    schedules: Vec<ScheduleCommand>,
    enabled:   Arc<Mutex<bool>>,
    datastore: Py<Datastore>,
}

#[pymethods]
impl Monitor {
    #[new]
    fn new(datastore: Py<Datastore>) -> Self {
        Self {
            schedules: Vec::new(),
            enabled: Arc::new(Mutex::new(false)),
            datastore,
        }
    }

    fn start(&self, py: Python, stocks: Vec<String>) -> PyResult<()> {
        let mut enabled = self.enabled.lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire lock"
            ))?; 
        let schedules = self.schedules.clone();
        let datastore = self.datastore.borrow(py);
        let variables = datastore.variables();

        *enabled = true;
        let enabled = self.enabled.clone();

        Python::allow_threads(py, move || {
            actix_rt::spawn(async move {
                let mut resolver = CronResolver::new();
                let _ = resolve_vps_routes(&mut resolver, &stocks, variables.clone());
                let _ = resolve_tcbs_routes(&mut resolver, &stocks, variables.clone());
                let cron = Arc::new(connect_to_cron(Arc::new(resolver)));
                
                for command in schedules {
                    let _ = cron.send(command)
                        .await
                        .unwrap();
                }

                while *enabled.lock().unwrap() {
                    match cron.send(TickCommand).await {
                        Ok(Ok(_)) => {},
                        Ok(Err(_)) => {},
                        Err(_) => break,
                    }

                    thread::sleep(Duration::from_secs(1));
                }

                if let Ok(mut enabled) = enabled.lock() {
                    *enabled = false;
                } else {
                    panic!("fail to secure mutex");
                }
            });
        });
        Ok(())
    }

    fn stop(&self) -> PyResult<()> {
        let mut enabled = self.enabled.lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire lock"
            ))?;
        *enabled = false;
        Ok(())
    }
}
