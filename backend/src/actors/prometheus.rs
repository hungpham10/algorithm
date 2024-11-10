use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client as HttpClient};
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use crate::actors::cron::CronResolver;
use crate::actors::redis::RedisActor;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryValueDataPoint {
    pub timestamp: f32,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryDataPoint {
    pub metric: HashMap<String, String>,
    pub value: QueryValueDataPoint,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct QueryDataResponse {
    pub resultType: String,
    pub result: Vec<QueryDataPoint>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryResponse {
    pub status: String,
    pub data: QueryDataResponse,
}

#[derive(Debug, Clone)]
pub struct FireantError {
    message: String,
}

impl fmt::Display for FireantError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct PrometheusActor {
    timeout: u64,
    host: String,
}

impl PrometheusActor {
    fn new(host: String) -> Self {
        Self {
            host: host.clone(),
            timeout: 60,
        }
    }
}

impl Actor for PrometheusActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct PrometheusError {
    message: String,
}

impl fmt::Display for PrometheusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<i64, PrometheusError>")]
pub struct GetCpuUsageCommand {
    container: String,
}

impl Handler<GetCpuUsageCommand> for PrometheusActor {
    type Result = ResponseFuture<Result<i64, PrometheusError>>;

    fn handle(&mut self, msg: GetCpuUsageCommand, _: &mut Self::Context) -> Self::Result {
        let host = self.host.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            fetch_container_cpu_usage_from_prometheus(host.clone(), msg.container, timeout).await;

            Ok(0)
        })
    }
}

async fn fetch_container_cpu_usage_from_prometheus(
    host: String,
    container: String,
    timeout: u64,
) -> Result<Vec<QueryDataPoint>, PrometheusError> {
    let client = Arc::new(HttpClient::default());
    let resp = fetch_data_from_prometheus(
        client.clone(),
        &host,
        format!(
            "sum by (container) (rate(container_cpu_usage_seconds_total{{co='{}'}}[5m]))",
            container
        ),
        timeout,
    )
    .await;

    match resp {
        Ok(resp) => Ok(resp.data.result),
        Err(error) => Err(error),
    }
}

async fn fetch_data_from_prometheus(
    client: Arc<HttpClient>,
    host: &String,
    query: String,
    timeout: u64,
) -> Result<QueryResponse, PrometheusError> {
    let resp = client
        .get(format!("https://{}/api/v1/query?query={}", host, query,))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<QueryResponse>().await {
            Ok(resp) => Ok(resp),
            Err(error) => Err(PrometheusError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(PrometheusError {
            message: format!("{:?}", error),
        }),
    }
}

pub fn connect_to_prometheus(
    resolver: &mut CronResolver,
    cache: Arc<Addr<RedisActor>>,
    hostname: String,
) -> Addr<PrometheusActor> {
    let actor = PrometheusActor::new(hostname.clone()).start();
    let prom = Arc::<Addr<PrometheusActor>>::new(actor.clone());

    resolve_estimate_container_cpu_utilization_momentum(
        resolver,
        prom.clone(),
        cache.clone(),
        hostname.clone(),
    );
    return actor;
}

fn resolve_estimate_container_cpu_utilization_momentum(
    resolver: &mut CronResolver,
    prom: Arc<Addr<PrometheusActor>>,
    cache: Arc<Addr<RedisActor>>,
    hostname: String,
) {
    resolver.resolve(
        "prometheus.estimate_cpu_usage_momentum".to_string(),
        move |_, _| {
            let prom = prom.clone();
            let cache = cache.clone();

            async move {
                prom.send(GetCpuUsageCommand {
                    container: "MWG".to_string(),
                });
            }
        },
    );
}
