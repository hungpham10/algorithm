use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{anyhow, Result};

use vnscope::algorithm::genetic::{Genetic, Individual, Model, Player};
use vnscope::schemas::CandleStick;

use crate::api::ohcl::v1::OhclResponse;

#[derive(Clone)]
struct Data {
    candles: Arc<Vec<CandleStick>>,
    range: usize,
    begin: usize,
    split: usize,
    end: usize,
}

impl Data {
    fn new(candles: Arc<Vec<CandleStick>>, range: usize) -> Self {
        Self {
            candles,
            range,
            begin: 0,
            split: 0,
            end: 0,
        }
    }

    fn shuttle(&mut self) {
        let len = self.candles.len();

        if len < 400 {
            self.begin = 0;
            self.split = 0;
            self.end = len;
            return;
        }

        use rand::Rng;
        let min_end = self.range;
        let max_end = len;
        let end = rand::thread_rng().gen_range(min_end..=max_end);

        let max_begin = end - self.range;
        let begin = if max_begin > 0 {
            rand::thread_rng().gen_range(0..=max_begin)
        } else {
            0
        };

        let split = rand::thread_rng().gen_range(begin..end);

        self.begin = begin;
        self.split = split;
        self.end = end;
    }
}

#[derive(Clone)]
struct Investor {
    // @NOTE: shared arguments
    data: Arc<RwLock<Data>>,

    // @NOTE:
    money: f64,
    stock: f64,
}

impl Investor {
    pub fn new(data: Arc<RwLock<Data>>, money: f64, stock: f64) -> Self {
        Self { data, money, stock }
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

struct Spot {
    data: Arc<RwLock<Data>>,
    money: f64,
    stock: f64,
}

impl Model<Investor> for Spot {
    fn random(&self) -> Result<Investor> {
        Ok(Investor::new(self.data.clone(), self.money, self.stock))
    }

    fn mutate(&self, item: &mut Investor, arguments: &Vec<f64>, index: usize) -> Result<()> {
        Ok(())
    }

    fn crossover(&self, father: &Investor, mother: &Investor) -> Result<Investor> {
        Ok(Investor::new(self.data.clone(), self.money, self.stock))
    }

    fn extinguish(&self, item: &Individual<Investor>) -> Result<bool> {
        Ok(true)
    }
}

struct Simulator {
    genetic: Option<Arc<Mutex<Genetic<Investor, Spot>>>>,
    candles: Option<Vec<CandleStick>>,
    money: Option<f64>,
    stock: Option<f64>,

    arguments: Vec<Vec<f64>>,
    paccentor: f64,
    pmutation: f64,
    session: i64,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            candles: None,
            genetic: None,
            money: None,
            stock: None,
            session: 0,
            arguments: Vec::new(),
            paccentor: 0.1,
            pmutation: 0.1,
        }
    }

    pub fn with_money(&mut self, money: f64) {
        self.money = Some(money);
    }

    pub fn with_stock(&mut self, stock: f64) {
        self.stock = Some(stock);
    }

    pub async fn with_sampling(
        &mut self,
        provider: &str,
        market: &str,
        symbol: &str,
        resolution: &str,
        from: i64,
        to: i64,
    ) -> Result<()> {
        let resp = reqwest::get(format!(
            "https://{}/api/investing/v1/ohcl/{}/{}?resolution={}&from={}&to={}",
            provider, market, symbol, resolution, from, to,
        ))
        .await;

        match resp {
            Ok(resp) => {
                self.candles = Some(
                    resp.json::<OhclResponse>()
                        .await
                        .map_err(|error| anyhow!("Failed parsing candlesticks: {:?}", error))?
                        .ohcl
                        .unwrap_or(Vec::new()),
                );
                Ok(())
            }
            Err(error) => Err(anyhow!("Failed fetching candlesticks: {:?}", error)),
        }
    }

    pub async fn with_genetic(
        &mut self,
        capacity: usize,
        n_loop: usize,
        d_range: usize,
    ) -> Result<()> {
        if self.genetic.is_none() {
            self.genetic = Some(Arc::new(Mutex::new(Genetic::new(
                capacity,
                Arc::new(Spot {
                    data: Arc::new(RwLock::new(Data::new(
                        Arc::new(self.candles.clone().ok_or(anyhow!(
                            "Not found candles, please call with_sampling first"
                        ))?),
                        d_range,
                    ))),
                    money: self.money.ok_or(anyhow!("Not found money"))?,
                    stock: self.stock.unwrap_or(0.0),
                }),
            ))));
        }

        let mut genetic = self
            .genetic
            .as_ref()
            .unwrap()
            .lock()
            .map_err(|error| anyhow!("Failed to lock genetic: {}", error))?;

        for i in 0..n_loop {
            genetic.initialize(((capacity as f64) * self.paccentor) as usize)?;
            genetic.fluctuate(
                self.session + i as i64,
                self.arguments.clone(),
                self.pmutation,
            )?;

            genetic.estimate(self.session + i as i64);
        }

        Ok(())
    }
}

pub async fn run() -> std::io::Result<()> {
    let mut sim = Simulator::new();

    sim.with_money(1_000_000_000.0);
    sim.with_stock(0.0);
    sim.with_sampling(
        "lighttrading.pp.ua",
        "stock",
        "MWG",
        "1D",
        1704607984,
        1755127894,
    )
    .await
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;
    sim.with_genetic(100, 1000, 400)
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;
    Ok(())
}
