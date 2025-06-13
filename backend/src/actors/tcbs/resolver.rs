use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix::prelude::*;
#[cfg(not(feature = "python"))]
use actix_web_prometheus::PrometheusMetrics;

use log::error;
use prometheus::{opts, IntCounterVec};

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::{PyList, PyTuple};

use crate::actors::cron::CronResolver;
use crate::actors::{GetVariableCommand, FUZZY_TRIGGER_THRESHOLD};
use crate::algorithm::fuzzy::{Delegate, Format, Variables};

use super::{GetOrderCommand, TcbsActor, TcbsError, UpdateVariablesCommand};

pub async fn resolve_tcbs_routes(
    #[cfg(not(feature = "python"))] prometheus: &PrometheusMetrics,
    resolver: &mut CronResolver,
    stocks: &[String],
    variables: Arc<Mutex<Variables>>,
    depth: usize,
) -> Arc<Addr<TcbsActor>> {
    let status_counter = IntCounterVec::new(
        opts!(
            "tcbs_bid_ask_status_count",
            "Number of bid-ask flow updates received by the TcbsActor"
        )
        .namespace("api"),
        &["status"],
    )
    .unwrap();

    let order_counter = IntCounterVec::new(
        opts!(
            "tcbs_bid_ask_order_flow_count",
            "Number of bid-ask orders received by the TcbsActor"
        )
        .namespace("api"),
        &["symbol"],
    )
    .unwrap();

    let tcbs = TcbsActor::new(stocks, "".to_string(), variables).await;
    let actor = Arc::new(tcbs.start());

    #[cfg(not(feature = "python"))]
    {
        prometheus
            .registry
            .register(Box::new(status_counter.clone()))
            .unwrap();

        prometheus
            .registry
            .register(Box::new(order_counter.clone()))
            .unwrap();
    }

    resolve_watching_tcbs_bid_ask_flow(
        actor.clone(),
        resolver,
        depth,
        Arc::new(status_counter),
        Arc::new(order_counter),
    );
    actor.clone()
}

fn resolve_watching_tcbs_bid_ask_flow(
    actor: Arc<Addr<TcbsActor>>,
    resolver: &mut CronResolver,
    depth: usize,
    status_counter: Arc<IntCounterVec>,
    order_counter: Arc<IntCounterVec>,
) {
    resolver.resolve("tcbs.watch_bid_ask_flow".to_string(), move |task, _, _| {
        let actor = actor.clone();
        let status_counter = status_counter.clone();
        let order_counter = order_counter.clone();

        async move {
            for i in (0..depth).rev() {
                let datapoints =
                    match actor
                        .send(GetOrderCommand { page: i })
                        .await
                        .map_err(|e| TcbsError {
                            message: e.to_string(),
                        }) {
                        Ok(datapoints) => datapoints,
                        Err(_) => {
                            status_counter.with_label_values(&["fail"]).inc();
                            return;
                        }
                    };

                // Build rule
                let mut rule = if let Some(fuzzy) = task.jsfuzzy() {
                    match Delegate::new()
                        .build(&fuzzy, Format::Json)
                        .map_err(|e| TcbsError {
                            message: e.to_string(),
                        }) {
                        Ok(rule) => rule,
                        Err(_) => Delegate::new().default(),
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

                for response in datapoints {
                    let mut inputs = HashMap::new();

                    let size = match actor
                        .send(UpdateVariablesCommand {
                            symbol: response.ticker.clone(),
                            orders: response.data.clone(),
                            counter: order_counter.clone(),
                        })
                        .await
                    {
                        Ok(Ok(size)) => size,
                        Ok(Err(e)) => {
                            error!("Failed to update variables: {}", e);
                            status_counter.with_label_values(&["fail"]).inc();
                            return;
                        }
                        Err(_) => {
                            status_counter.with_label_values(&["fail"]).inc();
                            return;
                        }
                    };

                    if size == 0 {
                        continue;
                    }

                    // Load inputs
                    for label in &labels {
                        if let Ok(value) = actor
                            .send(GetVariableCommand {
                                symbol: response.ticker.clone(),
                                variable: label.to_string(),
                            })
                            .await
                        {
                            match value {
                                Ok(val) => {
                                    inputs.insert(label.to_string(), val);
                                }
                                Err(e) => {
                                    error!("Failed to get variable: {}", e);
                                    status_counter.with_label_values(&["fail"]).inc();
                                    return;
                                }
                            }
                        }
                    }

                    rule.reload(&inputs);

                    // Evaluate rule
                    let result = rule.evaluate().map_err(|e| TcbsError {
                        message: e.to_string(),
                    });

                    // Handle result and callback
                    match result {
                        Ok(result) => {
                            if result == FUZZY_TRIGGER_THRESHOLD {
                                #[cfg(feature = "python")]
                                {
                                    let orders = &response.data;

                                    Python::with_gil(|py| {
                                        if let Some(callback) = task.pycallback() {
                                            let args: Py<PyList> = PyList::new(
                                                py,
                                                orders
                                                    .iter()
                                                    .map(|order| {
                                                        PyTuple::new(py, order.to_pytuple(py))
                                                    })
                                                    .collect::<Vec<_>>(),
                                            )
                                            .into();

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
                            status_counter.with_label_values(&["fail"]).inc();
                            return;
                        }
                    }
                }
            }

            status_counter.with_label_values(&["success"]).inc();
        }
    });
}
