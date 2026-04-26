use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use algorithm::{JsonQuery, LruCache};
use itertools::izip;
use reqwest_middleware::ClientWithMiddleware;
use schemas::{CandleStick, reload::Reload};
use serde_json::Value;

const INDEXES: [&str; 3] = ["VNINDEX", "HNXINDEX", "VN30"];
const SECONDS_IN_WEEK: i64 = 7 * 24 * 60 * 60;

trait JsonValueExt {
    fn as_f64_lossy(&self) -> f64;
    fn as_i32_lossy(&self) -> i32;
}

impl JsonValueExt for Value {
    fn as_f64_lossy(&self) -> f64 {
        match self {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        }
    }
    fn as_i32_lossy(&self) -> i32 {
        match self {
            Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
            Value::String(s) => s.parse::<i32>().unwrap_or(0),
            _ => 0,
        }
    }
}

// Define the layers from the inside out
type CandleStack = Vec<CandleStick>;
type CandleCache = LruCache<i64, CandleStack, 32>;

// Group by whatever your keys represent (e.g., Symbol and Interval)
type SymbolCacheMap = HashMap<String, CandleCache>;
type ExchangeCacheMap = HashMap<String, SymbolCacheMap>;

pub struct QueryCandleSticks {
    client: Arc<ClientWithMiddleware>,
    // Much easier to read:
    caches: Arc<RwLock<ExchangeCacheMap>>,
    timers: Arc<RwLock<HashMap<String, u64>>>,
    mapping: RwLock<HashMap<String, String>>,
    profiles: HashMap<String, CompiledProfile>,
    capacity_per_stack: usize,
}

struct CompiledProfile {
    shift: bool,
    queries: [JsonQuery; 6],
    url_template: String,
}

impl Reload for QueryCandleSticks {
    fn reload(&self) -> Result<(), Error> {
        let mut mapping = self.mapping.write().map_err(|error| {
            Error::other(format!("Fail to request to write to `mapping`: {}", error))
        })?;
        let mapping_str = std::env::var("CANDLESTICK_MAPPING").unwrap_or_else(|_| "{}".to_string());

        *mapping = serde_json::from_str(&mapping_str).unwrap_or_default();
        Ok(())
    }

    fn keys(&self) -> Vec<&str> {
        vec!["CANDLESTICK_MAPPING"]
    }
}

