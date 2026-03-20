use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use futures_util::StreamExt;
use vector_config_macro::source;

use serde_json::json;

use k8s_openapi::api::core::{v1::Pod, v1::Service};
use kube::api::LogParams;
use kube::runtime::{WatchStreamExt, watcher};
use kube::{Api, Client, Config};

use vector_runtime::{Component, Event, Identify, Message};

#[source(derive(PartialEq))]
pub struct LogSource {
    pub id: String,
    pub server: String,
    pub token: String,
    pub namespace: String,
    pub service: String,
}

impl_log_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<Message>,
        txs: &'life2 [mpsc::Sender<Message>],
        err: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut config = Config::new(
            self.server
                .parse()
                .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("URL wrong: {}", e)))?,
        );
        config.auth_info.token = Some(self.token.clone().into());
        config.accept_invalid_certs = false;
        config.default_namespace = self.namespace.clone();

        let client = Client::try_from(config)
            .map_err(|error| Error::other(format!("K8s Auth Error: {}", error)))?;

        let svc_api: Api<Service> = Api::namespaced(client.clone(), &self.namespace);
        let service_obj = svc_api.get(&self.service).await.map_err(|error| {
            Error::new(
                ErrorKind::NotFound,
                format!("Svc {} not found: {}", self.service, error),
            )
        })?;

        let selector_map = service_obj
            .spec
            .and_then(|spec| spec.selector)
            .ok_or_else(|| {
                Error::new(ErrorKind::InvalidData, "Service doesn't have any selectors")
            })?;

        let label_selector = selector_map
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");

        let pods_api: Api<Pod> = Api::namespaced(client, &self.namespace);

        let txs_owned: Vec<mpsc::Sender<Message>> = txs.to_vec();
        let txs_arc = Arc::new(txs_owned);

        let wc = watcher::Config {
            label_selector: Some(label_selector),
            field_selector: Some("status.phase=Running".to_string()),
            ..Default::default()
        };

        let tracked_pods = Arc::new(Mutex::new(HashSet::<String>::new()));

        loop {
            let mut pod_stream = watcher(pods_api.clone(), wc.clone())
                .applied_objects()
                .boxed();

            while let Some(result) = pod_stream.next().await {
                match result {
                    Ok(pod) => {
                        let p_name = pod.metadata.name.clone().unwrap_or_default();

                        {
                            let mut tracked = tracked_pods.lock().unwrap();
                            if tracked.contains(&p_name) {
                                continue;
                            }
                            tracked.insert(p_name.clone());
                        }

                        let txs_for_task = Arc::clone(&txs_arc);
                        let pod_api_task = pods_api.clone();
                        let tracked_clone = Arc::clone(&tracked_pods);
                        let p_name_task = p_name.clone();

                        tokio::spawn(async move {
                            let lp = LogParams {
                                follow: true,
                                tail_lines: Some(0),
                                ..LogParams::default()
                            };

                            loop {
                                match pod_api_task.log_stream(&p_name_task, &lp).await {
                                    Ok(stream) => {
                                        let mut reader = stream.compat().lines();

                                        while let Ok(Some(line)) = reader.next_line().await {
                                            let trimmed = line.trim();

                                            if !trimmed.is_empty() {
                                                let payload = serde_json::from_str(trimmed)
                                                    .unwrap_or_else(|_| json!(trimmed));

                                                for tx in txs_for_task.iter() {
                                                    let msg = Message {
                                                        payload: payload.clone(),
                                                    };
                                                    if let Err(_e) = tx.send(msg).await {
                                                        continue;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Kiểm tra xem Pod còn tồn tại không, nếu không thì giải phóng khỏi HashSet
                                        if pod_api_task.get(&p_name_task).await.is_err() {
                                            let mut tracked = tracked_clone.lock().unwrap();
                                            tracked.remove(&p_name_task);
                                            return; // Kết thúc task cho Pod này
                                        }
                                        // Thử lại sau 1s nếu chỉ là lỗi kết nối tạm thời
                                        sleep(Duration::from_secs(1)).await;
                                    }
                                }
                            }
                        });
                    }
                    Err(error) => {
                        let report_err = Error::other(format!("Failed to watch pods: {}", error));

                        if let Err(e) = err.send(Event::Minor((id, report_err))).await {
                            return Err(Error::new(
                                ErrorKind::BrokenPipe,
                                format!("Critical: Error channel closed: {}", e),
                            ));
                        }
                    }
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }
);
