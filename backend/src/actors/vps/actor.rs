use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::future;
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;

use reqwest_middleware::{ClientBuilder, ClientWithMiddleware as HttpClient};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use actix::prelude::*;
use actix::Addr;

use crate::actors::{ActorError, GetVariableCommand, HealthCommand, UpdateStocksCommand};
use crate::algorithm::Variables;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject))]
pub struct Price {
    pub id: i64,
    pub sym: String,
    pub mc: String,
    pub c: f64,
    pub f: f64,
    pub r: f64,
    pub lastPrice: f64,
    pub lastVolume: f64,
    pub lot: u64,
    pub ot: String,
    pub changePc: String,
    pub avePrice: String,
    pub highPrice: String,
    pub lowPrice: String,
    pub fBVol: String,
    pub fBValue: String,
    pub fSVolume: String,
    pub fSValue: String,
    pub fRoom: String,
    pub g1: String,
    pub g2: String,
    pub g3: String,
    pub g4: String,
    pub g5: String,
    pub g6: String,
    pub g7: String,
    pub CWUnderlying: String,
    pub CWType: String,
    pub CWLastTradingDate: String,
    pub CWExcersisePrice: String,
    pub CWExerciseRatio: String,
    pub CWListedShare: String,
    pub sType: String,
    pub sBenefit: String,
}

