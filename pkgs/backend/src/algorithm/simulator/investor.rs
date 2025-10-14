use anyhow::{anyhow, Result};
use log::warn;
use nalgebra::DVector;
use std::sync::{Arc, RwLock};

use crate::algorithm::genetic::Player;
use crate::algorithm::simulator::{Data, Phase};
use crate::schemas::CandleStick;

#[derive(Clone)]
pub struct Investor {
    // @NOTE: shared arguments
    data: Arc<RwLock<Data>>,
    phase: Arc<RwLock<Phase>>,

    // @NOTE: factors
    factors: DVector<f64>,

    // @NOTE: configuration
    fee: f64,
    initialize_money: f64,
    initialize_stock: f64,
    holding_period: usize,
    minimum_buy: usize,
}

impl Investor {
    pub fn new(
        data: Arc<RwLock<Data>>,
        phase: Arc<RwLock<Phase>>,
        money: f64,
        stock: f64,
        stock_holding_period: usize,
        minimum_stock_buy: usize,
        factors: DVector<f64>,
    ) -> Self {
        Self {
            data,
            factors,
            phase,
            fee: 0.001,
            initialize_money: money,
            initialize_stock: stock,
            holding_period: stock_holding_period,
            minimum_buy: minimum_stock_buy,
        }
    }

    pub fn set_factor(&mut self, i: usize, value: f64) -> f64 {
        self.factors[i] = value;
        value
    }

    pub fn get_factor(&self, i: usize) -> f64 {
        self.factors[i]
    }

    pub fn list_factors(&self) -> &DVector<f64> {
        &self.factors
    }

    pub fn logit(&self, candles: &[CandleStick]) -> f64 {
        let window = candles.len();
        if window == 0 {
            return 0.0;
        }

        let required_factors = 5 * window + 1;
        if self.factors.len() < required_factors {
            warn!(
                "Insufficient factors: expected at least {}, got {}",
                required_factors,
                self.factors.len()
            );
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..window {
            let candle = &candles[i];
            let prev_candle = if i == 0 { candle } else { &candles[i - 1] };

            let delta_o = (candle.o - prev_candle.c) / prev_candle.c;
            let delta_h = (candle.h - prev_candle.c) / prev_candle.c;
            let delta_c = (candle.c - prev_candle.c) / prev_candle.c;
            let delta_l = (candle.l - prev_candle.c) / prev_candle.c;
            let v_scaled = candle.v / 10_000_000.0;

            sum += self.factors[5 * i] * delta_o
                + self.factors[5 * i + 1] * delta_h
                + self.factors[5 * i + 2] * delta_c
                + self.factors[5 * i + 3] * delta_l
                + self.factors[5 * i + 4] * v_scaled;
        }

        let normalized = sum / (window as f64).sqrt();
        let bias = self.factors.iter().last().copied().unwrap_or(0.0);

        (normalized + bias).clamp(-3.0, 3.0)
    }

    pub fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}

impl Player for Investor {
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn estimate(&self) -> Result<f64> {
        let mut money = self.initialize_money;
        let mut stock = self.initialize_stock;

        let phase = self
            .phase
            .read()
            .map_err(|error| anyhow!("Failed read phase: {}", error))?;
        let data = self
            .data
            .read()
            .map_err(|error| anyhow!("Failed read data: {}", error))?;

        for i in 0..data.size(&phase) {
            let candles_window = data.sample(i, &phase)?;
            let p_buy = Self::sigmoid(self.logit(candles_window));

            if p_buy > 0.75 {
                let buy_candle = data.last_candle(i, &phase)?;
                if money > (self.minimum_buy as f64) * buy_candle.c {
                    money -= (1.0 + self.fee) * self.minimum_buy as f64 * buy_candle.c;
                    stock += self.minimum_buy as f64;

                    if i + self.holding_period < data.size(&phase) {
                        let sell_candle = data.last_candle(i + self.holding_period, &phase)?;
                        money += (1.0 - self.fee) * self.minimum_buy as f64 * sell_candle.o;
                        stock -= self.minimum_buy as f64;
                    }
                }
            }
        }

        if data.size(&phase) == 0 {
            Err(anyhow!(
                "Failed estimating data: data size is 0 cause nothing to calculate"
            ))
        } else {
            let last_price = data.last_candle(data.size(&phase) - 1, &phase)?.c;
            Ok((money + stock * last_price - self.initialize_money) / self.initialize_money)
        }
    }

    fn gene(&self) -> DVector<f64> {
        self.factors.clone()
    }
}
