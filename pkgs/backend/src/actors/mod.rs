use actix::prelude::*;
use serde::{Deserialize, Serialize};

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
    reqwest::get("https://bgapidatafeed.vps.com.vn/getlistckindex/hose")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_midcap() -> Vec<String> {
    reqwest::get("https://bgapidatafeed.vps.com.vn/getlistckindex/VNMID")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_penny() -> Vec<String> {
    reqwest::get("https://bgapidatafeed.vps.com.vn/getlistckindex/VNSML")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_vn30() -> Vec<String> {
    reqwest::get("https://bgapidatafeed.vps.com.vn/getlistckindex/VN30")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap()
}

pub async fn list_of_vn100() -> Vec<String> {
    reqwest::get("https://bgapidatafeed.vps.com.vn/getlistckindex/VN100")
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
    let industry = reqwest::get(format!(
        "https://histdatafeed.vps.com.vn/industry/symbols/{}",
        industry_code
    ))
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
    reqwest::get("https://bgapidatafeed.vps.com.vn/pslistmap")
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
    #[serde(rename = "code")]
    pub symbol: String,

    #[serde(rename = "underlyingAsset")]
    pub underlying: String,

    #[serde(rename = "exercisePrice", skip_serializing_if = "Option::is_none")]
    pub exercise_price: Option<f64>,

    #[serde(rename = "exerciseRatio")]
    pub exercise_ratio: String,

    #[serde(rename = "lastTradingDate")]
    pub last_trading_date: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CWInfoResponse {
    data: Option<Vec<CWInfo>>,
}

pub async fn list_cw() -> Result<Vec<CWInfo>, ActorError> {
    let resp = reqwest::get(
        "https://api-finfo.vndirect.com.vn/v4/derivatives?q=derType:CW~status:LISTED&size=1000",
    )
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
