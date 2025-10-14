use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use infisical::{AuthMethod, Client as InfiscalClient};
use reqwest;

use vnscope::algorithm::genetic::{Genetic, InfluxDb};
use vnscope::algorithm::simulator::{Data, Investor, Spot};
use vnscope::schemas::{CandleStick, Portal, CRONJOB, WATCHLIST};

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
    lifespan: i64,
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
            lifespan: 10,
            minimum_stock_buy: 100,
            stock_holding_period: 10,
        }
    }

    pub fn with_minimum_stock_buy(&mut self, value: usize) {
        self.minimum_stock_buy = value;
    }

    pub fn with_stock_holding_period(&mut self, value: usize) {
        self.stock_holding_period = value;
    }

    pub fn with_lifespan(&mut self, lifespan: i64) {
        self.lifespan = lifespan;
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
        symbol: &str,
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
                    self.lifespan,
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

                let stats = genetic
                    .statistic(self.session + (i + 1) as i64, symbol)
                    .await?;
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

                if i + 1 < n_train {
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

async fn simulate_with_trend_following(
    infisical_client: &InfiscalClient,
    market: &str,
    symbols: Vec<String>,
    resolution: &str,
    from: i64,
    to: i64,
) -> std::io::Result<Vec<Simulator>> {
    let mut simulators = Vec::new();
    let provider = get_secret_from_infisical(&infisical_client, "PROVIDER", "/simulator/").await?;
    let influx_url =
        get_secret_from_infisical(&infisical_client, "INFLUX_URL", "/simulator/").await?;
    let influx_token =
        get_secret_from_infisical(&infisical_client, "INFLUX_TOKEN", "/simulator/").await?;
    let influx_bucket = get_secret_from_infisical(
        &infisical_client,
        "INFLUX_BUCKET",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?;

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

    let passing_route_percent =
        get_secret_from_infisical(&infisical_client, "PASSING_ROUTE_PERCENT", "/simulator/")
            .await?
            .parse::<f64>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid PASSING_ROUTE_PERCENT"))?;
    let window = get_secret_from_infisical(
        &infisical_client,
        "WINDOW",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<usize>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid WINDOW"))?;
    let money = get_secret_from_infisical(
        &infisical_client,
        "MONEY",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<f64>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MONEY"))?;
    let stock = get_secret_from_infisical(
        &infisical_client,
        "STOCK",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<f64>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid STOCK"))?;
    let minimum_stock_buy = get_secret_from_infisical(
        &infisical_client,
        "MINIMUM_STOCK_FOR_BUY",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<usize>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MINIMUM_STOCK_FOR_BUY"))?;
    let stock_holding_period = get_secret_from_infisical(
        &infisical_client,
        "STOCK_HOLDING_PERIOD",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<usize>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid STOCK_HOLDING_PERIOD"))?;
    let lifespan = get_secret_from_infisical(
        &infisical_client,
        "LIFESPAN",
        format!("/simulator/{}/", market).as_str(),
    )
    .await?
    .parse::<i64>()
    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid LIFESPAN"))?;

    for symbol in symbols {
        let mut sim = Simulator::new();

        sim.with_money(money);
        sim.with_stock(stock);
        sim.with_minimum_stock_buy(minimum_stock_buy);
        sim.with_stock_holding_period(stock_holding_period);
        sim.with_lifespan(lifespan);
        sim.with_sampling(
            provider.as_str(),
            market,
            symbol.as_str(),
            resolution,
            from,
            to,
        )
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;

        for _ in 0..number_of_genetic_routes {
            sim.with_genetic(
                symbol.as_str(),
                number_of_investors,
                number_of_loop_per_route,
                number_of_evolution,
                number_of_falsitive_broken_per_route,
                window,
                passing_route_percent,
                Some(InfluxDb::new(
                    influx_url.as_str(),
                    influx_token.as_str(),
                    influx_bucket.as_str(),
                )),
            )
            .await
            .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;
        }
        simulators.push(sim);
    }

    Ok(simulators)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Model {
    TrendFollowing,
}

impl TryFrom<String> for Model {
    type Error = Error;

    fn try_from(value: String) -> std::io::Result<Model> {
        match value.as_str() {
            "trend-following" => Ok(Self::TrendFollowing),

            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Unsupported model: {}", value),
            )),
        }
    }
}

pub async fn run(
    model: &str,
    market: &str,
    resolution: &str,
    backtest_window_year_ago: i64,
) -> std::io::Result<()> {
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
    let portal = Arc::new(Portal::new(
        get_secret_from_infisical(&infisical_client, "AIRTABLE_API_KEY", "/feature-flags/")
            .await
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_API_KEY"))?
            .as_str(),
        get_secret_from_infisical(&infisical_client, "AIRTABLE_BASE_ID", "/feature-flags/")
            .await
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_BASE_ID"))?
            .as_str(),
        &HashMap::from([
            (
                WATCHLIST.to_string(),
                get_secret_from_infisical(
                    &infisical_client,
                    "AIRTABLE_TABLE_WATCHLIST",
                    "/feature-flags/",
                )
                .await
                .unwrap_or_else(|_| WATCHLIST.to_string()),
            ),
            (
                CRONJOB.to_string(),
                get_secret_from_infisical(
                    &infisical_client,
                    "AIRTABLE_TABLE_WATCHLIST",
                    "/feature-flags/",
                )
                .await
                .unwrap_or_else(|_| CRONJOB.to_string()),
            ),
        ]),
        get_secret_from_infisical(&infisical_client, "USE_AIRTABLE", "/feature-flags/")
            .await
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid USE_AIRTABLE"))?,
    ));

    let symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
        .collect::<Vec<String>>();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;
    let backtest_years_ago = now - (backtest_window_year_ago * 365 * 24 * 3600);

    match Model::try_from(model.to_string())? {
        Model::TrendFollowing => {
            simulate_with_trend_following(
                &infisical_client,
                market,
                symbols,
                resolution,
                backtest_years_ago,
                now,
            )
            .await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn test_simulate_single_symbol_with_trend_following() {
        let mut infisical_client = InfiscalClient::builder()
            .build()
            .await
            .map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Fail to build infisical client: {:?}", error),
                )
            })
            .unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;
        let three_years_ago = now - (3 * 365 * 24 * 3600);

        infisical_client
            .login(AuthMethod::new_universal_auth(
                std::env::var("INFISICAL_CLIENT_ID")
                    .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_ID"))
                    .unwrap(),
                std::env::var("INFISICAL_CLIENT_SECRET")
                    .map_err(|_| {
                        Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_SECRET")
                    })
                    .unwrap(),
            ))
            .await
            .unwrap();

        simulate_with_trend_following(
            &infisical_client,
            "stock",
            vec!["MWG".to_string()],
            "1D",
            three_years_ago,
            now,
        )
        .await
        .unwrap();
    }
}
