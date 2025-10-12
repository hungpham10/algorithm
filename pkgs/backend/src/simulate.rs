use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use nalgebra::DVector;
use rand::Rng;

use anyhow::{anyhow, Result};
use infisical::{AuthMethod, Client as InfiscalClient};
use reqwest;

use vnscope::algorithm::cmaes::{Convex, Sampling};
use vnscope::algorithm::genetic::{Genetic, Individual, InfluxDb, Model, Player};
use vnscope::algorithm::simulator::{Data, Investor, Phase, Spot};
use vnscope::schemas::CandleStick;

use crate::api::get_secret_from_infisical;
use crate::api::ohcl::v1::OhclResponse;

struct Simulator {
    genetic: Option<Arc<Mutex<Genetic<Investor, Spot>>>>,
    candles: Option<Vec<CandleStick>>,
    money: Option<f64>,
    stock: Option<f64>,
    arguments: Vec<Vec<f64>>,
    pmutation: f64,
    session: i64,
    stock_holding_period: usize,
    minimum_stock_buy: usize,
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
            pmutation: 0.1,
            minimum_stock_buy: 100,
            stock_holding_period: 10,
        }
    }

    pub fn with_minimum_stock_buy(&mut self, value: usize) {
        self.minimum_stock_buy = value
    }

    pub fn with_stock_holding_period(&mut self, value: usize) {
        self.stock_holding_period = value
    }

    pub fn with_arguments(&mut self, arguments: Vec<Vec<f64>>) {
        self.arguments = arguments;
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
            "https://{}/api/investing/v1/ohcl/{}/{}?resolution={}&from={}&to={}&limit=0",
            provider, market, symbol, resolution, from, to,
        ))
        .await?;
        self.candles = Some(
            resp.json::<OhclResponse>()
                .await
                .map_err(|error| anyhow!("Failed parsing candlesticks: {:?}", error))?
                .ohcl
                .unwrap_or(Vec::new()),
        );
        Ok(())
    }

    pub async fn with_genetic(
        &mut self,
        capacity: usize,
        n_loop: usize,
        n_train: usize,
        n_break: usize,
        d_range: usize,
        shuttle_rate: f64,
        influxdb: Option<InfluxDb>,
    ) -> Result<()> {
        if self.genetic.is_none() {
            let candles = self.candles.clone().ok_or(anyhow!(
                "Not found candles, please call with_sampling first"
            ))?;

            self.genetic = Some(Arc::new(Mutex::new(Genetic::new(
                capacity,
                Arc::new(RwLock::new(Spot::new(
                    Arc::new(RwLock::new(Data::new(Arc::new(candles), d_range))),
                    self.money.ok_or(anyhow!("Not found money"))?,
                    self.stock.unwrap_or(0.0),
                    30,
                    self.stock_holding_period,
                    self.minimum_stock_buy,
                )?)),
                influxdb,
            ))));
        }

        let mut genetic = self
            .genetic
            .as_ref()
            .unwrap()
            .lock()
            .map_err(|error| anyhow!("Failed to lock genetic: {}", error))?;
        let mut step_cnt = 0;
        let mut breaking_cnt = 0;
        let mut previous_p55 = 0.0;
        let mut previous_diff_p55 = 0.0;

        if self.session == 0 {
            genetic.initialize(capacity, self.session, Some(shuttle_rate))?;
        }

        for _ in 0..n_loop {
            for i in 0..n_train {
                genetic.evolute(capacity / 5, self.session + (i + 1) as i64, self.pmutation)?;

                let stats = genetic.statistic(self.session + (i + 1) as i64).await?;
                let current_p55 = stats.p55;
                let current_diff_p55 = current_p55 - previous_p55;

                if current_p55 <= previous_p55 {
                    breaking_cnt += 1;
                } else if current_diff_p55 <= previous_diff_p55 {
                    breaking_cnt += 1;
                } else {
                    breaking_cnt = 0;
                }

                if breaking_cnt > n_break {
                    break;
                }

                step_cnt += 1;
                previous_p55 = current_p55;
                previous_diff_p55 = current_diff_p55;

                if i + 1 < n_loop {
                    genetic.fluctuate(
                        self.session + (i + 1) as i64,
                        &self.arguments,
                        self.pmutation,
                    )?;
                }
            }
            genetic.optimize()?;
            genetic.initialize(capacity, self.session, Some(shuttle_rate))?;

            if step_cnt < n_train {
                step_cnt = 0;
                breaking_cnt = 0;
                previous_p55 = 0.0;
                previous_diff_p55 = 0.0;
            } else {
                break;
            }
        }

        self.session += n_train as i64;
        Ok(())
    }
}

