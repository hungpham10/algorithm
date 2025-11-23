use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::OnceLock;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{debug, warn};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use actix::prelude::*;
use actix::Addr;

use crate::actors::ActorError;
use crate::algorithm::lru::LruCache;
use crate::schemas::CandleStick;

const INDEXES: [&str; 3] = ["VNINDEX", "HNXINDEX", "VN30"];

static CACHE_TTL_CONFIG: OnceLock<CacheTtlConfig> = OnceLock::new();

#[derive(Clone, Debug)]
struct CacheTtlConfig {
    ttls: HashMap<String, u64>,
    default_ttl: u64, // fallback khi resolution không match
}

impl CacheTtlConfig {
    fn load_from_env() -> Self {
        let default_ttl = std::env::var("CACHE_DEFAULT_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60); // mặc định 1 phút như cũ

        let mut ttls = HashMap::new();

        // Ví dụ các env bạn sẽ set:
        // CACHE_TTL_1=60
        // CACHE_TTL_5=300
        // CACHE_TTL_1H=3600
        // CACHE_TTL_1D=86400
        // CACHE_TTL_1W=604800
        // ...

        let possible_resolutions = vec![
            "1", "3", "5", "15", "30", "45", "1H", "4H", "1D", "1W", "1M",
        ];

        for res in possible_resolutions {
            if let Ok(seconds) = std::env::var(format!("CACHE_TTL_{}", res)) {
                if let Ok(sec) = seconds.parse::<u64>() {
                    ttls.insert(res.to_string(), sec);
                }
            }
        }

        if ttls.is_empty() {
            ttls.insert("1".into(), 60);
            ttls.insert("5".into(), 300);
            ttls.insert("15".into(), 900);
            ttls.insert("1H".into(), 3600);
            ttls.insert("4H".into(), 4 * 3600);
            ttls.insert("1D".into(), 86400);
            ttls.insert("1W".into(), 7 * 86400);
        }

        Self { ttls, default_ttl }
    }

    fn get(&self, resolution: &str) -> u64 {
        if let Some(&ttl) = self.ttls.get(resolution) {
            ttl
        } else if let Ok(minutes) = resolution.parse::<u64>() {
            // Các resolution kiểu "5", "15", "30"...
            minutes * 60
        } else {
            self.default_ttl
        }
    }
}

fn cache_ttl_config() -> &'static CacheTtlConfig {
    CACHE_TTL_CONFIG.get_or_init(|| CacheTtlConfig::load_from_env())
}

pub struct PriceActor {
    // @NOTE: caching
    size_of_block_in_cache: i64,
    num_of_available_cache: usize,
    caches: Arc<RwLock<BTreeMap<String, BTreeMap<String, LruCache<i64, Vec<CandleStick>>>>>>,
    timers: Arc<RwLock<BTreeMap<String, usize>>>,

    // @NOTE: parameters
    timeout: u64,
}

impl PriceActor {
    fn new() -> Self {
        Self {
            size_of_block_in_cache: 24 * 60 * 60 * 7, // 1 week
            num_of_available_cache: 70,
            caches: Arc::new(RwLock::new(BTreeMap::new())),
            timers: Arc::new(RwLock::new(BTreeMap::new())),
            timeout: 60,
        }
    }

