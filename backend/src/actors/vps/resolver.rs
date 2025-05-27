use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use actix::Actor;
use actix::prelude::*;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::actors::cron::CronResolver;
use crate::algorithm::{Delegate, Variables, Format};

use super::{GetPriceCommand, GetVariableCommand, UpdateVariablesCommand, VpsError};
use super::VpsActor;


pub fn resolve_vps_routes(
    resolver: &mut CronResolver,
    stocks: &Vec<String>,
    variables: Arc<Mutex<Variables>>,
) -> Arc<Addr<VpsActor>> {
    let vps = VpsActor::new(stocks, variables);
    let actor= Arc::new(vps.start());

    resolve_watching_vps_board(actor.clone(), resolver);
    return actor;
}

fn resolve_watching_vps_board(actor: Arc<Addr<VpsActor>>, resolver: &mut CronResolver) {
    resolver.resolve("vps.watch_boards".to_string(), move |task, _, _| {
        let actor = actor.clone();
        
        async move {
            // Get price data
            let datapoints = actor.send(GetPriceCommand)
                .await
                .unwrap();
            let fuzzy = task.pyfuzzy();

            // Build rule
            let mut rule = Delegate::new()
                .build(&*fuzzy, Format::Python)
                .map_err(|e| VpsError{ message: e.to_string() })
                .unwrap();

            // Get labels
            let labels: Vec<String> = rule.labels()
                .iter()
                .map(|l| l.to_string())
                .collect();

            // Update variables
            let _ = actor.send(UpdateVariablesCommand { prices: datapoints.clone() })
                .await
                .unwrap();

            for point in &datapoints {
                let mut inputs = HashMap::new();
                let symbol = point.sym.clone();

                // Load inputs
                for label in &labels {
                    if let Ok(value) = actor.send(GetVariableCommand {
                        symbol:   symbol.clone(),
                        variable: label.clone(),
                        index:    0, 
                    }).await {
                        match value {
                            Ok(val) => { inputs.insert(label.to_string(), val); },
                            Err(e) => eprintln!("Failed to get value for {}: {}", label, e),
                        }
                    }
                }

                rule.reload(&inputs);
            
                // Evaluate rule
                let result = rule.evaluate()
                    .map_err(|e| VpsError{ message: e.to_string() });

                // Handle result and callback
                match result {
                    Ok(result) => {
                        if result == 1.0 {
                            Python::with_gil(|py| {
                                let callback = task.pycallback();
                                let args = PyTuple::new(
                                    py, 
                                    point.to_pytuple(py),
                                );

                                // Call Python callback
                                if let Err(e) = callback.call1(py, (args,)) {
                                    e.print_and_set_sys_last_vars(py);
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
