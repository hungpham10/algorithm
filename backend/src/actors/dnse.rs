use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client as HttpClient, Error as HttpError};
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use crate::schemas::CandleStick;

const INDEXES: [&str; 3] = ["VNINDEX", "HNXINDEX", "VN30"];

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
    pub v: Option<Vec<i64>>,
    pub nextTime: Option<i64>,
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

impl Handler<super::HealthCommand> for DnseActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Vec<(f64, f64)>, HttpError>")]
pub struct GetVolumeProfileCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
    pub number_of_levels: i64,
}

impl Handler<GetVolumeProfileCommand> for DnseActor {
    type Result = ResponseFuture<Result<Vec<(f64, f64)>, HttpError>>;

    fn handle(&mut self, msg: GetVolumeProfileCommand, _: &mut Self::Context) -> Self::Result {
        let number_of_levels = msg.number_of_levels;
        let resolution = msg.resolution.clone();
        let stock = msg.stock.clone();
        let from = msg.from;
        let to = msg.to;
        let timeout = self.timeout;

        Box::pin(async move {
            let mut volumes = vec![0.0; number_of_levels as usize];
            let mut prices = vec![0.0; number_of_levels as usize];

            let client = Arc::new(HttpClient::default());
            let candles =
                fetch_ohcl_by_stock(client.clone(), &stock, &resolution, from, to, timeout).await;

            match candles {
                Ok(candles) => {
                    let mut ret = Vec::new();
                    let max_price = candles
                        .iter()
                        .map(|candle| candle.h)
                        .fold(f64::MIN, f64::max);
                    let min_price = candles
                        .iter()
                        .map(|candle| candle.l)
                        .fold(f64::MAX, f64::min);
                    let price_step = (max_price - min_price) / number_of_levels as f64;

                    candles.iter().for_each(|candle| {
                        let price_range = candle.h - candle.l;
                        let volume_per_price = candle.v / price_range;

                        for level in 0..number_of_levels {
                            let price_level_low = min_price + (level as f64) * price_step;
                            let price_level_high = min_price + ((level + 1) as f64) * price_step;

                            let overlap_start = candle.l.max(price_level_low);
                            let overlap_end = candle.h.min(price_level_high);

                            if overlap_start < overlap_end {
                                volumes[level as usize] +=
                                    volume_per_price * (overlap_end - overlap_start);

                                // @TODO: better calculation to estimate the price level center
                                //        according to the volume, rather than using the average
                                prices[level as usize] = (price_level_low + price_level_high) / 2.0;
                            }
                        }
                    });

                    for level in 0..number_of_levels {
                        ret.push((prices[level as usize], volumes[level as usize]));
                    }
                    Ok(ret)
                }
                Err(error) => Err(error),
            }
        })
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

            fetch_ohcl_by_stock(client.clone(), &stock, &resolution, from, to, timeout).await
        })
    }
}

pub async fn fetch_ohcl_by_stock(
    client: Arc<HttpClient>,
    stock: &String,
    resolution: &String,
    from: i64,
    to: i64,
    timeout: u64,
) -> Result<Vec<CandleStick>, HttpError> {
    let mut kind = "stock";

    if INDEXES.iter().any(|&s| s == *stock) {
        kind = "index";
    }

    let resp = client.get(format!(
            "https://services.entrade.com.vn/chart-api/v2/ohlcs/{}?from={}&to={}&symbol={}&resolution={}",
            kind,
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
                            Some(v) => v[i] as f64,
                            None => 0.0,
                        },
                    })
                }
            }

            Ok(candles)
        }
        Err(error) => Err(error),
    }
}

pub fn list_of_resolution() -> Vec<String> {
    vec!["1D".to_string(), "1M".to_string(), "1W".to_string()]
}

pub fn connect_to_dnse() -> Addr<DnseActor> {
    DnseActor::new().start()
}
