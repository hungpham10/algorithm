use actix::prelude::*;
use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fmt;

pub mod cron;
pub mod dnse;
pub mod fireant;
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

#[derive(Debug, Deserialize, Serialize)]
pub struct CWInfo {
    #[serde(rename = "stockSymbol")]
    pub symbol: String,

    #[serde(rename = "underlyingSymbol")]
    pub underlying: String,

    #[serde(rename = "exercisePrice")]
    pub exercise_price: u64,

    #[serde(rename = "exerciseRatio")]
    pub exercise_ratio: String,

    #[serde(rename = "lastTradingDate")]
    pub last_trading_date: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CWInfoResponse {
    code: String,
    message: String,
    data: Option<Vec<CWInfo>>,
}

pub async fn list_cw() -> Vec<CWInfo> {
    let resp = reqwest::get("https://iboard-query.ssi.com.vn/stock/type/w/hose")
        .await
        .expect("Fail to fetch list of CW")
        .json::<CWInfoResponse>()
        .await
        .expect("Fail to parse list of CW");

    resp.data.unwrap_or_default()
}
