use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix::prelude::*;
use actix::Actor;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::actors::cron::CronResolver;
use crate::actors::FUZZY_TRIGGER_THRESHOLD;
use crate::algorithm::{Delegate, Format, Variables};

use super::VpsActor;
use super::{GetPriceCommand, GetVariableCommand, UpdateVariablesCommand, VpsError};

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
            let mut rule = if let Some(fuzzy) = task.pyfuzzy() {
                Delegate::new()
                    .build(&*fuzzy, Format::Python)
                    .map_err(|e| VpsError {
                        message: e.to_string(),
                    })
                    .unwrap()
            } else if let Some(fuzzy) = task.jsfuzzy() {
                Delegate::new()
                    .build(&fuzzy, Format::Json)
                    .map_err(|e| VpsError {
                        message: e.to_string(),
                    })
                    .unwrap()
            } else {
                Delegate::new().default()
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
                            index: 0,
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
                    Err(e) => eprintln!("Resolver error: {:?}", e),
                }
            }
        }
    });
}
