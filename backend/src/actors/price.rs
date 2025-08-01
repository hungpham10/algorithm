use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use crate::actors::ActorError;
use crate::schemas::CandleStick;

const INDEXES: [&str; 3] = ["VNINDEX", "HNXINDEX", "VN30"];

pub struct PriceActor {
    timeout: u64,
    provider: String,
}

impl PriceActor {
    fn new(provider: &str) -> Self {
        Self {
            timeout: 60,
            provider: provider.to_string(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SsiOhclWrapper {
    code: String,
    data: Ohcl,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Ohcl {
    t: Option<Vec<i32>>,
    o: Option<Vec<f64>>,
    c: Option<Vec<f64>>,
    h: Option<Vec<f64>>,
    l: Option<Vec<f64>>,
    v: Option<Vec<i64>>,
    nextTime: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Kline {
    timestamp: i64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    close_time: i64,
    quote_volume: String,
    trade_count: u64,
    taker_buy_volume: String,
    taker_buy_quote_volume: String,
    _ignored: String,
}

impl Actor for PriceActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct PriceError {
    message: String,
}

impl fmt::Display for PriceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Handler<super::HealthCommand> for PriceActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Vec<(f64, f64)>, ActorError>")]
pub struct GetVolumeProfileCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
    pub number_of_levels: i64,
}

impl Handler<GetVolumeProfileCommand> for PriceActor {
    type Result = ResponseFuture<Result<Vec<(f64, f64)>, ActorError>>;

    fn handle(&mut self, msg: GetVolumeProfileCommand, _: &mut Self::Context) -> Self::Result {
        let number_of_levels = msg.number_of_levels;
        let resolution = msg.resolution.clone();
        let provider = self.provider.clone();
        let stock = msg.stock.clone();
        let from = msg.from;
        let to = msg.to;
        let timeout = self.timeout;

        Box::pin(async move {
            let mut volumes = vec![0.0; number_of_levels as usize];
            let mut prices = vec![0.0; number_of_levels as usize];

            let client = Arc::new(HttpClient::default());
            let candles = fetch_ohcl_by_stock(
                client.clone(),
                &provider,
                &stock,
                &resolution,
                from,
                to,
                timeout,
            )
            .await;

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
#[rtype(result = "Result<Vec<CandleStick>, ActorError>")]
pub struct GetOHCLCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
}

impl Handler<GetOHCLCommand> for PriceActor {
    type Result = ResponseFuture<Result<Vec<CandleStick>, ActorError>>;

    fn handle(&mut self, msg: GetOHCLCommand, _: &mut Self::Context) -> Self::Result {
        let resolution = msg.resolution.clone();
        let provider = self.provider.clone();
        let stock = msg.stock.clone();
        let from = msg.from;
        let to = msg.to;
        let timeout = self.timeout;

        Box::pin(async move {
            let client = Arc::new(HttpClient::default());

            fetch_ohcl_by_stock(
                client.clone(),
                &provider,
                &stock,
                &resolution,
                from,
                to,
                timeout,
            )
            .await
        })
    }
}

pub async fn fetch_ohcl_by_stock(
    client: Arc<HttpClient>,
    provider: &String,
    stock: &String,
    resolution: &String,
    from: i64,
    to: i64,
    timeout: u64,
) -> Result<Vec<CandleStick>, ActorError> {
    let mut kind = "stock";

    if INDEXES.iter().any(|&s| s == *stock) {
        kind = "index";
    }

    if provider.as_str() == "ssi" {
        let resp = client.get(format!(
            "https://iboard-api.ssi.com.vn/statistics/charts/history?from={}&to={}&symbol={}&resolution={}",
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
                let ohcl = resp
                    .json::<SsiOhclWrapper>()
                    .await
                    .map_err(|error| ActorError {
                        message: format!("{}", error),
                    })?
                    .data;

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

                    Ok(candles)
                } else {
                    Err(ActorError {
                        message: format!("cannot fetch any data from provider"),
                    })
                }
            }
            Err(error) => Err(ActorError {
                message: format!("{}", error),
            }),
        }
    } else if provider.as_str() == "dnse" {
        let resp = client
            .get(format!(
            "https://api.dnse.com.vn/chart-api/v2/ohlcs/{}?from={}&to={}&symbol={}&resolution={}",
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
                let ohcl = resp.json::<Ohcl>().await.map_err(|error| ActorError {
                    message: format!("{}", error),
                })?;

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

                    Ok(candles)
                } else {
                    Err(ActorError {
                        message: format!("cannot fetch any data from provider"),
                    })
                }
            }
            Err(error) => Err(ActorError {
                message: format!("{}", error),
            }),
        }
    } else if provider == "binance" {
        let mut candles = Vec::<CandleStick>::new();
        let mut from = from * 1000;
        let to = to * 1000;

        for _ in 0..10 {
            let resp = client.get(format!(
                    "https://api.binance.com/api/v3/klines?startTime={}&endTime={}&symbol={}&interval={}&limit=1000",
                    from,
                    to,
                    (*stock),
                    (*resolution).to_lowercase(),
                ))
                .timeout(Duration::from_secs(timeout))
                .send()
                .await;

            match resp {
                Ok(resp) => {
                    let klines = resp
                        .json::<Vec<Kline>>()
                        .await
                        .map_err(|error| ActorError {
                            message: format!("{}", error),
                        })?;

                    if klines.len() == 0 {
                        break;
                    }
                    if klines[0].timestamp == klines.last().unwrap().timestamp {
                        break;
                    }

                    for it in &klines {
                        candles.push(CandleStick {
                            t: (it.timestamp / 1000) as i32,
                            o: it.open.parse().map_err(|error| ActorError {
                                message: format!("{}", error),
                            })?,
                            h: it.high.parse().map_err(|error| ActorError {
                                message: format!("{}", error),
                            })?,
                            c: it.close.parse().map_err(|error| ActorError {
                                message: format!("{}", error),
                            })?,
                            l: it.low.parse().map_err(|error| ActorError {
                                message: format!("{}", error),
                            })?,
                            v: it.volume.parse().map_err(|error| ActorError {
                                message: format!("{}", error),
                            })?,
                        })
                    }

                    from = klines.last().unwrap().close_time;
                }
                Err(error) => {
                    return Err(ActorError {
                        message: format!("{}", error),
                    });
                }
            }
        }

        Ok(candles)
    } else {
        Err(ActorError {
            message: format!("No support {}", provider),
        })
    }
}

pub fn list_of_resolution() -> Vec<String> {
    vec!["1D".to_string(), "1M".to_string(), "1W".to_string()]
}

pub fn connect_to_price(provider: &str) -> Addr<PriceActor> {
    PriceActor::new(provider).start()
}
