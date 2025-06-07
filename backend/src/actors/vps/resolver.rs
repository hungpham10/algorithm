use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix::prelude::*;
use actix::Actor;
#[cfg(not(feature = "python"))]
use actix_web_prometheus::PrometheusMetrics;

use prometheus::{opts, IntCounterVec};

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyTuple;

use crate::actors::cron::CronResolver;
use crate::actors::{GetVariableCommand, FUZZY_TRIGGER_THRESHOLD};
use crate::algorithm::{Delegate, Format, Variables};

use super::{GetPriceCommand, UpdateVariablesCommand, VpsActor, VpsError};

/// Initializes a `VpsActor` for the given stock symbols and shared variables, starts it as an Actix actor, and registers a periodic VPS board watching task with the provided `CronResolver`.
///
/// # Examples
///
/// ```
/// let mut resolver = CronResolver::default();
/// let stocks = vec!["AAPL".to_string(), "GOOG".to_string()];
/// let variables = Arc::new(Mutex::new(Variables::default()));
/// let vps_addr = resolve_vps_routes(&mut resolver, &stocks, variables.clone());
/// assert!(Arc::strong_count(&vps_addr) > 0);
/// ```
pub fn resolve_vps_routes(
    #[cfg(not(feature = "python"))] prometheus: &PrometheusMetrics,
    resolver: &mut CronResolver,
    stocks: &[String],
    variables: Arc<Mutex<Variables>>,
) -> Arc<Addr<VpsActor>> {
    let status_counter = IntCounterVec::new(
        opts!(
            "vps_watching_board_count",
            "Number of watching board received by the VpsActor"
        )
        .namespace("api"),
        &["status"],
    )
    .unwrap();
    let vps = VpsActor::new(stocks, variables);
    let actor = Arc::new(vps.start());

    #[cfg(not(feature = "python"))]
    prometheus
        .registry
        .register(Box::new(status_counter.clone()))
        .unwrap();

    resolve_watching_vps_board(actor.clone(), resolver, Arc::new(status_counter));
    actor
}

/// Registers a cron task that periodically retrieves price data from a VPS actor, applies a fuzzy logic rule to each data point, and optionally triggers a Python callback when a threshold is met.
///
/// The task fetches current price data, builds a fuzzy rule from the task configuration (JSON or Python), updates the actor's variables, and evaluates the rule for each data point. If the evaluation result matches the trigger threshold and a Python callback is configured (with the `python` feature enabled), the callback is invoked with the data point.
///
/// # Examples
///
/// ```
/// let actor = Arc::new(vps_actor.start());
/// let mut resolver = CronResolver::new();
/// resolve_watching_vps_board(actor, &mut resolver);
/// // The resolver will now periodically process VPS board data.
/// ```
fn resolve_watching_vps_board(
    actor: Arc<Addr<VpsActor>>,
    resolver: &mut CronResolver,
    status_counter: Arc<IntCounterVec>,
) {
    resolver.resolve("vps.watch_boards".to_string(), move |task, _, _| {
        let actor = actor.clone();
        let status_counter = status_counter.clone();

        async move {
            // Get price data
            let datapoints = match actor.send(GetPriceCommand).await.map_err(|e| VpsError {
                message: e.to_string(),
            }) {
                Ok(Ok(datapoints)) => datapoints,
                Ok(Err(_)) => {
                    status_counter.with_label_values(&["fail"]).inc();
                    return;
                }
                Err(_) => {
                    status_counter.with_label_values(&["fail"]).inc();
                    return;
                }
            };

            // Build rule
            let mut rule = if let Some(fuzzy) = task.jsfuzzy() {
                match Delegate::new()
                    .build(&fuzzy, Format::Json)
                    .map_err(|e| VpsError {
                        message: e.to_string(),
                    }) {
                    Ok(rule) => rule,
                    Err(_) => Delegate::new().default(),
                }
            } else {
                #[cfg(feature = "python")]
                {
                    if let Some(fuzzy) = task.pyfuzzy() {
                        match Delegate::new()
                            .build(&*fuzzy, Format::Python)
                            .map_err(|e| VpsError {
                                message: e.to_string(),
                            }) {
                            Ok(rule) => rule,
                            Err(_) => Delegate::new().default(),
                        }
                    } else {
                        Delegate::new().default()
                    }
                }
                #[cfg(not(feature = "python"))]
                {
                    Delegate::new().default()
                }
            };

            // Get labels
            let labels: Vec<String> = rule.inputs().iter().map(|l| l.to_string()).collect();

            // Update variables
            let _ = actor
                .send(UpdateVariablesCommand {
                    prices: datapoints.clone(),
                })
                .await
                .unwrap();

            for point in &datapoints {
                let mut inputs = HashMap::new();
                let symbol = point.sym.clone();

                // Load inputs
                for label in &labels {
                    if let Ok(value) = actor
                        .send(GetVariableCommand {
                            symbol: symbol.clone(),
                            variable: label.clone(),
                        })
                        .await
                    {
                        match value {
                            Ok(val) => {
                                inputs.insert(label.to_string(), val);
                            }
                            Err(e) => {
                                eprintln!("Failed to get value for {}: {}", label, e);
                                status_counter.with_label_values(&["fail"]).inc();
                                return;
                            }
                        }
                    }
                }

                rule.reload(&inputs);

                // Evaluate rule
                let result = rule.evaluate().map_err(|e| VpsError {
                    message: e.to_string(),
                });

                // Handle result and callback
                match result {
                    Ok(result) => {
                        if result == FUZZY_TRIGGER_THRESHOLD {
                            #[cfg(feature = "python")]
                            {
                                Python::with_gil(|py| {
                                    if let Some(callback) = task.pycallback() {
                                        let args = PyTuple::new(py, point.to_pytuple(py));

                                        // Call Python callback
                                        if let Err(e) = callback.call1(py, (args,)) {
                                            e.print_and_set_sys_last_vars(py);
                                        }
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Resolver error: {:?}", e);
                        status_counter.with_label_values(&["fail"]).inc();
                        return;
                    }
                }
            }

            status_counter.with_label_values(&["success"]).inc();
        }
    });
}
