/// Tests use Rust built-in #[test] framework executed via `cargo test`.

use std::collections::HashMap;
use chrono::{NaiveDate, NaiveDateTime};
use polars::prelude::*;
use backend::repl::market::*;
use backend::actors::vps::Price;
use backend::actors::dnse::OHCL;
use lazy_static::lazy_static;

#[cfg(test)]
mod mocks {
    use super::*;
    use std::sync::Mutex;

    lazy_static! {
        pub static ref CALL_LOG: Mutex<Vec<String>> = Mutex::new(vec![]);
    }

    pub async fn list_of_vn30() -> Vec<String> {
        CALL_LOG.lock().unwrap().push("vn30".into());
        vec!["AAA".into(), "BBB".into()]
    }

    pub async fn list_of_vn100() -> Vec<String> {
        CALL_LOG.lock().unwrap().push("vn100".into());
        vec!["CCC".into(), "DDD".into()]
    }

    pub async fn list_of_industry(code: &str) -> Vec<String> {
        vec![format!("{}_X", code)]
    }

    pub fn connect_to_vps(_symbols: &[String]) -> MockAddr {
        MockAddr
    }
    pub struct MockAddr;
    impl MockAddr {
        pub async fn send(&self, _cmd: GetPriceCommand) -> Result<Vec<Price>, ()> {
            Ok(vec![
                Price {
                    sym: "AAA".into(),
                    lastPrice: 1.0,
                    lastVolume: 10.0,
                    changePc: "0.5".into(),
                    fBVol: "1".into(),
                    fSVolume: "2".into(),
                    g1: "1.1|11".into(),
                    g2: "1.2|12".into(),
                    g3: "1.3|13".into(),
                    g4: "0.9|9".into(),
                    g5: "0.8|8".into(),
                    g6: "0.7|7".into(),
                }
            ])
        }
    }

    pub fn connect_to_dnse() -> MockOHCLAddr {
        MockOHCLAddr
    }
    pub struct MockOHCLAddr;
    impl MockOHCLAddr {
        pub async fn send(&self, _cmd: GetOHCLCommand) -> Result<Vec<OHCL>, ()> {
            Ok(vec![
                OHCL {
                    t: 1,
                    o: 1.0,
                    h: 1.2,
                    l: 0.9,
                    c: 1.1,
                    v: 100,
                }
            ])
        }
    }
}

fn init_runtime() -> actix_rt::Runtime {
    actix_rt::Runtime::new().expect("runtime")
}

#[test]
fn test_industry_codes_contains_expected() {
    assert_eq!(INDUSTRY_CODES.get("petroleum"), Some(&"0500"));
    assert!(INDUSTRY_CODES.get("nonexistent").is_none());
}

#[test]
fn test_sectors_returns_all_keys() {
    let mut expected: Vec<_> = INDUSTRY_CODES.keys().map(|k| k.to_string()).collect();
    expected.sort();
    let mut actual = sectors();
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn test_vn30_vn100_calls() {
    let rt = init_runtime();
    let vn30_syms = rt.block_on(vn30());
    assert_eq!(vn30_syms, vec!["AAA".to_string(), "BBB".to_string()]);
    let vn100_syms = rt.block_on(vn100());
    assert_eq!(vn100_syms, vec!["CCC".to_string(), "DDD".to_string()]);
}

#[test]
fn test_industry_valid_and_invalid() {
    let rt = init_runtime();
    let petroleum = rt.block_on(industry("petroleum".into()));
    assert_eq!(petroleum, vec!["0500_X".to_string()]);
    let unknown = rt.block_on(industry("unknown".into()));
    assert!(unknown.is_empty());
}

#[test]
fn test_market_dataframe() {
    let df = market(vec!["AAA".into()]).unwrap().0;
    assert_eq!(df.shape(), (1, 18));
    assert_eq!(df.column("symbol").unwrap().utf8().unwrap().get(0), Some("AAA"));
    assert_eq!(df.column("price_plus1").unwrap().f64().unwrap().get(0), Some(1.1));
}

#[test]
fn test_market_empty() {
    let res = market(vec![]);
    assert!(res.is_err());
}

#[test]
fn test_price_dataframe_dates() {
    let df = price("AAA".into(), "D".into(), "1970-01-01".into(), "1970-01-02".into())
        .unwrap()
        .0;
    assert_eq!(df.shape().1, 6);
    let first_ts = df.column("t").unwrap().datetime().unwrap().get(0);
    assert_eq!(first_ts, Some(NaiveDateTime::from_timestamp(1, 0)));
}