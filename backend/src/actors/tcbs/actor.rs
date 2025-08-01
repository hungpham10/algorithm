use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::error;
use prometheus::IntCounterVec;

use reqwest_middleware::{ClientBuilder, ClientWithMiddleware as HttpClient};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use futures::future;
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;

use actix::prelude::*;
use actix::Addr;

use crate::actors::list_futures;
use crate::actors::{
    ActorError, FlushVariablesCommand, GetVariableCommand, HealthCommand, UpdateStocksCommand,
};
use crate::algorithm::fuzzy::Variables;

#[cfg(not(feature = "python"))]
use super::monitor::monitor_order_flow;

pub struct TcbsActor {
    stocks: Vec<String>,
    futures: HashSet<String>,
    token: String,
    timeout: u64,
    page_size: usize,
    variables: Arc<Mutex<Variables>>,
}

impl TcbsActor {
    pub async fn new(stocks: &[String], token: String, variables: Arc<Mutex<Variables>>) -> Self {
        for symbol in stocks {
            let vars_to_create = Self::list_of_variables(symbol);

            for var in &vars_to_create {
                match variables.lock() {
                    Ok(mut vars) => {
                        vars.scope(&symbol, &vars_to_create);

                        if let Err(err) = vars.create(var) {
                            error!("Failed to create variable {}: {}", var, err);
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Failed to create variable {}: {}", var, err);
                        break;
                    }
                }
            }
        }

        Self {
            stocks: stocks.to_owned(),
            futures: list_futures().await.into_iter().collect(),
            timeout: 10,
            page_size: 100,
            token,
            variables,
        }
    }

    fn list_of_variables(symbol: &str) -> Vec<String> {
        vec![
            format!("{}.price", symbol),
            format!("{}.volume", symbol),
            format!("{}.type", symbol),
            format!("{}.ba", symbol),
            format!("{}.sa", symbol),
            format!("{}.cp", symbol),
            format!("{}.time", symbol),
        ]
    }
}

impl Actor for TcbsActor {
    type Context = Context<Self>;
}

impl Handler<HealthCommand> for TcbsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

impl Handler<UpdateStocksCommand> for TcbsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, msg: UpdateStocksCommand, _: &mut Self::Context) -> Self::Result {
        let stocks = msg.stocks.clone();

        match self.variables.lock() {
            Ok(mut vars) => {
                vars.clear_all();
            }
            Err(err) => {
                error!("Failed to clear variables: {}", err);
                return Box::pin(async move { false });
            }
        }

        for symbol in stocks {
            let vars_to_create = [
                format!("{}.price", symbol),
                format!("{}.volume", symbol),
                format!("{}.type", symbol),
                format!("{}.ba", symbol),
                format!("{}.sa", symbol),
            ];

            for var in &vars_to_create {
                match self.variables.lock() {
                    Ok(mut vars) => {
                        vars.scope(&symbol, &vars_to_create);

                        if let Err(err) = vars.create(var) {
                            error!("Failed to create variable {}: {}", var, err);
                            return Box::pin(async move { false });
                        }
                    }
                    Err(err) => {
                        error!("Failed to create variable {}: {}", var, err);
                        return Box::pin(async move { false });
                    }
                }
            }
        }

        self.stocks = msg.stocks.clone();
        Box::pin(async move { true })
    }
}

impl Handler<FlushVariablesCommand> for TcbsActor {
    type Result = ResponseFuture<Result<(), ActorError>>;

