use anyhow::Result;
use statrs::distribution::{ContinuousCDF, Normal};

use std::f64::consts::PI;
use std::io;

#[inline]
/// Tính giá quyền chọn mua (Call Option) bằng công thức Black-Scholes.
/// S: Giá tài sản cơ sở
/// K: Giá thực hiện (Strike Price)
/// T: Thời gian đáo hạn (tính bằng năm)
/// r: Lãi suất phi rủi ro (liên tục)
/// sigma: Độ biến động (Volatility)
pub fn black_scholes(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> Result<f64> {
    // C = S * N(d1) - K * e^(-rT) * N(d2)
    // d1 = (ln(S/K) + (r + sigma^2/2) * T) / (sigma * sqrt(T)
    // d2 = d1 - sigma * sqrt(T)

    let norm = Normal::new(0.0, 1.0)?;

    let d1 = ((s / k).ln() + (r + sigma.powi(2) / 2.0) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();

    Ok(s * norm.cdf(d1) - k * (-r * t).exp() * norm.cdf(d2))
}

#[inline]
pub fn log_likelihood(y_true: &[f64], y_pred: &[f64], eps: f64) -> f64 {
    let y_pred_clipped = y_pred
        .iter()
        .map(|&x| x.max(eps).min(1.0 - eps))
        .collect::<Vec<_>>();

    y_true
        .iter()
        .zip(y_pred_clipped.iter())
        .map(|(t, p)| t * p.ln() + (1.0 - t) * (1.0 - p).ln())
        .sum()
}