    fn is_cache_invalidated(&self, stock: &str, resolution: &str) -> bool {
        let key = format!("{}:{}", stock, resolution);

        let last_update_sec = {
            let timers = self.timers.read().unwrap();
            if let Some(&ts) = timers.get(&key) {
                ts as u64
            } else {
                return true; // chưa có cache → invalidated
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let max_age_sec = cache_ttl_config().get(resolution);

        now.saturating_sub(last_update_sec) > max_age_sec
    }

    fn update_cache_timestamp(&self, stock: &str, resolution: &str) {
        let key = format!("{}:{}", stock, resolution);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let now_usize = now as usize;

        let mut timers = self.timers.write().unwrap();
        timers.insert(key, now_usize);
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct DragonOhcl {
    t: Option<Vec<i32>>,
    o: Option<Vec<String>>,
    c: Option<Vec<String>>,
    h: Option<Vec<String>>,
    l: Option<Vec<String>>,
    v: Option<Vec<String>>,
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

        self.update_cache_timestamp(&stock, &resolution);

        Box::pin(async move {
            if candles.is_empty() {
                return Err(ActorError {
                    message: "Empty candle list provided".to_string(),
                });
            }

            let mut block_map: BTreeMap<i64, Vec<CandleStick>> = BTreeMap::new();
            let latest = candles.last().unwrap().t as i64;
            let last_block = latest / size_of_block + ((latest % size_of_block) as i64);

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

            let mut caches_write = caches.write().map_err(|error| ActorError {
                message: format!("Fail to access cache: {}", error),
            })?;

            let stock_caches = caches_write
                .entry(stock.clone())
                .or_insert_with(BTreeMap::new);

            let cache = stock_caches
                .entry(resolution.clone())
                .or_insert_with(|| LruCache::new(capacity));

            let block_size = block_map
                .iter()
                .map(|(_, block_candles)| block_candles.len())
                .max()
                .ok_or(ActorError {
                    message: format!("Failed to calculate block size"),
                })?;

            for (block_id, block_candles) in block_map {
                let uncover = ((block_size - block_candles.len()) as f64) / block_size as f64;

                cache.put(block_id, block_candles.clone());
                if (uncover > 0.05) && (block_id != last_block) {
                    debug!(
                        "Block {} has uncover above 0.05 ({}, {}, {})",
                        block_id,
                        uncover,
                        block_candles.len(),
                        block_size
                    );
                }
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
        let mut to = msg.to;
        let broker = msg.broker.clone();
        let resolution = msg.resolution.clone();
        let stock = msg.stock.clone();
        let limit = msg.limit;
        let timeout = self.timeout;
        let size_of_block = self.size_of_block_in_cache;
        let caches = self.caches.clone();
        let is_invalid = self.is_cache_invalidated(&stock, &resolution);

        Box::pin(async move {
            // Read lock để check cache (non-blocking)
            let caches_read = caches.read().map_err(|error| ActorError {
                message: format!("Fail to read cache: {}", error),
            })?;
            let mut result = Vec::new();

            if is_invalid {
                warn!("cache is invalidated now");
            } else {
                if let Some(stock_caches) = caches_read.get(&stock) {
                    if let Some(cache) = stock_caches.get(&resolution) {
                        let mut keep = true;
                        let mut first = to;
                        let mut last = from;
                        let i_from =
                            (from / size_of_block) - (((from % size_of_block) != 0) as i64);
                        let i_to = (to / size_of_block) + (((to % size_of_block) != 0) as i64);

                        for i in i_from..=i_to {
                            if let Some(candles) = cache.get(&i) {
                                for candle in candles {
                                    let candle_time = candle.t as i64;
                                    if from <= candle_time && candle_time < to {
                                        result.push(candle.clone());

                                        if first > (candle.t as i64) {
                                            first = candle.t as i64
                                        }
                                        if last < (candle.t as i64) {
                                            last = candle.t as i64
                                        }
                                    }
                                    if candle_time >= to
                                        || (limit > 0 && (i * size_of_block) as usize > limit)
                                    {
                                        break;
                                    }
                                }
                            } else {
                                from = (i - 1) * size_of_block;
                                keep = false;
                                break;
                            }
                        }

                        if !keep {
                            if ((last - first) as f64) / ((to - from) as f64) > 0.90 {
                                keep = true;
                            } else {
                                debug!(
                                    "Only cover {}",
                                    ((last - first) as f64) / ((to - from) as f64)
                                );
                            }
                        }

                        if keep || from > to {
                            return Ok((result, false)); // Cache hit full
                        }
                    } else {
                        debug!("Not found any cache data");
                        from = (from / size_of_block - 1) * size_of_block;
                        to = (to / size_of_block + 1) * size_of_block;
                    }
                } else {
                    debug!("Not found any cache data");
                    from = (from / size_of_block - 1) * size_of_block;
                    to = (to / size_of_block + 1) * size_of_block;
                }
            }
            drop(caches_read); // Release read lock sớm

            let tail = fetch_ohcl_by_stock(
                Arc::new(HttpClient::default()),
                &broker,
                &stock,
                &resolution,
                from,
                to,
                if limit > 0 {
                    limit + (size_of_block as usize) * 2
                } else {
                    0
                },
                timeout,
            )
            .await?;

            if result.len() > 0 {
                for candle in tail {
                    let last = result.len() - 1;

                    if result[last].t < candle.t {
                        result.push(candle);
                    } else if result[last].t == candle.t {
                        result[last] = candle;
                    }
                }
            } else {
                result = tail;
            }

            Ok((result, true)) // Return fetched data, đánh dấu là fresh
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
    let default_stock = std::env::var("DEFAULT_STOCK").unwrap_or_else(|_| "dnse".to_string());

    if INDEXES.iter().any(|&s| s == *stock) {
        kind = "index";
    }

    let provider = if (provider == "dragon" || default_stock == "dragon") && kind == "index" {
        "dnse"
    } else {
        provider
    };

    if provider == "ssi" || (provider == "stock" && default_stock == "ssi") {
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
    } else if provider == "dnse" || (provider == "stock" && default_stock == "dnse") {
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
    } else if provider == "dragon" || (provider == "stock" && default_stock == "dragon") {
        let limit = if limit == 0 { 1_000_000 } else { limit };
        let resp = client
            .get(format!(
                "https://godragon.vdsc.com.vn/IdragonMarketDataServer/trading-view/rest/history?lang=vi&symbol={}&resolution={}&from={}&to={}&countback={}",
                (*stock),
                (*resolution),
                from,
                to,
                limit,
            ))
            .timeout(Duration::from_secs(timeout))
            .send()
            .await;

        match resp {
            Ok(resp) => {
                let mut candles = Vec::<CandleStick>::new();
                let ohcl = resp
                    .json::<DragonOhcl>()
                    .await
                    .map_err(|error| ActorError {
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
                                Some(o) => o[i].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Fail parse Open: {}", error),
                                })?,
                                None => 0.0,
                            },
                            h: match ohcl.h.as_ref() {
                                Some(h) => h[i].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Fail parse High: {}", error),
                                })?,
                                None => 0.0,
                            },
                            c: match ohcl.c.as_ref() {
                                Some(c) => c[i].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Fail parse Close: {}", error),
                                })?,
                                None => 0.0,
                            },
                            l: match ohcl.l.as_ref() {
                                Some(l) => l[i].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Fail parse Low: {}", error),
                                })?,
                                None => 0.0,
                            },
                            v: match ohcl.v.as_ref() {
                                Some(v) => v[i].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Fail parse Volume: {}", error),
                                })?,
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
        let limit = if limit == 0 { 1_000_000 } else { limit };

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
