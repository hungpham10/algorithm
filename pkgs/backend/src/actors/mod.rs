use actix::prelude::*;
use chrono::{Datelike, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use log::info;

use std::error::Error;
use std::fmt;

pub mod cron;
pub mod fireant;
pub mod price;
pub mod tcbs;
pub mod vps;

const FUZZY_TRIGGER_THRESHOLD: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct ActorError {
    pub message: String,
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ActorError {}

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct UpdateStocksCommand {
    pub stocks: Vec<String>,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), ActorError>")]
pub struct FlushVariablesCommand;

#[derive(Message)]
#[rtype(result = "Result<f64, ActorError>")]
pub struct GetVariableCommand {
    pub symbol: String,
    pub variable: String,
}

pub async fn list_active_stocks() -> Vec<String> {
    list_of_vn30().await
}

pub async fn list_of_hose() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/hose")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_midcap() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/VNMID")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_penny() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/VNSML")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_vn30() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/VN30")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_vn100() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/VN100")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_etf() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/getlistckindex/hsx_e")
        .send()
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Industry {
    data: Vec<String>,
}

pub async fn list_of_industry(industry_code: &str) -> Vec<String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap();
    let industry = client
        .get(format!(
            "https://histdatafeed.vps.com.vn/industry/symbols/{}",
            industry_code
        ))
        .send()
        .await
        .unwrap()
        .json::<Industry>()
        .await
        .unwrap();
    industry.data
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Future {
    #[serde(rename = "SYMBOL")]
    symbol: String,

    #[serde(rename = "CHART_CODE")]
    code: String,

    #[serde(rename = "FULL_NAME")]
    full_name: String,
}

pub async fn list_futures() -> Vec<String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap()
        .get("https://bgapidatafeed.vps.com.vn/pslistmap")
        .send()
        .await
        .unwrap()
        .json::<Vec<Future>>()
        .await
        .unwrap()
        .iter()
        .map(|item| item.symbol.clone())
        .collect()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CWInfo {
    #[serde(rename = "StockCode")]
    pub symbol: String,

    #[serde(rename = "underlyingAsset", skip_serializing_if = "Option::is_none")]
    pub underlying: Option<String>,

    #[serde(rename = "ExcercisePrice", skip_serializing_if = "Option::is_none")]
    pub exercise_price: Option<f64>,

    #[serde(rename = "ExcerciseRatio", skip_serializing_if = "Option::is_none")]
    pub exercise_ratio: Option<String>,

    #[serde(rename = "LastTradingDate")]
    pub last_trading_date: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CWInfoResponse {
    #[serde(rename = "listStock")]
    data: Option<Vec<CWInfo>>,
}

pub async fn list_cw() -> Result<Vec<CWInfo>, ActorError> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .map_err(|error| ActorError {
            message: format!("Fail to build client: {}", error),
        })?;
    let from = Utc::now();
    let now = from + Duration::days(365);
    let resp = client.get(
        format!(
            "https://livedragon.vdsc.com.vn//general/cwHistoryMainBoardInfo.rv?fromDate={from_year:04}-{from_month:02}-{from_day:02}&toDate={to_year:04}-{to_month:02}-{to_day:02}&mode=ALL",
            from_year = from.year(),
            from_month = from.month(),
            from_day = from.day(),
            to_year = now.year(),
            to_month = now.month(),
            to_day = now.day(),
        )
    )
    .send()
    .await
    .map_err(|error| ActorError {
        message: format!("Fail to fetch list of CW: {}", error),
    })?
    .json::<CWInfoResponse>()
    .await
    .map_err(|error| ActorError {
        message: format!("Fail to parse list of CW: {}", error),
    })?;

    Ok(resp.data.unwrap_or_default())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CryptoInfo {
    symbol: String,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CryptoInfoResponse {
    symbols: Vec<CryptoInfo>,
}

pub async fn list_crypto() -> Vec<String> {
    let resp = reqwest::get("https://api.binance.us/api/v1/exchangeInfo")
        .await
        .expect("Fail to fetch list of crypto")
        .json::<CryptoInfoResponse>()
        .await
        .expect("Fail to parse list of crypto");

    resp.symbols
        .iter()
        .filter_map(|item| {
            if item.status == "TRADING" {
                Some(item.symbol.clone())
            } else {
                None
            }
        })
        .collect()
}