async fn simulate_single_symbol_with_trend_following(
    kind: &str,
    symbol: &str,
    resolution: &str,
    from: i64,
    to: i64,
) -> std::io::Result<Simulator> {
    let mut sim = Simulator::new();

    let mut infisical_client = InfiscalClient::builder().build().await.map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Fail to build infisical client: {:?}", error),
        )
    })?;

    infisical_client
        .login(AuthMethod::new_universal_auth(
            std::env::var("INFISICAL_CLIENT_ID")
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_ID"))?,
            std::env::var("INFISICAL_CLIENT_SECRET").map_err(|_| {
                Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_SECRET")
            })?,
        ))
        .await
        .map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Fail to login to infisical: {:?}", error),
            )
        })?;

    let provider = get_secret_from_infisical(&infisical_client, "PROVIDER", "/simulator/").await?;
    let number_of_genetic_routes =
        get_secret_from_infisical(&infisical_client, "NUMBER_OF_GENETIC_ROUTES", "/simulator/")
            .await?
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid NUMBER_OF_GENETIC_ROUTES"))?;
    let number_of_investors =
        get_secret_from_infisical(&infisical_client, "NUMBER_OF_INVESTORS", "/simulator/")
            .await?
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid NUMBER_OF_INVESTORS"))?;

    let number_of_loop_per_route =
        get_secret_from_infisical(&infisical_client, "NUMBER_OF_LOOP_PER_ROUTE", "/simulator/")
            .await?
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid NUMBER_OF_LOOP_PER_ROUTE"))?;

    let number_of_evolution =
        get_secret_from_infisical(&infisical_client, "NUMBER_OF_EVOLUTION", "/simulator/")
            .await?
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid NUMBER_OF_EVOLUTION"))?;

    let number_of_falsitive_broken_per_route = get_secret_from_infisical(
        &infisical_client,
        "NUMBER_OF_FALSITIVE_BROKEN_PER_ROUTE",
        "/simulator/",
    )
    .await?
    .parse::<usize>()
    .map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "Invalid NUMBER_OF_FALSITIVE_BROKEN_PER_ROUTE",
        )
    })?;

    let number_of_falsitive_broken_per_route = get_secret_from_infisical(
        &infisical_client,
        "NUMBER_OF_FALSITIVE_BROKEN_PER_ROUTE",
        "/simulator/",
    )
    .await?
    .parse::<usize>()
    .map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "Invalid NUMBER_OF_FALSITIVE_BROKEN_PER_ROUTE",
        )
    })?;

    let passing_route_percent =
        get_secret_from_infisical(&infisical_client, "PASSING_ROUTE_PERCENT", "/simulator/")
            .await?
            .parse::<f64>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid PASSING_ROUTE_PERCENT"))?;

    let window = get_secret_from_infisical(
        &infisical_client,
        "WINDOW",
        format!("/simulator/{}/", kind).as_str(),
    )
    .await?
    .parse::<usize>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid WINDOW"))?;

    sim.with_money(
        get_secret_from_infisical(
            &infisical_client,
            "MONEY",
            format!("/simulator/{}/", kind).as_str(),
        )
        .await?
        .parse::<f64>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MONEY"))?,
    );
    sim.with_stock(
        get_secret_from_infisical(
            &infisical_client,
            "STOCK",
            format!("/simulator/{}/", kind).as_str(),
        )
        .await?
        .parse::<f64>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid STOCK"))?,
    );
    sim.with_minimum_stock_buy(
        get_secret_from_infisical(
            &infisical_client,
            "MINIMUM_STOCK_FOR_BUY",
            format!("/simulator/{}/", kind).as_str(),
        )
        .await?
        .parse::<usize>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MINIMUM_STOCK_FOR_BUY"))?,
    );
    sim.with_stock_holding_period(
        get_secret_from_infisical(
            &infisical_client,
            "STOCK_HOLDING_PERIOD",
            format!("/simulator/{}/", kind).as_str(),
        )
        .await?
        .parse::<usize>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid STOCK_HOLDING_PERIOD"))?,
    );

    sim.with_sampling(provider.as_str(), kind, symbol, resolution, from, to)
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;

    for _ in 0..number_of_genetic_routes {
        sim.with_genetic(
            number_of_investors,
            number_of_loop_per_route,
            number_of_evolution,
            number_of_falsitive_broken_per_route,
            window,
            passing_route_percent,
            None,
        )
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;
    }
    Ok(sim)
}

pub async fn run() -> std::io::Result<()> {
    // @TODO: implement flow to cross validate with another stock or another timeline
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;
    let three_years_ago = now - (5 * 365 * 24 * 3600); // Approximate, ignoring leap seconds

    simulate_single_symbol_with_trend_following("stock", "MWG", "1D", three_years_ago, now).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn test_simulate_single_symbol_with_trend_following() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let three_years_ago = now - (3 * 365 * 24 * 3600); // Approximate, ignoring leap seconds

        simulate_single_symbol_with_trend_following("stock", "MWG", "1D", three_years_ago, now)
            .await
            .unwrap();
    }
}