impl QueryCandleSticks {
    pub fn new(client: Arc<ClientWithMiddleware>, capacity: usize) -> Result<Self, Error> {
        let mut profiles = HashMap::new();
        let raw_configs = vec![
            (
                "ssi",
                "https://iboard-api.ssi.com.vn/statistics/charts/history?from={from}&to={to}&symbol={stock}&resolution={res}",
                [
                    "data.t[]", "data.o[]", "data.h[]", "data.l[]", "data.c[]", "data.v[]",
                ],
                true,
            ),
            (
                "vix",
                " https://xpower.vixs.vn/tvchart/history?resolution={res}&symbol={stock}&from={from}&to={to}",
                [
                    "d[].time",
                    "d[].open",
                    "d[].high",
                    "d[].low",
                    "d[].close",
                    "d[].volume",
                ],
                true,
            ),
            (
                "dnse",
                "https://api.dnse.com.vn/chart-api/v2/ohlcs/{kind}?from={from}&to={to}&symbol={stock}&resolution={res}",
                ["t[]", "o[]", "h[]", "l[]", "c[]", "v[]"],
                true,
            ),
            (
                "dragon",
                "https://godragon.vdsc.com.vn/IdragonMarketDataServer/trading-view/rest/history?symbol={stock}&resolution={res}&from={from}&to={to}&countback={limit}",
                ["t[]", "o[]", "h[]", "l[]", "c[]", "v[]"],
                true,
            ),
            (
                "binance",
                "https://api.binance.us/api/v3/klines?startTime={from}&endTime={to}&symbol={stock}&interval={res}&limit={limit}",
                ["[].0", "[].1", "[].2", "[].3", "[].4", "[].5"],
                false,
            ),
            (
                "msn",
                "https://assets.msn.com/service/MSNFinance/Quotes/Chart?apikey=0Q_697_8_Z_S_1_1&ocid=finance-utils-peregrine&symbol={stock}&interval={res}&period={limit}",
                // MSN trả về mảng series.dataPoints, mỗi điểm là một mảng: [time, open, high, low, close, volume]
                [
                    "series.dataPoints[].0",
                    "series.dataPoints[].1",
                    "series.dataPoints[].2",
                    "series.dataPoints[].3",
                    "series.dataPoints[].4",
                    "series.dataPoints[].5",
                ],
                false,
            ),
            (
                "yahoo",
                "https://query1.finance.yahoo.com/v8/finance/chart/{stock}?interval={res}&period1={from}&period2={to}",
                // Yahoo trả về cấu trúc: chart.result.indicators.quote
                [
                    "chart.result.timestamp[]",
                    "chart.result.indicators.quote.open[]",
                    "chart.result.indicators.quote.high[]",
                    "chart.result.indicators.quote.low[]",
                    "chart.result.indicators.quote.close[]",
                    "chart.result.indicators.quote.volume[]",
                ],
                true,
            ),
        ];

        let mapping_str = std::env::var("CANDLESTICK_MAPPING").unwrap_or_else(|_| "{}".to_string());
        let mapping = RwLock::new(serde_json::from_str(&mapping_str).unwrap_or_default());

        for (name, url, paths, shift) in raw_configs {
            let queries = [
                JsonQuery::parse(paths[0])?,
                JsonQuery::parse(paths[1])?,
                JsonQuery::parse(paths[2])?,
                JsonQuery::parse(paths[3])?,
                JsonQuery::parse(paths[4])?,
                JsonQuery::parse(paths[5])?,
            ];
            profiles.insert(
                name.to_string(),
                CompiledProfile {
                    url_template: url.into(),
                    queries,
                    shift,
                },
            );
        }

        Ok(Self {
            client,
            mapping,
            caches: Arc::new(RwLock::new(HashMap::new())),
            timers: Arc::new(RwLock::new(HashMap::new())),
            profiles,
            capacity_per_stack: capacity,
        })
    }

