use std::io::{Error, ErrorKind};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::sleep;

use k8s_openapi::api::core::v1::Node;
use kube::{Api, Client, Config};
use serde::Deserialize;
use serde_json::json;
use vector_config_macro::source;

use vector_runtime::{Component, Event, Identify, Message};

#[derive(Deserialize, Debug, Clone)]
struct NodeMetricsList {
    pub items: Vec<NodeMetrics>,
}

#[derive(Deserialize, Debug, Clone)]
struct NodeMetrics {
    pub metadata: kube::core::ObjectMeta,
    pub usage: NodeUsage,
}

#[derive(Deserialize, Debug, Clone)]
struct NodeUsage {
    pub cpu: String,
    pub memory: String,
}

#[source(derive(PartialEq))]
pub struct NodeSource {
    pub id: String,
    pub server: String,
    pub token: String,
}

fn parse_cpu_to_cores(val: &str) -> f64 {
    let numeric_part: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
    let val_f = numeric_part.parse::<f64>().unwrap_or(0.0);
    if val.ends_with('n') {
        val_f / 1_000_000_000.0
    } else if val.ends_with('m') {
        val_f / 1000.0
    } else {
        val_f
    }
}

fn parse_mem_to_gb(val: &str) -> f64 {
    let numeric_part: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
    let val_f = numeric_part.parse::<f64>().unwrap_or(0.0);
    if val.ends_with("Ki") {
        val_f / (1024.0 * 1024.0)
    } else if val.ends_with("Mi") {
        val_f / 1024.0
    } else if val.ends_with("Gi") {
        val_f
    } else {
        val_f / (1024.0 * 1024.0 * 1024.0)
    }
}

impl_node_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<Message>,
        txs: &Vec<mpsc::Sender<Message>>,
        err: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        let mut config = Config::new(self.server.parse().expect("Invalid Server URL"));
        config.accept_invalid_certs = true;

        let mut auth_info = kube::config::AuthInfo::default();
        auth_info.token = Some(self.token.clone().into());
        config.auth_info = auth_info;

        let client = Client::try_from(config).map_err(|error| {
            Error::new(
                ErrorKind::Other,
                format!("Kube Init Error ({}): {}", id, error),
            )
        })?;

        let nodes_api: Api<Node> = Api::all(client.clone());

        loop {
            let path = "/apis/metrics.k8s.io/v1beta1/nodes";

            let req = http::Request::builder()
                .method("GET")
                .uri(path)
                .header("Accept", "application/json")
                .body(vec![])
                .expect("Failed to build request");

            match client.request::<NodeMetricsList>(req).await {
                Ok(metrics_list) => {
                    for m in metrics_list.items {
                        let name = m.metadata.name.clone().unwrap_or_default();

                        if let Ok(node_info) = nodes_api.get(&name).await {
                            if let Some(allocatable) = node_info.status.and_then(|s| s.allocatable)
                            {
                                let cpu_total = parse_cpu_to_cores(
                                    allocatable.get("cpu").map(|x| x.0.as_str()).unwrap_or("1"),
                                );
                                let mem_total = parse_mem_to_gb(
                                    allocatable
                                        .get("memory")
                                        .map(|x| x.0.as_str())
                                        .unwrap_or("1Gi"),
                                );

                                let cpu_used = parse_cpu_to_cores(&m.usage.cpu);
                                let mem_used = parse_mem_to_gb(&m.usage.memory);

                                let cpu_p = (cpu_used / cpu_total) * 100.0;
                                let mem_p = (mem_used / mem_total) * 100.0;

                                let payload = json!({
                                    "node_name": name,
                                    "cpu": cpu_p,
                                    "mem": mem_p,
                                });

                                for tx in txs {
                                    let _ = tx
                                        .send(Message {
                                            payload: payload.clone(),
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    err.send(Event::Minor((
                        id,
                        Error::new(ErrorKind::Other, format!("Failed to find pods: {}", error,)),
                    )))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to send issue: {}", error,),
                        )
                    })?;
                }
            }

            sleep(Duration::from_secs(15)).await;
        }
    }
);
