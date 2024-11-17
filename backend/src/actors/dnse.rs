use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use juniper::GraphQLObject;
use log::info;
use reqwest::{Client as HttpClient, Error as HttpError};
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

pub struct DnseActor {
    timeout: u64,
}

impl DnseActor {
    fn new() -> Self {
        Self { timeout: 60 }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Ohcl {
    pub t: Option<Vec<i32>>,
    pub o: Option<Vec<f64>>,
    pub c: Option<Vec<f64>>,
    pub h: Option<Vec<f64>>,
    pub l: Option<Vec<f64>>,
    pub v: Option<Vec<i32>>,
    pub nextTime: i64,
}

#[derive(GraphQLObject)]
#[graphql(description = "Information about japaness candle stick")]
pub struct CandleStick {
    #[graphql(description = "timestamp")]
    pub t: i32,

    #[graphql(description = "open price")]
    pub o: f64,

    #[graphql(description = "highest price")]
    pub h: f64,

    #[graphql(description = "close price")]
    pub c: f64,

    #[graphql(description = "lowest price")]
    pub l: f64,

    #[graphql(description = "volume")]
    pub v: i32,
}

impl Actor for DnseActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct DnseError {
    message: String,
}

impl fmt::Display for DnseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

impl Handler<HealthCommand> for DnseActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Vec<CandleStick>, HttpError>")]
pub struct GetOHCLCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
}

impl Handler<GetOHCLCommand> for DnseActor {
    type Result = ResponseFuture<Result<Vec<CandleStick>, HttpError>>;

    fn handle(&mut self, msg: GetOHCLCommand, _: &mut Self::Context) -> Self::Result {
        let resolution = msg.resolution.clone();
        let stock = msg.stock.clone();
        let from = msg.from;
        let to = msg.to;
        let timeout = self.timeout;

        Box::pin(async move {
            let client = Arc::new(HttpClient::default());
            let datapoints =
                fetch_ohcl_by_stock(client.clone(), &stock, &resolution, from, to, timeout).await;

            return datapoints;
        })
    }
}

async fn fetch_ohcl_by_stock(
    client: Arc<HttpClient>,
    stock: &String,
    resolution: &String,
    from: i64,
    to: i64,
    timeout: u64,
) -> Result<Vec<CandleStick>, HttpError> {
    let resp = client.get(format!(
            "https://services.entrade.com.vn/chart-api/v2/ohlcs/stock?from={}&to={}&symbol={}&resolution={}",
            from / 1000,
            to / 1000,
            (*stock),
            (*resolution),
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => {
            info!("{:?}", resp);

            let mut candles = Vec::<CandleStick>::new();
            let ohcl = resp.json::<Ohcl>().await.unwrap();

            if let Some(t) = ohcl.t {
                for i in 0..t.len() {
                    candles.push(CandleStick {
                        t: t[i],
                        o: match ohcl.o.as_ref() {
                            Some(o) => o[i],
                            None => 0.0,
                        },
                        h: match ohcl.h.as_ref() {
                            Some(h) => h[i],
                            None => 0.0,
                        },
                        c: match ohcl.c.as_ref() {
                            Some(c) => c[i],
                            None => 0.0,
                        },
                        l: match ohcl.l.as_ref() {
                            Some(l) => l[i],
                            None => 0.0,
                        },
                        v: match ohcl.v.as_ref() {
                            Some(v) => v[i],
                            None => 0,
                        },
                    })
                }
            }

            Ok(candles)
        }
        Err(error) => Err(error),
    }
}

pub fn connect_to_dnse() -> Addr<DnseActor> {
    DnseActor::new().start()
}
