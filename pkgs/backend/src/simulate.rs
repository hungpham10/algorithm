use anyhow::{anyhow, Result};
use std::sync::Arc;

use vnscope::algorithm::genetic::{Genetic, Individual, Model, Player};
use vnscope::schemas::CandleStick;

use crate::api::ohcl::v1::OhclResponse;

#[derive(Clone)]
struct Investor {
    candles: Arc<Vec<CandleStick>>,
}

impl Investor {
    pub fn new(candles: Arc<Vec<CandleStick>>) -> Self {
        Self { candles }
    }
}

impl Player for Investor {
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn estimate(&self) -> f64 {
        0.0
    }

    fn gene(&self) -> Vec<f64> {
        Vec::new()
    }
}

struct Stock {
    candles: Arc<Vec<CandleStick>>,
}

impl Model<Investor> for Stock {
    fn mutate(&self, item: &mut Investor, arguments: &Vec<f64>, index: usize) -> Result<()> {
        Ok(())
    }

    fn crossover(&self, father: &Investor, mother: &Investor) -> Result<Investor> {
        Ok(Investor::new(self.candles.clone()))
    }

    fn extinguish(&self, item: &Individual<Investor>) -> Result<bool> {
        Ok(true)
    }
}

pub async fn run() -> std::io::Result<()> {
    Ok(())
}

pub async fn simulate_single_stock_tradding(
    kind: String,
    symbol: String,
    resolution: String,
    from: i64,
    to: i64,
) -> Result<()> {
    let resp = reqwest::get(format!(
        "https://lighttrading.pp.ua/api/investing/v1/ohcl/{}/{}?resolution={}&from={}&to={}",
        kind, symbol, resolution, from, to,
    ))
    .await;

    match resp {
        Ok(resp) => {
            let resp = resp
                .json::<OhclResponse>()
                .await
                .map_err(|error| anyhow!("Failed parsing candlesticks: {:?}", error))?;

            let genetic = Genetic::new(
                1,
                Arc::new(Stock {
                    candles: Arc::new(resp.ohcl.unwrap()),
                }),
            );
            Ok(())
        }
        Err(error) => Err(anyhow!("Failed fetching candlesticks: {:?}", error)),
    }
}
