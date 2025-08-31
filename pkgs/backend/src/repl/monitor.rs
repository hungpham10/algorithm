use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{debug, error, warn};

use crate::actors::cron::{connect_to_cron, CronResolver, ScheduleCommand, TickCommand};
use crate::actors::tcbs::resolve_tcbs_routes;
use crate::actors::vps::resolve_vps_routes;

use super::datastore::Datastore;

#[pyclass]
pub struct Monitor {
    schedules: Vec<ScheduleCommand>,
    enabled: Arc<Mutex<bool>>,
    datastore: Py<Datastore>,
    running: Arc<AtomicI64>,
    done: Arc<AtomicI64>,
}

#[pymethods]
impl Monitor {
    #[new]
    fn new(datastore: Py<Datastore>) -> Self {
        Self {
            schedules: Vec::new(),
            enabled: Arc::new(Mutex::new(false)),
            running: Arc::new(AtomicI64::new(0)),
            done: Arc::new(AtomicI64::new(0)),
            datastore,
        }
    }

    fn schedule(
        &mut self,
        cron: String,
        route: String,
        timeout: i32,
        fuzzy: Py<PyDict>,
        callback: Py<PyAny>,
    ) -> PyResult<()> {
        self.schedules.push(ScheduleCommand {
            cron,
            route,
            timeout,
            jsfuzzy: None,
            pyfuzzy: Some(Arc::new(fuzzy)),
            pycallback: Some(Arc::new(callback)),
        });
        Ok(())
    }

    fn start(&self, py: Python, stocks: Vec<String>) -> PyResult<()> {
        let mut enabled = self.enabled.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire lock")
        })?;
        let schedules = self.schedules.clone();
        let datastore = self.datastore.borrow(py);
        let vps_vars = datastore.vps();
        let tcbs_vars = datastore.tcbs();

        *enabled = true;

        let enabled = self.enabled.clone();
        let running = self.running.clone();
        let done = self.done.clone();

        Python::allow_threads(py, move || {
            thread::spawn(move || {
                actix_rt::Runtime::new().unwrap().block_on(async move {
                    let mut resolver = CronResolver::new();
                    let _ = resolve_vps_routes(&mut resolver, &stocks, vps_vars.clone());
                    let _ = resolve_tcbs_routes(&mut resolver, &stocks, tcbs_vars.clone(), 1).await;
                    let cron = Arc::new(connect_to_cron(Rc::new(resolver)));

                    for command in schedules {
                        let _ = cron.send(command).await.unwrap();
                    }

                    while *enabled.lock().unwrap() {
                        match cron
                            .send(TickCommand {
                                running: running.clone(),
                                done: done.clone(),
                            })
                            .await
                        {
                            Ok(Ok(cnt)) => debug!("Finish {} tasks", cnt),
                            Ok(Err(error)) => warn!("Failed with error: {}", error),
                            Err(error) => {
                                error!("Panic with error: {}", error);
                                break;
                            }
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
        });
        Ok(())
    }

    fn stop(&self) -> PyResult<()> {
        let mut enabled = self.enabled.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire lock")
        })?;
        *enabled = false;
        Ok(())
    }

    fn inflight(&self) -> i64 {
        let running = self.running.load(Ordering::SeqCst);
        let done = self.done.load(Ordering::SeqCst);
        running.saturating_sub(done)
    }
}
