use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use log::debug;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use actix::prelude::*;
use actix::Addr;

use crate::actors::ActorError;
use crate::algorithm::lru::LruCache;
use crate::schemas::CandleStick;

const INDEXES: [&str; 3] = ["VNINDEX", "HNXINDEX", "VN30"];

pub struct PriceActor {
    // @NOTE: caching
    size_of_block_in_cache: i64,
    num_of_available_cache: usize,
    caches: Arc<RwLock<BTreeMap<String, BTreeMap<String, LruCache<i64, Vec<CandleStick>>>>>>,

    // @NOTE: parameters
    timeout: u64,
}

impl PriceActor {
    fn new() -> Self {
        Self {
            size_of_block_in_cache: 24 * 60 * 60 * 7, // 1 week
            num_of_available_cache: 70,
            caches: Arc::new(RwLock::new(BTreeMap::new())),
            timeout: 60,
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
struct BinanceError {
    code: i64,
    msg: String,
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
    _quote_volume: String,
    _trade_count: u64,
    _taker_buy_volume: String,
    _taker_buy_quote_volume: String,
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
#[rtype(result = "Result<(), ActorError>")]
pub struct UpdateOHCLToCacheCommand {
    pub candles: Vec<CandleStick>,
    pub resolution: String,
    pub stock: String,
}

impl Handler<UpdateOHCLToCacheCommand> for PriceActor {
    type Result = ResponseFuture<Result<(), ActorError>>;

    fn handle(&mut self, msg: UpdateOHCLToCacheCommand, _: &mut Self::Context) -> Self::Result {
        let caches = self.caches.clone(); // Clone Arc để dùng trong future
        let size_of_block = self.size_of_block_in_cache;
        let capacity = self.num_of_available_cache;
        let stock = msg.stock.clone();
        let resolution = msg.resolution.clone();
        let candles = msg.candles;

        debug!(
            "{} -> {}",
            candles.first().unwrap().t,
            candles.last().unwrap().t
        );
        Box::pin(async move {
            if candles.is_empty() {
                return Err(ActorError {
                    message: "Empty candle list provided".to_string(),
                });
            }

            // Group candles vào blocks
            let mut block_map: BTreeMap<i64, Vec<CandleStick>> = BTreeMap::new();
            for candle in candles {
                let block_id = (candle.t as i64) / size_of_block;
                block_map
                    .entry(block_id)
                    .or_insert_with(Vec::new)
                    .push(candle);
            }

            if block_map.is_empty() {
                return Err(ActorError {
                    message: "No valid blocks created from candles".to_string(),
                });
            }

            // Write lock để update cache
            let mut caches_write = caches.write().map_err(|error| ActorError {
                message: format!("Fail to access cache: {}", error),
            })?;

            // Tạo nested BTreeMap nếu chưa có
            let stock_caches = caches_write
                .entry(stock.clone())
                .or_insert_with(BTreeMap::new);

            // Tạo LRU cache nếu chưa có
            let cache = stock_caches
                .entry(resolution.clone())
                .or_insert_with(|| LruCache::new(capacity));

            // Insert blocks vào LRU cache
            for (block_id, block_candles) in block_map {
                debug!("Update block {}", block_id);
                cache.put(block_id, block_candles);
            }

            Ok(())
        })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(Vec<CandleStick>, bool), ActorError>")]
pub struct GetOHCLCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
    pub broker: String,
    pub limit: usize,
}

impl Handler<GetOHCLCommand> for PriceActor {
    type Result = ResponseFuture<Result<(Vec<CandleStick>, bool), ActorError>>;

    fn handle(&mut self, msg: GetOHCLCommand, _: &mut Self::Context) -> Self::Result {
        let mut from = msg.from;
        let to = msg.to;
        let broker = msg.broker.clone();
        let resolution = msg.resolution.clone();
        let stock = msg.stock.clone();
        let limit = msg.limit;
        let timeout = self.timeout;
        let size_of_block = self.size_of_block_in_cache;
        let caches = self.caches.clone(); // Clone Arc để share vào future

        Box::pin(async move {
            // Read lock để check cache (non-blocking)
            let caches_read = caches.read().map_err(|error| ActorError {
                message: format!("Fail to read cache: {}", error),
            })?;

            if let Some(stock_caches) = caches_read.get(&stock) {
                if let Some(cache) = stock_caches.get(&resolution) {
                    let mut result = Vec::new();
                    let mut keep = true;
                    let i_from = from / size_of_block;
                    let i_to = to / size_of_block;

                    for i in i_from..=i_to {
                        if let Some(candles) = cache.get(&i) {
                            // Filter candles trong range (tối ưu: nếu sorted, dùng binary search)
                            for candle in candles {
                                let candle_time = candle.t as i64;
                                if from <= candle_time && candle_time < to {
                                    result.push(candle.clone());
                                }
                                if candle_time >= to
                                    || (limit > 0 && (i * size_of_block) as usize > limit)
                                {
                                    break;
                                }
                            }
                        } else {
                            debug!(
                                "Cannot find block {}, timestamp at {}",
                                i,
                                i * size_of_block
                            );

                            from = (i - 1) * size_of_block;
                            keep = false;
                            break;
                        }
                    }

                    if keep {
                        return Ok((result, false)); // Cache hit full
                    }
                } else {
                    from = (from / size_of_block - 1) * size_of_block;
                }
            } else {
                from = (from / size_of_block - 1) * size_of_block;
            }
            drop(caches_read); // Release read lock sớm

            Ok((
                fetch_ohcl_by_stock(
                    Arc::new(HttpClient::default()),
                    &broker,
                    &stock,
                    &resolution,
                    from,
                    to,
                    limit + (size_of_block as usize) * 2,
                    timeout,
                )
                .await?,
                true,
            )) // Return fetched data, đánh dấu là fresh
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
    limit: usize,
    timeout: u64,
) -> Result<Vec<CandleStick>, ActorError> {
    let mut kind = "stock";

    if INDEXES.iter().any(|&s| s == *stock) {
        kind = "index";
    }

    if provider == "ssi" {
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
                        if limit > 0 && i >= limit {
                            break;
                        }

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
    } else if provider == "dnse" || provider == "stock" {
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
                        if limit > 0 && i >= limit {
                            break;
                        }

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
    } else if provider == "binance" || provider == "crypto" {
        let mut candles = Vec::<CandleStick>::new();
        let mut from = from * 1000;
        let to = to * 1000;
        let limit = if limit == 0 { 1000 } else { limit };

        for _ in 0..10 {
            let resp = client.get(format!(
                    "https://api.binance.us/api/v3/klines?startTime={}&endTime={}&symbol={}&interval={}&limit={}",
                    from,
                    to,
                    (*stock),
                    (*resolution).to_lowercase(),
                    limit,
                ))
                .timeout(Duration::from_secs(timeout))
                .send()
                .await;

            match resp {
                Ok(resp) => {
                    let json_value: Value = resp.json().await.map_err(|error| ActorError {
                        message: format!("Failed to parse JSON: {}", error),
                    })?;

                    let klines = serde_json::from_value::<Vec<Kline>>(json_value.clone()).map_err(
                        |error| match serde_json::from_value::<BinanceError>(json_value) {
                            Ok(error) => ActorError {
                                message: format!(
                                    "API error: code={}, reason={}",
                                    error.code, error.msg
                                ),
                            },
                            Err(_) => ActorError {
                                message: format!("Failed to parse Error message: {}", error),
                            },
                        },
                    )?;

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

pub fn connect_to_price() -> Addr<PriceActor> {
    PriceActor::new().start()
}
