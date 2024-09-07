use std::error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::future;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware as HttpClient};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sentry::capture_error;
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use influxdb::{Client as InfluxClient, InfluxDbWriteable};

use crate::actors::cron::CronResolver;
use crate::schemas::tsdb::Order;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Price {
    pub id: i64,
    pub sym: String,
    pub mc: String,
    pub c: f64,
    pub f: f64,
    pub r: f64,
    pub lastPrice: f64,
    pub lastVolume: f64,
    pub lot: u64,
    pub ot: String,
    pub changePc: String,
    pub avePrice: String,
    pub highPrice: String,
    pub lowPrice: String,
    pub fBVol: String,
    pub fBValue: String,
    pub fSVolume: String,
    pub fSValue: String,
    pub fRoom: String,
    pub g1: String,
    pub g2: String,
    pub g3: String,
    pub g4: String,
    pub g5: String,
    pub g6: String,
    pub g7: String,
    pub mp: String,
    pub CWUnderlying: String,
    pub CWType: String,
    pub CWLastTradingDate: String,
    pub CWExcersisePrice: String,
    pub CWExerciseRatio: String,
    pub CWListedShare: String,
    pub sType: String,
    pub sBenefit: String,
}

#[derive(Debug, Clone)]
pub struct VpsError {
    message: String,
}

impl fmt::Display for VpsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for VpsError {}

pub struct VpsActor {
    stocks: Vec<String>,
    timeout: u64,
}

impl VpsActor {
    fn new(stocks: Vec<String>) -> Self {
        Self {
            stocks: stocks,
            timeout: 300,
        }
    }
}

impl Actor for VpsActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

impl Handler<HealthCommand> for VpsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct UpdateStocksCommand {
    pub stocks: Vec<String>,
}

impl Handler<UpdateStocksCommand> for VpsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, msg: UpdateStocksCommand, _: &mut Self::Context) -> Self::Result {
        self.stocks = msg.stocks.clone();

        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<Price>")]
pub struct GetPriceCommand;

impl Handler<GetPriceCommand> for VpsActor {
    type Result = ResponseFuture<Vec<Price>>;

    fn handle(&mut self, _msg: GetPriceCommand, _: &mut Self::Context) -> Self::Result {
        let stocks = self.stocks.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let datapoints = fetch_price_depth(&stocks, timeout).await;

            return datapoints;
        })
    }
}

async fn fetch_price_depth(stocks: &Vec<String>, timeout: u64) -> Vec<Price> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
    let client = Arc::new(
        ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build(),
    );
    let blocks: Vec<Vec<String>> = (*stocks)
        .chunks(100)
        .map(|block| block.iter().map(|stock| (*stock).clone()).collect())
        .collect();

    future::try_join_all(
        blocks
            .iter()
            .map(move |block| fetch_price_depth_per_block(client.clone(), block, timeout)),
    )
    .await
    .unwrap()
    .into_iter()
    .flatten()
    .collect()
}

async fn fetch_price_depth_per_block(
    client: Arc<HttpClient>,
    block: &Vec<String>,
    timeout: u64,
) -> Result<Vec<Price>, VpsError> {
    let resp = client
        .get(format!(
            "https://bgapidatafeed.vps.com.vn/getliststockdata/{}",
            (*block).join(","),
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<Price>>().await {
            Ok(resp) => Ok(resp),
            Err(error) => Err(VpsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(VpsError {
            message: format!("{:?}", error),
        }),
    }
}

pub async fn list_of_vn30() -> Vec<String> {
    reqwest::get("https://bgapidatafeed.vps.com.vn/listvn30")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub fn connect_to_vps(
    resolver: &mut CronResolver,
    tsdb: Arc<InfluxClient>,
    stocks: Vec<String>,
) -> Addr<VpsActor> {
    let actor = VpsActor::new(stocks).start();
    let vps = actor.clone();

    resolver.resolve("vps.get_price_command".to_string(), move || {
        let vps = vps.clone();
        let tsdb = tsdb.clone();

        async move {
            let datapoints = match vps.send(GetPriceCommand).await {
                Ok(datapoints) => datapoints,
                Err(error) => {
                    capture_error(&error);

                    // @NOTE: ignore this error, only return empty BTreeMap
                    Vec::<Price>::new()
                }
            };
            let order_insert = datapoints
                .iter()
                .map(|point| {
                    let g1 = point.g1.split("|").collect::<Vec<&str>>();
                    let g2 = point.g2.split("|").collect::<Vec<&str>>();
                    let g3 = point.g3.split("|").collect::<Vec<&str>>();
                    let g4 = point.g4.split("|").collect::<Vec<&str>>();
                    let g5 = point.g5.split("|").collect::<Vec<&str>>();
                    let g6 = point.g6.split("|").collect::<Vec<&str>>();

                    Order {
                        // @NOTE: clock
                        time: chrono::offset::Utc::now().into(),

                        // @NOTE: price
                        PricePlus1: g4[0].parse::<f64>().unwrap(),
                        PricePlus2: g5[0].parse::<f64>().unwrap(),
                        PricePlus3: g5[0].parse::<f64>().unwrap(),
                        PriceMinus1: g1[0].parse::<f64>().unwrap(),
                        PriceMinus2: g2[0].parse::<f64>().unwrap(),
                        PriceMinus3: g3[0].parse::<f64>().unwrap(),

                        // @NOTE: volume
                        VolumePlus1: g4[1].parse::<i64>().unwrap(),
                        VolumePlus2: g5[1].parse::<i64>().unwrap(),
                        VolumePlus3: g6[1].parse::<i64>().unwrap(),
                        VolumeMinus1: g1[1].parse::<i64>().unwrap(),
                        VolumeMinus2: g2[1].parse::<i64>().unwrap(),
                        VolumeMinus3: g3[1].parse::<i64>().unwrap(),
                    }
                    .into_query(point.sym.clone())
                })
                .collect::<Vec<_>>();

            tsdb.clone().query(order_insert).await;
        }
    });

    return actor.clone();
}
