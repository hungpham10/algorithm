use std::error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::future;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware as HttpClient};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use crate::actors::HealthCommand;
use crate::algorithm::Variables;

#[derive(Debug, Clone)]
pub struct TcbsError {
    pub message: String,
}

impl fmt::Display for TcbsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for TcbsError {}

pub struct TcbsActor {
    stocks: Vec<String>,
    token: String,
    timeout: u64,
    page_size: usize,
    variables: Arc<Mutex<Variables>>,
}

impl TcbsActor {
    pub fn new(stocks: &Vec<String>, token: String, variables: Arc<Mutex<Variables>>) -> Self {
        Self {
            stocks:    stocks.clone(),
            timeout:   60,
            page_size: 100,
            token,
            variables,
        }
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

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Order {
    pub p: f64,
    pub v: u64,
    pub cp: f64,
    pub rcp: f64,
    pub a: String,
    pub ba: f64,
    pub sa: f64,
    pub hl: bool,
    pub pcp: f64,
    pub t: String,
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
        let timeout = self.timeout;
        let page_size = self.page_size;

        Box::pin(async move {
            let datapoints = fetch_orders(&stocks, timeout, msg.page, page_size).await;

            return datapoints;
        })
    }
}

async fn fetch_orders(
    stocks: &Vec<String>,
    timeout: u64,
    page: usize,
    page_size: usize,
) -> Vec<OrderResponse> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
    let client = Arc::new(
        ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build(),
    );

    future::try_join_all(
        stocks.iter().map(move |stock| {
            fetch_order_per_stock(client.clone(), stock, timeout, page, page_size)
        }),
    )
    .await
    .unwrap()
    .into_iter()
    .collect::<Vec<_>>()
}

async fn fetch_order_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    timeout: u64,
    page: usize,
    page_size: usize,
) -> Result<OrderResponse, TcbsError> {
    let resp = client.get(format!(
            "https://apipubaws.tcbs.com.vn/stock-insight/v1/intraday/{}/his/paging?page={}&size={}&headIndex={}",
            stock,
            page, page_size,
            -1,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<OrderResponse>().await {
            Ok(resp) => Ok(resp),
            Err(error) => Err(TcbsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(TcbsError {
            message: format!("{:?}", error),
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
                ClientBuilder::new(reqwest::Client::new())
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
) -> Result<Vec<BalanceSheet>, TcbsError> {
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
            Err(error) => Err(TcbsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(TcbsError {
            message: format!("{:?}", error),
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
                ClientBuilder::new(reqwest::Client::new())
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
) -> Result<Vec<IncomeStatement>, TcbsError> {
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
            Err(error) => Err(TcbsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(TcbsError {
            message: format!("{:?}", error),
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
                ClientBuilder::new(reqwest::Client::new())
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
    fromInvest: Option<i32> ,
    fromFinancial: Option<i32>,
    fromSale: Option<i32>,
    freeCashFlow: Option<i32>,
}

async fn fetch_cash_flow_per_stock(
    client: Arc<HttpClient>,
    stock: &String,
    timeout: u64,
) -> Result<Vec<CashFlow>, TcbsError> {
    let resp = client.get(format!(
            "https://apipubaws.tcbs.com.vn/tcanalysis/v1/finance/{}/cashflow?yearly=0&isAll=true",
            stock,
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<CashFlow>>().await {
            Ok(resp) => Ok(resp),
            Err(error) => Err(TcbsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(TcbsError {
            message: format!("{:?}", error),
        }),
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<bool, TcbsError>")]
pub struct SetAlertCommand {
    stock: String,
    price: f64,
}

impl Handler<SetAlertCommand> for TcbsActor {
    type Result = ResponseFuture<Result<bool, TcbsError>>;

    fn handle(&mut self, msg: SetAlertCommand, _: &mut Self::Context) -> Self::Result {
        let stock = msg.stock.clone();
        let price = msg.price;
        let token = self.token.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(100);
            let client = Arc::new(
                ClientBuilder::new(reqwest::Client::new())
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
    stock: &String, 
    token: &String,
    price: f64,
    timeout: u64,
) -> Result<bool, TcbsError> {
    let resp = client.post(
            "https://apiextaws.tcbs.com.vn/ligo/v1/warning"
        )
        .timeout(Duration::from_secs(timeout))
        .bearer_auth(token)
        .json(&SetAlertRequest {
            name: stock.clone(),
            conditions: vec![SetAlertCondition {
                key: "price".to_string(),
                operator: "<=".to_string(),
                value: price,
            }],
            objectType: "ticker".to_string(),
            objectData: stock.clone(),
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
                    Err(TcbsError {
                        message: format!("code {}: {:?}", resp.status, resp.message),
                    })
                }
            }
            Err(error) => Err(TcbsError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(TcbsError {
            message: format!("{:?}", error),
        }),
    }
}


#[derive(Message)]
#[rtype(result = "Result<bool, TcbsError>")]
pub struct UpdateVariablesCommand {
    pub orders: Vec<Order>,
    pub symbol: String,
}


impl Handler<UpdateVariablesCommand> for TcbsActor {
    type Result = ResponseFuture<Result<bool, TcbsError>>;

    fn handle(&mut self, _msg: UpdateVariablesCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            Ok(false)
        })
    }
}

pub fn connect_to_tcbs(stocks: &Vec<String>, token: String) -> Addr<TcbsActor> {
    TcbsActor::new(stocks, token, Arc::new(Mutex::new(Variables::new(0))))
        .start()
}