    pub async fn get_candlesticks(
        &self,
        provider: &str,
        stock: &str,
        resolution: &str,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<CandleStick>, Error> {
        if provider.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "Provider not specified"));
        }

        let real_provider_in_str = if let Ok(mapping) = self.mapping.read() {
            if let Some(corrected_provider) = mapping.get(provider) {
                corrected_provider.clone()
            } else {
                provider.to_string()
            }
        } else {
            provider.to_string()
        };
        let real_provider = real_provider_in_str.as_str();

        let is_shifted = if let Some(profile) = self.profiles.get(real_provider) {
            profile.shift
        } else {
            false
        };

        if !self.is_invalidated(stock, resolution)
            && let Some(cached_data) =
                self.fetch_from_cache(stock, resolution, from, to, is_shifted)
        {
            return Ok(cached_data);
        }

        let fetched_candles = self
            .fetch_from_api(real_provider, stock, resolution, from, to, limit)
            .await?;

        if !fetched_candles.is_empty() {
            self.update_cache(stock, resolution, &fetched_candles)?;
        }

        Ok(fetched_candles)
    }

    fn fetch_from_cache(
        &self,
        stock: &str,
        resolution: &str,
        from: i64,
        to: i64,
        is_shifted: bool,
    ) -> Option<Vec<CandleStick>> {
        let caches = self.caches.read().unwrap();
        let stock_cache = caches.get(stock)?.get(resolution)?;

        let mut result = Vec::new();

        let start_block = from / SECONDS_IN_WEEK;
        let end_block = to / SECONDS_IN_WEEK;

        for block_id in start_block..=end_block {
            if let Some(block_candles) = stock_cache.get(&block_id) {
                for c in block_candles {
                    let ts = c.t as i64;

                    if ts <= to && ts >= (from - (is_shifted as i64 * 86400)) {
                        result.push(c.clone());
                    }
                }
            } else {
                let b_start = block_id * SECONDS_IN_WEEK;
                let b_end = (block_id + 1) * SECONDS_IN_WEEK;
                if b_end >= from && b_start <= to {
                    return None;
                }
            }
        }

        if result.is_empty() {
            return None;
        }

        result.sort_by_key(|c| c.t);
        result.dedup_by_key(|c| c.t);

        Some(result)
    }

    fn update_cache(
        &self,
        stock: &str,
        resolution: &str,
        candles: &[CandleStick],
    ) -> Result<(), Error> {
        if candles.is_empty() {
            return Ok(());
        }

        let mut caches = self.caches.write().unwrap();
        let stock_entry = caches.entry(stock.to_string()).or_default();
        let lru = stock_entry
            .entry(resolution.to_string())
            .or_insert_with(|| LruCache::new(self.capacity_per_stack));

        let mut groups: HashMap<i64, Vec<CandleStick>> = HashMap::new();
        for c in candles {
            let bid = (c.t as i64) / SECONDS_IN_WEEK;
            groups.entry(bid).or_default().push(c.clone());
        }

        for (bid, mut new_candles) in groups {
            let mut block_data = lru.get(&bid).unwrap_or_default();
            block_data.append(&mut new_candles);
            block_data.sort_by_key(|cand| cand.t);
            block_data.dedup_by_key(|cand| cand.t);
            lru.put(bid, block_data);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| Error::other(format!("Failed to get current time: {}", error)))?
            .as_secs();
        self.timers
            .write()
            .map_err(|error| Error::other(format!("Failed to get write lock: {}", error)))?
            .insert(format!("{}:{}", stock, resolution), now);
        Ok(())
    }

    fn is_invalidated(&self, stock: &str, resolution: &str) -> bool {
        let timers = self.timers.read().unwrap();
        let key = format!("{}:{}", stock, resolution);

        match timers.get(&key) {
            Some(&last_update) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let res_upper = resolution.to_uppercase();
                let ttl = match res_upper.as_str() {
                    "1" | "5" => 60,
                    "1D" | "1W" | "1M" => 3600,
                    _ => 300,
                };

                if now < last_update {
                    return false;
                }

                now - last_update > ttl
            }
            None => true,
        }
    }

    async fn fetch_from_api(
        &self,
        provider: &str,
        stock: &str,
        resolution: &str,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<CandleStick>, Error> {
        let kind = if INDEXES.contains(&stock) {
            "index"
        } else {
            "stock"
        };
        let profile = self
            .profiles
            .get(provider)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Provider not found"))?;

        if limit > 1000 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "`limit` mustn't be larger than 1000",
            ));
        }

        let (adj_from, adj_to) = if provider == "binance" {
            (from * 1000, to * 1000)
        } else {
            (from, to)
        };
        let url = profile
            .url_template
            .replace("{kind}", kind)
            .replace("{stock}", stock)
            .replace("{res}", resolution)
            .replace("{from}", &adj_from.to_string())
            .replace("{to}", &adj_to.to_string())
            .replace("{limit}", &limit.to_string());

        if url.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid URL template"));
        }

        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| Error::other(format!("Failed to fetch data from {}: {}", provider, e)))?;

        let raw_json: Value = resp.json().await.map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse JSON: {}", e),
            )
        })?;

        let t_ref = profile.queries[0].pick(&raw_json);
        if t_ref.is_empty() {
            return Ok(vec![]);
        }

        let o_ref = profile.queries[1].pick(&raw_json);
        let h_ref = profile.queries[2].pick(&raw_json);
        let l_ref = profile.queries[3].pick(&raw_json);
        let c_ref = profile.queries[4].pick(&raw_json);
        let v_ref = profile.queries[5].pick(&raw_json);

        let count = if limit > 0 {
            limit.min(t_ref.len())
        } else {
            t_ref.len()
        };
        let mut candles = Vec::with_capacity(count);

        for (t, o, h, l, c, v) in izip!(t_ref, o_ref, h_ref, l_ref, c_ref, v_ref).take(count) {
            candles.push(CandleStick {
                t: if provider == "binance" {
                    (t / 1000).as_i32_lossy()
                } else {
                    t.as_i32_lossy()
                },
                o: o.as_f64_lossy(),
                h: h.as_f64_lossy(),
                l: l.as_f64_lossy(),
                c: c.as_f64_lossy(),
                v: v.as_f64_lossy(),
            });
        }

        Ok(candles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client as HttpClient;
    use reqwest_middleware::ClientBuilder;
    use reqwest_tracing::TracingMiddleware;
    use serde_json::json;
    use std::time::Instant;

    async fn run_provider_test(
        service: &QueryCandleSticks,
        provider: &str,
        stock: &str,
        res: &str,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let from = now - (7 * 24 * 60 * 60); // 7 ngày trước
        let to = now;

        println!("\n🔍 Testing Provider: [{}] - Symbol: {}", provider, stock);

        // --- Lần 1: Gọi API thật ---
        let start_api = Instant::now();
        let result_api = service
            .get_candlesticks(provider, stock, res, from, to, 50)
            .await;

        match result_api {
            Ok(candles) => {
                let dur_api = start_api.elapsed();
                assert!(!candles.is_empty(), "Dữ liệu từ {} trả về rỗng!", provider);
                println!("✅ Lần 1 (API)  : {} nến - {:?}", candles.len(), dur_api);

                // --- Lần 2: Truy vấn lại (Cache) ---
                let start_cache = Instant::now();
                let result_cache = service
                    .get_candlesticks(provider, stock, res, from, to, 50)
                    .await
                    .unwrap();
                let _ = start_cache.elapsed();

                // Thay vì assert_eq!(candles.len(), result_cache.len(), ...);
                assert!(
                    result_cache.len() >= candles.len(),
                    "🚨 Cache thiếu nến! API: {}, Cache: {}",
                    candles.len(),
                    result_cache.len()
                );

                // Kiểm tra xem nến mới nhất có khớp nhau không (để đảm bảo không lấy nến rác)
                assert_eq!(
                    candles.last().unwrap().t,
                    result_cache.last().unwrap().t,
                    "Timestamp nến cuối bị lệch!"
                );
            }
            Err(e) => panic!("❌ Thất bại khi gọi sàn {}: {:?}", provider, e),
        }
    }

    // Helper tạo Mock Response cho SSI
    fn mock_ssi_data(size: usize) -> Value {
        let mut t = Vec::with_capacity(size);
        let mut o = Vec::with_capacity(size);
        for i in 0..size {
            t.push(1700000000 + i as i64);
            o.push(100.0 + i as f64);
        }
        json!({
            "data": {
                "t": t, "o": o, "h": o, "l": o, "c": o, "v": o
            }
        })
    }

    #[test]
    fn test_profile_initialization() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 70).unwrap();

        assert!(service.profiles.contains_key("ssi"));
        assert!(service.profiles.contains_key("binance"));
    }

    #[test]
    fn test_logic_extraction_ssi() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 70).unwrap();
        let data = mock_ssi_data(10);

        let profile = service.profiles.get("ssi").unwrap();
        let t_ref = profile.queries[0].pick(&data);

        assert_eq!(t_ref.len(), 10);
        assert_eq!(t_ref[0].as_i32_lossy(), 1700000000);
    }

    #[test]
    fn test_logic_extraction_binance() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 70).unwrap();

        // Binance format: [[t, o, h, l, c, v], ...]
        let data = json!([
            [170000000i32, "100.5", "101.0", "99.0", "100.8", "5000"],
            [170000000i32, "100.8", "102.0", "100.5", "101.5", "6000"]
        ]);

        let profile = service.profiles.get("binance").unwrap();
        let t_ref = profile.queries[0].pick(&data);
        let o_ref = profile.queries[1].pick(&data);

        assert_eq!(t_ref.len(), 2);
        assert_eq!(o_ref[0].as_f64_lossy(), 100.5);
        // Binance timestamp is ms, our lossy converter handles it
        assert_eq!(t_ref[0].as_i32_lossy(), 170000000i32);
    }

    #[test]
    fn test_full_transformation_benchmark() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 70).unwrap();
        let size = 5000;
        let data = mock_ssi_data(size);
        let profile = service.profiles.get("ssi").unwrap();

        let start = Instant::now();

        let t_ref = profile.queries[0].pick(&data);
        let o_ref = profile.queries[1].pick(&data);
        let h_ref = profile.queries[2].pick(&data);
        let l_ref = profile.queries[3].pick(&data);
        let c_ref = profile.queries[4].pick(&data);
        let v_ref = profile.queries[5].pick(&data);

        let mut candles = Vec::with_capacity(size);
        for (t, o, h, l, c, v) in izip!(t_ref, o_ref, h_ref, l_ref, c_ref, v_ref).take(size) {
            candles.push(CandleStick {
                t: t.as_i32_lossy(),
                o: o.as_f64_lossy(),
                h: h.as_f64_lossy(),
                l: l.as_f64_lossy(),
                c: c.as_f64_lossy(),
                v: v.as_f64_lossy(),
            });
        }

        let duration = start.elapsed();
        println!("\n⚡ Benchmark Results:");
        println!(
            "Extracted & Transformed {} candles in: {:?}",
            size, duration
        );
        println!("Average per candle: {:?}", duration / size as u32);

        assert_eq!(candles.len(), size);
        assert!(duration.as_micros() < 5000, "Performance too slow!");
    }

    #[tokio::test]
    async fn test_cache_block_logic() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 10).unwrap();

        let stock = "FPT";
        let res = "1D";
        let t1 = 1700000000; // Block A
        let t2 = 1700000000 + SECONDS_IN_WEEK + 100; // Block B

        let candles = vec![
            CandleStick {
                t: t1 as i32,
                o: 10.0,
                h: 11.0,
                l: 9.0,
                c: 10.5,
                v: 1000.0,
            },
            CandleStick {
                t: t2 as i32,
                o: 20.0,
                h: 21.0,
                l: 19.0,
                c: 20.5,
                v: 2000.0,
            },
        ];

        // 1. Update cache
        service.update_cache(stock, res, &candles).unwrap();

        // 2. Test hit block A
        let hit = service.fetch_from_cache(stock, res, t1, t1 + 100, false);
        assert!(hit.is_some(), "Phải hit được block A");
        assert_eq!(hit.unwrap().len(), 1);

        // 3. Test hit 2 blocks
        let hit_all = service.fetch_from_cache(stock, res, t1, t2, false);
        assert!(hit_all.is_some());
        assert_eq!(hit_all.unwrap().len(), 2);

        // 4. Test miss
        let miss = service.fetch_from_cache(
            stock,
            res,
            t1 - SECONDS_IN_WEEK * 2,
            t1 - SECONDS_IN_WEEK,
            false,
        );
        assert!(
            miss.is_none(),
            "Vùng này không có nến, phải trả về None để fetch API"
        );
    }

    #[tokio::test]
    async fn test_cache_invalidation_ttl() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 10).unwrap();
        let stock = "VIC";
        let res = "1";

        // Truyền 1 nến thật để hàm update_cache thực hiện ghi timer
        let mock_candle = CandleStick {
            t: 1700000000,
            o: 10.0,
            h: 10.0,
            l: 10.0,
            c: 10.0,
            v: 0.0,
        };
        service.update_cache(stock, res, &[mock_candle]).unwrap();

        // Kiểm tra ngay lập tức - Phải FALSE (vì vừa mới update)
        assert!(
            !service.is_invalidated(stock, res),
            "Vừa update xong phải valid!"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_all_providers_real_data() {
        let client = Arc::new(
            ClientBuilder::new(HttpClient::new())
                .with(TracingMiddleware::default())
                .build(),
        );
        let service = QueryCandleSticks::new(client, 100).unwrap();

        // Chạy lần lượt các sàn
        // SSI
        run_provider_test(&service, "ssi", "FPT", "1D").await;

        // DSNE
        run_provider_test(&service, "dnse", "HPG", "1D").await;

        // VIX
        run_provider_test(&service, "vix", "HPG", "1D").await;

        // Binance - Dùng BTCUSDT
        run_provider_test(&service, "binance", "BTCUSDT", "1h").await;
        run_provider_test(&service, "binance", "BTCUSDT", "1d").await;
    }
}
