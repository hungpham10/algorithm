use std::io::Error;

use chrono::NaiveDate;

use ta::Next;
use ta::indicators::ExponentialMovingAverage;

use ndarray::prelude::*;

use crate::api::investing::v1::OhclResponse;
use schemas::CandleStick;

// @NOTE: định nghĩa các hàm số
// 1. Gaussian Matrix
fn gaussian(x: &Array2<f64>, mu: f64, sigma: f64) -> Array2<f64> {
    let variance2 = 2.0 * sigma.powi(2);
    // Tính toán element-wise
    x.mapv(|val| (-(val - mu).powi(2) / variance2).exp())
}

// 2. Sigmoid Matrix
fn sigmoid(x: &Array2<f64>, a: f64, c: f64) -> Array2<f64> {
    x.mapv(|val| 1.0 / (1.0 + (-a * (val - c)).exp()))
}

// AND có trọng số: Nhân element-wise cả 3 ma trận
fn and(l: &Array2<f64>, r: &Array2<f64>, w: &Array2<f64>) -> Array2<f64> {
    l * r * w
}

// OR có trọng số: Áp dụng trọng số lên kết quả của phép OR đại số
fn or(l: &Array2<f64>, r: &Array2<f64>, w: &Array2<f64>) -> Array2<f64> {
    let algebraic_sum = l + r - (l * r);
    algebraic_sum * w
}

// Diff Norm có trọng số: Kết hợp Zip để xử lý chia cho 0 và nhân trọng số
fn diff_norm(l: &Array2<f64>, r: &Array2<f64>, w: &Array2<f64>) -> Array2<f64> {
    Zip::from(l).and(r).and(w).map_collect(|&a, &b, &weight| {
        if b.abs() < f64::EPSILON {
            0.0
        } else {
            ((a - b) / b) * weight
        }
    })
}

async fn fetch_candles(
    broker: &str,
    symbol: &str,
    from: &str,
    to: &str,
) -> Result<Vec<CandleStick>, Error> {
    let from_ts = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|error| Error::other(format!("Invalid 'from' date: {error}")))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::other(format!("Failed to setup hms")))?
        .and_utc()
        .timestamp();

    let to_ts = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|error| Error::other(format!("Invalid 'to' date: {error}")))?
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| Error::other(format!("Failed to setup hms")))?
        .and_utc()
        .timestamp();

    let resp = reqwest::get(format!(
            "https://lighttrading.pp.ua/api/investing/v1/ohcl/symbols/{broker}/{symbol}?resolution=1D&from={from_ts}&to={to_ts}&limit=0"
        ))
        .await
        .map_err(|error| Error::other(format!("Query failed: {error}")))?
        .json::<OhclResponse>()
        .await
        .map_err(|error| Error::other(format!("Parsing failed: {error}")))?;

    if resp.error.is_some() {
        Err(Error::other(format!("Respons failed: {:?}", resp.error)))
    } else {
        Ok(resp.ohcl.ok_or_else(|| Error::other("Empty response"))?)
    }
}

pub async fn run() -> std::io::Result<()> {
    let candles = fetch_candles("dnse", "SSI", "2022-03-27", "2026-03-27").await?;
    let mut short_ema = ExponentialMovingAverage::new(55).unwrap();
    let mut long_ema = ExponentialMovingAverage::new(200).unwrap();

    for (i, candle) in candles.iter().enumerate() {
        let short_ema = short_ema.next(candle.c);
        let long_ema = long_ema.next(candle.c);
        let ema_diff_norm = diff_norm(short_ema, long_ema);
        let short_diff_norm = diff_norm(short_ema, candle.c);
        let long_diff_norm = diff_norm(long_ema, candle.c);

        if i > 200 {}
    }
    Ok(())
}
