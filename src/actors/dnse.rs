use std::time::Duration;
use std::sync::Arc;
use std::fmt;

use serde::{Deserialize, Serialize};
use juniper::GraphQLObject;
use reqwest::{
    Client as HttpClient, 
    Error as HttpError,
};

use actix::prelude::*;
use actix::Addr;

pub struct DnseActor {
    timeout: u64,
}

impl DnseActor {
    fn new() -> Self {
        Self {
            timeout: 60,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Ohcl {
    pub t: Vec<i32>,
    pub o: Vec<f64>,
    pub c: Vec<f64>,
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub v: Vec<i32>,
    pub nextTime: i64,
}

#[derive(GraphQLObject)]
pub struct CandleStick {
    pub t: i32,
    pub o: f64,
    pub h: f64,
    pub c: f64,
    pub l: f64,
    pub v: i32,
}

impl Actor for DnseActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct DnseError {
    message: String
}

impl fmt::Display for DnseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Vec<CandleStick>, HttpError>")]
pub struct GetOHCLCommand {
    pub resolution: String,
    pub stock:      String,
    pub from:       i32,
    pub to:         i32,
}

impl Handler<GetOHCLCommand> for DnseActor {
    type Result = ResponseFuture<Result<Vec<CandleStick>, HttpError>>;

    fn handle(&mut self, msg: GetOHCLCommand, _: &mut Self::Context) -> Self::Result { 
        let resolution = msg.resolution.clone();
        let stock      = msg.stock.clone();
        let from       = msg.from;
        let to         = msg.to;
        let timeout    = self.timeout;

        Box::pin(async move {
            let client     = Arc::new(HttpClient::default());
            let datapoints = fetch_ohcl_by_stock(
                    client.clone(),
                    &stock,
                    &resolution,
                    from,
                    to,
                    timeout,
                )
                .await;

            return datapoints;
        })
    }
}

async fn fetch_ohcl_by_stock(
    client:     Arc<HttpClient>,
    stock:      &String,
    resolution: &String,
    from:       i32, 
    to:         i32,
    timeout:    u64,
) -> Result<Vec<CandleStick>, HttpError> {
    let resp = client.get(format!(
            "https://services.entrade.com.vn/chart-api/v2/ohlcs/stock?from={}&to={}&symbol={}&resolution={}",
            from,
            to,
            (*stock),
            (*resolution),
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => { 
            let mut candles = Vec::<CandleStick>::new();
            let ohcl        = match resp.json::<Ohcl>().await {
                Ok(ohcl) => ohcl,
                Err(_)   => Ohcl{
                    t:        Vec::<i32>::new(),
                    o:        Vec::<f64>::new(),
                    c:        Vec::<f64>::new(),
                    h:        Vec::<f64>::new(),
                    l:        Vec::<f64>::new(),
                    v:        Vec::<i32>::new(),
                    nextTime: -1,
                },
            };

            for i in 0..ohcl.t.len() {
                candles.push(CandleStick{
                    t: ohcl.t[i],
                    o: ohcl.o[i],
                    h: ohcl.h[i],
                    c: ohcl.c[i],
                    l: ohcl.l[i],
                    v: ohcl.v[i],
                })
            }

            Ok(candles)
        }
        Err(error) => Err(error),
    }
}

pub fn connect_to_dnse() -> Addr<DnseActor> {
    DnseActor::new()
        .start()
}