impl Price {
    #[cfg(feature = "python")]
    pub fn to_pytuple(&self, py: Python) -> Vec<Py<PyAny>> {
        let g1 = self.g1.split("|").collect::<Vec<&str>>();
        let g2 = self.g2.split("|").collect::<Vec<&str>>();
        let g3 = self.g3.split("|").collect::<Vec<&str>>();
        let g4 = self.g4.split("|").collect::<Vec<&str>>();
        let g5 = self.g5.split("|").collect::<Vec<&str>>();
        let g6 = self.g6.split("|").collect::<Vec<&str>>();

        vec![
            // Basic infomation
            self.sym.clone().into_py(py),
            self.lastPrice.into_py(py),
            self.lastVolume.into_py(py),
            self.lot.into_py(py),
            self.changePc.parse::<f64>().unwrap_or(0.0).into_py(py),
            // Order book data
            g4[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g4[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            g5[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g5[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            g6[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g6[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            g1[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g1[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            g2[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g2[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            g3[0].parse::<f64>().unwrap_or(0.0).into_py(py),
            g3[1].parse::<f64>().unwrap_or(0.0).into_py(py),
            // Foreign flow
            self.fBVol.parse::<f64>().unwrap_or(0.0).into_py(py),
            self.fSVolume.parse::<f64>().unwrap_or(0.0).into_py(py),
        ]
    }
}
#[derive(Debug, Clone)]
pub struct VpsError {
    pub message: String,
}

impl fmt::Display for VpsError {
    /// Formats the error message for display.
    ///
    /// This method writes the contained error message to the given formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for VpsError {}

pub struct VpsActor {
    variables: Arc<Mutex<Variables>>,
    stocks: Vec<String>,
    timeout: u64,
}

impl VpsActor {
    pub fn new(stocks: &[String], variables: Arc<Mutex<Variables>>) -> Self {
        Self {
            stocks: stocks.to_owned(),
            timeout: 300,
            variables,
        }
    }
}

impl Actor for VpsActor {
    type Context = Context<Self>;
}

impl Handler<HealthCommand> for VpsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

impl Handler<UpdateStocksCommand> for VpsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, msg: UpdateStocksCommand, _: &mut Self::Context) -> Self::Result {
        self.stocks = msg.stocks.clone();

        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<Price>")]
pub struct GetPriceCommand;

impl Handler<GetPriceCommand> for VpsActor {
    type Result = ResponseFuture<Vec<Price>>;

    fn handle(&mut self, _msg: GetPriceCommand, _: &mut Self::Context) -> Self::Result {
        let stocks = self.stocks.clone();
        let timeout = self.timeout;

        Box::pin(async move { fetch_price_depth(&stocks, timeout).await })
    }
}

async fn fetch_price_depth(stocks: &Vec<String>, timeout: u64) -> Vec<Price> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
    let client = Arc::new(
        ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build(),
    );
    let blocks: Vec<Vec<String>> = (*stocks)
        .chunks(100)
        .map(|block| block.iter().map(|stock| (*stock).clone()).collect())
        .collect();

    future::try_join_all(
        blocks
            .iter()
            .map(move |block| fetch_price_depth_per_block(client.clone(), block, timeout)),
    )
    .await
    .unwrap()
    .into_iter()
    .flatten()
    .collect()
}

async fn fetch_price_depth_per_block(
    client: Arc<HttpClient>,
    block: &Vec<String>,
    timeout: u64,
) -> Result<Vec<Price>, VpsError> {
    let resp = client
        .get(format!(
            "https://bgapidatafeed.vps.com.vn/getliststockdata/{}",
            (*block).join(","),
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<Price>>().await {
            Ok(resp) => Ok(resp),
            Err(error) => Err(VpsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(VpsError {
            message: format!("{:?}", error),
        }),
    }
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

#[derive(Message)]
#[rtype(result = "Result<HashMap<String, usize>, VpsError>")]
pub struct UpdateVariablesCommand {
    pub prices: Vec<Price>,
}

impl Handler<UpdateVariablesCommand> for VpsActor {
    type Result = ResponseFuture<Result<HashMap<String, usize>, VpsError>>;

    /// Updates shared variables with the latest stock price and order book data.
    ///
    /// For each provided `Price`, this function creates and updates variables representing
    /// the current price, volume, change percent, price and volume levels, and foreign buy/sell volumes.
    /// Returns a map of variable names to their updated counts or lengths.
    ///
    /// # Returns
    ///
    /// A `Result` containing a map from variable names to their updated counts, or a `VpsError` if an error occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `actor` is a VpsActor and `prices` is a Vec<Price>
    /// let cmd = UpdateVariablesCommand { prices };
    /// let result = actor.handle(cmd, &mut ctx).await;
    /// assert!(result.is_ok());
    /// ```
    fn handle(&mut self, msg: UpdateVariablesCommand, _: &mut Self::Context) -> Self::Result {
        let variables = self.variables.clone();

        Box::pin(async move {
            let mut updates = HashMap::new();
            let mut vars = variables.lock().unwrap();

            for price in msg.prices {
                // Split order book data
                let g1 = price.g1.split("|").collect::<Vec<&str>>();
                let g2 = price.g2.split("|").collect::<Vec<&str>>();
                let g3 = price.g3.split("|").collect::<Vec<&str>>();
                let g4 = price.g4.split("|").collect::<Vec<&str>>();
                let g5 = price.g5.split("|").collect::<Vec<&str>>();
                let g6 = price.g6.split("|").collect::<Vec<&str>>();

                // Create variable names
                let vars_to_create = [
                    format!("{}.price", price.sym),
                    format!("{}.volume", price.sym),
                    format!("{}.change", price.sym),
                    // Price levels
                    format!("{}.price_minus1", price.sym),
                    format!("{}.price_minus2", price.sym),
                    format!("{}.price_minus3", price.sym),
                    format!("{}.price_plus1", price.sym),
                    format!("{}.price_plus2", price.sym),
                    format!("{}.price_plus3", price.sym),
                    // Volume levels
                    format!("{}.volume_minus1", price.sym),
                    format!("{}.volume_minus2", price.sym),
                    format!("{}.volume_minus3", price.sym),
                    format!("{}.volume_plus1", price.sym),
                    format!("{}.volume_plus2", price.sym),
                    format!("{}.volume_plus3", price.sym),
                    // Foreign flow
                    format!("{}.fb_buy_volume", price.sym),
                    format!("{}.fb_sell_volume", price.sym),
                ];

                // Create variables
                for var in &vars_to_create {
                    if let Err(e) = vars.create(var) {
                        log::error!("Failed to create variable {}: {}", var, e);
                    }
                }

                // Update current price and volume
                let current_price = if price.lastPrice == 0.0 {
                    price.r
                } else {
                    price.lastPrice
                };
                if let Ok(len) = vars
                    .update(&format!("{}_price", price.sym), current_price)
                    .await
                {
                    updates.insert(format!("{}_price", price.sym), len);
                }
                if let Ok(len) = vars
                    .update(&format!("{}_volume", price.sym), price.lot as f64)
                    .await
                {
                    updates.insert(format!("{}_volume", price.sym), len);
                }

                // Update change percent
                let change_percent = if price.r < price.lastPrice {
                    price.changePc.parse::<f64>().unwrap_or(0.0)
                } else {
                    -1.0 * price.changePc.parse::<f64>().unwrap_or(0.0)
                };
                if let Ok(len) = vars
                    .update(&format!("{}_change", price.sym), change_percent)
                    .await
                {
                    updates.insert(format!("{}_change", price.sym), len);
                }

                // Update price levels
                let price_updates = [
                    (format!("{}.price_minus1", price.sym), g4[0]),
                    (format!("{}.price_minus2", price.sym), g5[0]),
                    (format!("{}.price_minus3", price.sym), g6[0]),
                    (format!("{}.price_plus1", price.sym), g1[0]),
                    (format!("{}.price_plus2", price.sym), g2[0]),
                    (format!("{}.price_plus3", price.sym), g3[0]),
                ];

                // Update volume levels
                let volume_updates = [
                    (format!("{}.volume_minus1", price.sym), g4[1]),
                    (format!("{}.volume_minus2", price.sym), g5[1]),
                    (format!("{}.volume_minus3", price.sym), g6[1]),
                    (format!("{}.volume_plus1", price.sym), g1[1]),
                    (format!("{}.volume_plus2", price.sym), g2[1]),
                    (format!("{}.volume_plus3", price.sym), g3[1]),
                ];

                // Update all price levels
                for (var, val) in &price_updates {
                    if let Ok(len) = vars.update(var, val.parse::<f64>().unwrap_or(0.0)).await {
                        updates.insert(var.clone(), len);
                    }
                }

                // Update all volume levels
                for (var, val) in &volume_updates {
                    if let Ok(len) = vars.update(var, val.parse::<f64>().unwrap_or(0.0)).await {
                        updates.insert(var.clone(), len);
                    }
                }

                // Update foreign flow
                if let Ok(len) = vars
                    .update(
                        &format!("{}.fb_buy_volume", price.sym),
                        price.fBVol.parse::<f64>().unwrap_or(0.0),
                    )
                    .await
                {
                    updates.insert(format!("{}.fb_buy_volume", price.sym), len);
                }
                if let Ok(len) = vars
                    .update(
                        &format!("{}.fb_sell_volume", price.sym),
                        price.fSVolume.parse::<f64>().unwrap_or(0.0),
                    )
                    .await
                {
                    updates.insert(format!("{}_fb_sell_volume", price.sym), len);
                }
            }

            Ok(updates)
        })
    }
}

impl Handler<GetVariableCommand> for VpsActor {
    type Result = ResponseFuture<Result<f64, ActorError>>;

    /// Retrieves the value of a specific variable for a given stock symbol asynchronously.
    ///
    /// Returns the variable value as an `f64` if found, or an `ActorError` if the variable does not exist or the lock cannot be acquired.
    fn handle(&mut self, msg: GetVariableCommand, _: &mut Self::Context) -> Self::Result {
        let variables = self.variables.clone();

        Box::pin(async move {
            let vars = variables.lock().map_err(|e| ActorError {
                message: format!("Failed to acquire lock: {}", e),
            })?;
            let var_name = format!("{}.{}", msg.symbol, msg.variable);

            vars.get_by_expr(&var_name).map_err(|e| ActorError {
                message: format!("Failed to get variable {}: {}", var_name, e),
            })
        })
    }
}

pub fn connect_to_vps(stocks: &[String]) -> Addr<VpsActor> {
    VpsActor::new(stocks, Arc::new(Mutex::new(Variables::new(0, 0)))).start()
}