    fn handle(&mut self, _: FlushVariablesCommand, _: &mut Self::Context) -> Self::Result {
        let variables = self.variables.clone();
        let stocks = self.stocks.clone();

        Box::pin(async move {
            match variables.lock() {
                Ok(mut vars) => {
                    let mut error = None;

                    for sym in stocks {
                        if let Err(error_while_flush) = vars.flush(sym.as_str()).await {
                            error = Some(ActorError {
                                message: error_while_flush.message,
                            });
                        }

                        if error.is_some() {
                            break;
                        }
                    }

                    if let Some(error) = error {
                        Err(error)
                    } else {
                        Ok(())
                    }
                }
                Err(error) => Err(ActorError {
                    message: format!("{}", error),
                }),
            }
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject))]
pub struct Order {
    pub p: f64,  // price
    pub v: u64,  // volume
    pub cp: f64, // nghi ngờ là khối lượng dư khớp
    pub rcp: f64,
    pub a: String,
    pub ba: f64,  // nghi ngờ là mã định danh id của bên mua chủ động
    pub sa: f64,  // nghi ngờ là mã định danh id của bên bán chủ động
    pub hl: bool, // cờ này khá quái lạ, có khả năng liên quan đến việc mua bán chủ động
    pub pcp: f64, // diff pricing between current and previous order, cái này có thể dùng để
    // theo dõi điểm di chuyển của giá, cái này ta chỉ nên dùng để tham khảo cung
    // cầu khi mua bán
    pub t: String, // time
}

impl Order {
    #[cfg(feature = "python")]
    pub fn to_pytuple(&self, py: Python) -> Vec<Py<PyAny>> {
        vec![
            self.p.into_py(py),
            self.v.into_py(py),
            self.cp.into_py(py),
            self.rcp.into_py(py),
            self.a.clone().into_py(py),
            self.ba.into_py(py),
            self.sa.into_py(py),
            self.hl.into_py(py),
            self.pcp.into_py(py),
            self.t.clone().into_py(py),
        ]
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponse {
    pub page: u64,
    pub size: u64,
    pub headIndex: i64,
    pub numberOfItems: u64,
    pub total: u64,
    pub ticker: String,
    pub data: Vec<Order>,
    pub d: Option<String>,
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<OrderResponse>")]
pub struct GetOrderCommand {
    pub page: usize,
}

impl Handler<GetOrderCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<OrderResponse>>;

    fn handle(&mut self, msg: GetOrderCommand, _: &mut Self::Context) -> Self::Result {
        let stocks = self.stocks.clone();
        let futures = self.futures.clone();
        let timeout = self.timeout;
        let page_size = self.page_size;

        Box::pin(async move { fetch_orders(&stocks, &futures, timeout, msg.page, page_size).await })
    }
}

async fn fetch_orders(
    stocks: &[String],
    futures: &HashSet<String>,
    timeout: u64,
    page: usize,
    page_size: usize,
) -> Vec<OrderResponse> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(7);
    let client = Arc::new(
        ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build(),
    );

    future::try_join_all(stocks.iter().map(move |stock| {
        fetch_order_per_stock(client.clone(), stock, futures, timeout, page, page_size)
    }))
    .await
    .unwrap()
    .into_iter()
    .collect::<Vec<_>>()
}

async fn fetch_order_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    futures: &HashSet<String>,
    timeout: u64,
    page: usize,
    page_size: usize,
) -> Result<OrderResponse, ActorError> {
    let kind = if futures.contains(stock) {
        "futures"
    } else {
        "stock"
    };

    let resp = client.get(format!(
            "https://apipubaws.tcbs.com.vn/{}-insight/v1/intraday/{}/his/paging?page={}&size={}&headIndex={}",
            kind,
            stock,
            page, page_size,
            -1,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<OrderResponse>().await {
            Ok(resp) => Ok(OrderResponse {
                page: resp.page,
                size: resp.size,
                headIndex: resp.headIndex,
                numberOfItems: resp.numberOfItems,
                total: resp.total,
                ticker: resp.ticker,
                data: resp.data.iter().rev().map(|d| d.clone()).collect(),
                d: resp.d,
            }),
            Err(err) => Err(ActorError {
                message: format!("{:?}", err),
            }),
        },
        Err(err) => Err(ActorError {
            message: format!("{:?}", err),
        }),
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<BalanceSheet>")]
pub struct GetBalanceSheetCommand {
    stock: String,
}

impl Handler<GetBalanceSheetCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<BalanceSheet>>;

    fn handle(&mut self, msg: GetBalanceSheetCommand, _: &mut Self::Context) -> Self::Result {
        let stock = msg.stock.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
            let client = Arc::new(
                ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
                    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                    .build(),
            );

            fetch_balance_sheet_per_stock(client, &stock, timeout)
                .await
                .unwrap()
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BalanceSheet {
    quarter: u8,
    year: u16,
    shortAsset: Option<i32>,
    cash: Option<i32>,
    shortInvest: Option<i32>,
    shortReceivable: Option<i32>,
    inventory: Option<i32>,
    longAsset: Option<i32>,
    fixedAsset: Option<i32>,
    asset: Option<i32>,
    debt: Option<i32>,
    shortDebt: Option<i32>,
    longDebt: Option<i32>,
    equity: Option<i32>,
    capital: Option<i32>,
    centralBankDeposit: Option<i32>,
    otherBankDeposit: Option<i32>,
    otherBankLoan: Option<i32>,
    stockInvest: Option<i32>,
    customerLoan: Option<i32>,
    badLoan: Option<i32>,
    provision: Option<i32>,
    netCustomerLoan: Option<i32>,
    otherAsset: Option<i32>,
    otherBankCredit: Option<i32>,
    oweOtherBank: Option<i32>,
    oweCentralBank: Option<i32>,
    valuablePaper: Option<i32>,
    payableInterest: Option<i32>,
    receivableInterest: Option<i32>,
    deposit: Option<i32>,
    otherDebt: Option<i32>,
    fund: Option<i32>,
    unDistributedIncome: Option<i32>,
    minorShareHolderProfit: Option<i32>,
    payable: Option<i32>,
}

async fn fetch_balance_sheet_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    timeout: u64,
) -> Result<Vec<BalanceSheet>, ActorError> {
    let resp = client.get(format!(
            "https://apipubaws.tcbs.com.vn/tcanalysis/v1/finance/{}/balancesheet?yearly=0&isAll=true",
            stock,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<BalanceSheet>>().await {
            Ok(resp) => Ok(resp),
            Err(err) => Err(ActorError {
                message: format!("{:?}", err),
            }),
        },
        Err(err) => Err(ActorError {
            message: format!("{:?}", err),
        }),
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<IncomeStatement>")]
pub struct GetIncomeStatementCommand {
    stock: String,
}

impl Handler<GetIncomeStatementCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<IncomeStatement>>;

    fn handle(&mut self, msg: GetIncomeStatementCommand, _: &mut Self::Context) -> Self::Result {
        let stock = msg.stock.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
            let client = Arc::new(
                ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
                    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                    .build(),
            );

            fetch_income_statement_per_stock(client, &stock, timeout)
                .await
                .unwrap()
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IncomeStatement {
    quarter: u8,
    year: u16,
    revenue: Option<i32>,
    yearRevenueGrowth: Option<f64>,
    quarterRevenueGrowth: Option<f64>,
    costOfGoodSold: Option<i32>,
    grossProfit: Option<i32>,
    operationExpense: Option<i32>,
    operationProfit: Option<i32>,
    yearOperationProfitGrowth: Option<f64>,
    quarterOperationProfitGrowth: Option<f64>,
    interestExpense: Option<i32>,
    preTaxProfit: Option<i32>,
    postTaxProfit: Option<i32>,
    shareHolderIncome: Option<i32>,
    yearShareHolderIncomeGrowth: Option<f64>,
    quarterShareHolderIncomeGrowth: Option<f64>,
    investProfit: Option<i32>,
    serviceProfit: Option<i32>,
    otherProfit: Option<i32>,
    provisionExpense: Option<i32>,
    operationIncome: Option<i32>,
    ebitda: Option<i32>,
}

async fn fetch_income_statement_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    timeout: u64,
) -> Result<Vec<IncomeStatement>, ActorError> {
    let resp = client.get(format!(
            "https://apipubaws.tcbs.com.vn/tcanalysis/v1/finance/{}/incomestatement?yearly=0&isAll=true",
            stock,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<IncomeStatement>>().await {
            Ok(resp) => Ok(resp),
            Err(err) => Err(ActorError {
                message: format!("{:?}", err),
            }),
        },
        Err(err) => Err(ActorError {
            message: format!("{:?}", err),
        }),
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<CashFlow>")]
pub struct GetCashFlowCommand {
    stock: String,
}

impl Handler<GetCashFlowCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<CashFlow>>;

    fn handle(&mut self, msg: GetCashFlowCommand, _: &mut Self::Context) -> Self::Result {
        let stock = msg.stock.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
            let client = Arc::new(
                ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
                    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                    .build(),
            );

            fetch_cash_flow_per_stock(client, &stock, timeout)
                .await
                .unwrap()
        })
    }
}
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CashFlow {
    quarter: u8,
    year: u16,
    investCost: Option<i32>,
    fromInvest: Option<i32>,
    fromFinancial: Option<i32>,
    fromSale: Option<i32>,
    freeCashFlow: Option<i32>,
}

async fn fetch_cash_flow_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    timeout: u64,
) -> Result<Vec<CashFlow>, ActorError> {
    let resp = client
        .get(format!(
            "https://apipubaws.tcbs.com.vn/tcanalysis/v1/finance/{}/cashflow?yearly=0&isAll=true",
            stock,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<CashFlow>>().await {
            Ok(resp) => Ok(resp),
            Err(err) => Err(ActorError {
                message: format!("{:?}", err),
            }),
        },
        Err(err) => Err(ActorError {
            message: format!("{:?}", err),
        }),
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<bool, ActorError>")]
pub struct SetAlertCommand {
    stock: String,
    price: f64,
}

impl Handler<SetAlertCommand> for TcbsActor {
    type Result = ResponseFuture<Result<bool, ActorError>>;

    fn handle(&mut self, msg: SetAlertCommand, _: &mut Self::Context) -> Self::Result {
        let stock = msg.stock.clone();
        let price = msg.price;
        let token = self.token.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
            let client = Arc::new(
                ClientBuilder::new(reqwest_middleware::reqwest::Client::new())
                    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                    .build(),
            );

            set_alert(client, &token, &stock, price, timeout).await
        })
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SetAlertCondition {
    key: String,
    operator: String,
    value: f64,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SetAlertRequest {
    name: String,
    conditions: Vec<SetAlertCondition>,
    objectType: String,
    objectData: String,
    additionalInfo: Vec<String>,
    enable: bool,
    sendInbox: bool,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SetAlertResponsePayload {
    id: u64,
    name: String,
    objectType: String,
    objectData: String,
    additionalInfo: Vec<String>,
    enable: bool,
    sendInbox: bool,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct SetAlertResponse {
    status: u64,
    message: String,
    data: SetAlertResponsePayload,
}

async fn set_alert(
    client: Arc<HttpClient>,
    stock: &str,
    token: &String,
    price: f64,
    timeout: u64,
) -> Result<bool, ActorError> {
    let resp = client
        .post("https://apiextaws.tcbs.com.vn/ligo/v1/warning")
        .timeout(Duration::from_secs(timeout))
        .bearer_auth(token)
        .json(&SetAlertRequest {
            name: stock.to_owned(),
            conditions: vec![SetAlertCondition {
                key: "price".to_string(),
                operator: "<=".to_string(),
                value: price,
            }],
            objectType: "ticker".to_string(),
            objectData: stock.to_owned(),
            additionalInfo: vec![
                "rsi14".to_string(),
                "dividendYield".to_string(),
                "strongBuyPercentage".to_string(),
            ],
            enable: true,
            sendInbox: true,
        })
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<SetAlertResponse>().await {
            Ok(resp) => {
                if resp.status == 200 {
                    Ok(resp.data.enable)
                } else {
                    Err(ActorError {
                        message: format!("code {}: {:?}", resp.status, resp.message),
                    })
                }
            }
            Err(err) => Err(ActorError {
                message: format!("{:?}", err),
            }),
        },
        Err(err) => Err(ActorError {
            message: format!("{:?}", err),
        }),
    }
}

impl Handler<GetVariableCommand> for TcbsActor {
    type Result = ResponseFuture<Result<f64, ActorError>>;

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

#[derive(Message)]
#[rtype(result = "Result<usize, ActorError>")]
pub struct UpdateVariablesCommand {
    pub orders: Vec<Order>,
    pub symbol: String,
    pub counter: Arc<IntCounterVec>,
}

impl Handler<UpdateVariablesCommand> for TcbsActor {
    type Result = ResponseFuture<Result<usize, ActorError>>;

    fn handle(&mut self, msg: UpdateVariablesCommand, _: &mut Self::Context) -> Self::Result {
        let variables = self.variables.clone();

        Box::pin(async move {
            let mut updated = 0;
            match variables.lock() {
                Ok(mut vars) => {
                    #[cfg(not(feature = "python"))]
                    let counter = msg.counter.clone();

                    let vars_to_create = Self::list_of_variables(&msg.symbol);

                    for order in msg.orders {
                        let (hour, min, sec) = if let Ok(parts) = order
                            .t
                            .split(':')
                            .map(|s| s.parse::<i64>())
                            .collect::<Result<Vec<_>, _>>()
                        {
                            if parts.len() == 3 {
                                (parts[0], parts[1], parts[2])
                            } else {
                                (0, 0, 0) // Default values for invalid format
                            }
                        } else {
                            (0, 0, 0) // Default values for parse errors
                        };
                        let time = (hour * 3600 + min * 60 + sec) as f64;
                        let last = vars.last(vars_to_create[5].as_str()).unwrap_or(0.0);

                        if time <= last {
                            break;
                        }

                        #[cfg(not(feature = "python"))]
                        monitor_order_flow(&msg.symbol.to_string(), &order, counter.clone());

                        if let Err(e) = vars
                            .update(&msg.symbol, &vars_to_create[0].to_string(), order.p)
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[0], e);
                        }

                        if let Err(e) = vars
                            .update(&msg.symbol, &vars_to_create[1].to_string(), order.v as f64)
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[1], e);
                        }

                        if let Err(e) = vars
                            .update(
                                &msg.symbol,
                                &vars_to_create[2].to_string(),
                                match order.a.as_str() {
                                    "BU" => 1.0,
                                    "SD" => 0.0,
                                    _ => 0.5,
                                },
                            )
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[2], e);
                        }

                        if let Err(e) = vars
                            .update(&msg.symbol, &vars_to_create[3].to_string(), order.ba)
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[3], e);
                        }

                        if let Err(e) = vars
                            .update(&msg.symbol, &vars_to_create[4].to_string(), order.sa)
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[4], e);
                        }

                        if let Err(e) = vars
                            .update(&msg.symbol, &vars_to_create[5].to_string(), order.cp)
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[5], e);
                        }

                        if let Err(e) = vars
                            .update(
                                &msg.symbol,
                                &vars_to_create[6].to_string(),
                                (hour * 3600 + min * 60 + sec) as f64,
                            )
                            .await
                        {
                            error!("Failed to update variable {}: {}", vars_to_create[6], e);
                        }

                        updated += 1;
                    }
                    Ok(updated)
                }
                Err(err) => Err(ActorError {
                    message: format!("Fail to lock variables: {}", err),
                }),
            }
        })
    }
}

pub async fn connect_to_tcbs(
    stocks: &[String],
    token: String,
    variables: Arc<Mutex<Variables>>,
) -> Addr<TcbsActor> {
    TcbsActor::new(stocks, token, variables).await.start()
}
