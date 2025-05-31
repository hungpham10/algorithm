use std::collections::HashMap;
use std::sync::Arc;

use actix::prelude::*;
use log::error;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyTuple;

use crate::actors::cron::CronResolver;
use crate::actors::FUZZY_TRIGGER_THRESHOLD;
use crate::algorithm::{Delegate, Format};

use super::{GetOrderCommand, TcbsActor, TcbsError};

pub fn resolve_tcbs_routes(resolver: &mut CronResolver, stocks: &[String]) -> Arc<Addr<TcbsActor>> {
    let tcbs = TcbsActor::new(stocks, "".to_string());
    let actor = Arc::new(tcbs.start());

    resolve_watching_tcbs_bid_ask_flow(actor.clone(), resolver);
    actor.clone()
}

fn resolve_watching_tcbs_bid_ask_flow(actor: Arc<Addr<TcbsActor>>, resolver: &mut CronResolver) {
    resolver.resolve("tcbs.watch_bid_ask_flow".to_string(), move |task, _, _| {
        let actor = actor.clone();

        async move {
            let datapoints = actor.send(GetOrderCommand { page: 0 }).await.unwrap();

            // Build rule
            let mut rule = if let Some(fuzzy) = task.jsfuzzy() {
                match Delegate::new()
                    .build(&fuzzy, Format::Json)
                    .map_err(|e| TcbsError {
                        message: e.to_string(),
                    }) {
                    Ok(rule) => rule,
                    Err(err) => {
                        error!("Failed to build fuzzy rule: {}", err);
                        return;
                    }
                }
            } else {
                #[cfg(feature = "python")]
                {
                    if let Some(fuzzy) = task.pyfuzzy() {
                        match Delegate::new().build(&*fuzzy, Format::Python).map_err(|e| {
                            TcbsError {
                                message: e.to_string(),
                            }
                        }) {
                            Ok(rule) => rule,
                            Err(err) => {
                                error!("Failed to build fuzzy rule: {}", err);
                                return;
                            }
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
            let labels: Vec<String> = rule.labels().iter().map(|l| l.to_string()).collect();

            for response in datapoints {
                for order in response.data {
                    let mut inputs = HashMap::<String, f64>::new();

                    // Load inputs
                    for label in &labels {
                        match label.as_str() {
                            "p" => {
                                inputs.insert(label.clone(), order.p);
                            }
                            "v" => {
                                inputs.insert(label.clone(), order.v as f64);
                            }
                            "cp" => {
                                inputs.insert(label.clone(), order.cp);
                            }
                            "rcp" => {
                                inputs.insert(label.clone(), order.rcp);
                            }
                            "ba" => {
                                inputs.insert(label.clone(), order.ba);
                            }
                            "sa" => {
                                inputs.insert(label.clone(), order.sa);
                            }
                            "hl" => {
                                inputs.insert(label.clone(), if order.hl { 1.0 } else { 0.0 });
                            }
                            "pcp" => {
                                inputs.insert(label.clone(), order.pcp);
                            }
                            _ => {}
                        };
                    }

                    rule.reload(&inputs);

                    // Evaluate rule
                    let result = rule.evaluate().map_err(|e| TcbsError {
                        message: e.to_string(),
                    });

                    match result {
                        Ok(result) => {
                            if result == FUZZY_TRIGGER_THRESHOLD {
                                #[cfg(feature = "python")]
                                {
                                    Python::with_gil(|py| {
                                        if let Some(callback) = task.pycallback() {
                                            let args = PyTuple::new(py, order.to_pytuple(py));

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
                            eprintln!("Failed to evaluate rule: {}", e);
                        }
                    }
                }
            }
        }
    });
}
