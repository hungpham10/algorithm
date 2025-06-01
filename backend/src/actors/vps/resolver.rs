use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::error;

use actix::prelude::*;
use actix::Actor;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyTuple;

use crate::actors::cron::CronResolver;
use crate::actors::{GetVariableCommand, FUZZY_TRIGGER_THRESHOLD};
use crate::algorithm::{Delegate, Format, Variables};

use super::{GetPriceCommand, UpdateVariablesCommand, VpsActor, VpsError};

pub fn resolve_vps_routes(
    resolver: &mut CronResolver,
    stocks: &[String],
    variables: Arc<Mutex<Variables>>,
) -> Arc<Addr<VpsActor>> {
    let vps = VpsActor::new(stocks, variables);
    let actor = Arc::new(vps.start());

    resolve_watching_vps_board(actor.clone(), resolver);
    actor
}

fn resolve_watching_vps_board(actor: Arc<Addr<VpsActor>>, resolver: &mut CronResolver) {
    resolver.resolve("vps.watch_boards".to_string(), move |task, _, _| {
        let actor = actor.clone();

        async move {
            // Get price data
            let datapoints = actor.send(GetPriceCommand).await.unwrap();

            // Build rule
            let mut rule = if let Some(fuzzy) = task.jsfuzzy() {
                match Delegate::new()
                    .build(&fuzzy, Format::Json)
                    .map_err(|e| VpsError {
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
                        match Delegate::new()
                            .build(&*fuzzy, Format::Python)
                            .map_err(|e| VpsError {
                                message: e.to_string(),
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
                            Err(e) => eprintln!("Failed to get value for {}: {}", label, e),
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
                    Err(e) => eprintln!("Resolver error: {:?}", e),
                }
            }
        }
    });
}
