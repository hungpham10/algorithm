use std::collections::HashMap;
use std::sync::Arc;

use actix::prelude::*;

use crate::actors::cron::CronResolver;
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
            let fuzzy = task.pyfuzzy();

            // Build rule
            let rule = Delegate::new()
                .build(&*fuzzy, Format::Python)
                .map_err(|e| TcbsError {
                    message: e.to_string(),
                })
                .unwrap();

            // Get labels
            let labels: Vec<String> = rule.labels().iter().map(|l| l.to_string()).collect();

            for response in datapoints {
                for order in response.data {
                    let mut inputs = HashMap::<String, f64>::new();

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
                }
            }
        }
    });
}
