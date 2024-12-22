use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::future;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware as HttpClient};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sentry::capture_error;
use serde::{Deserialize, Serialize};

use chrono::{NaiveTime, Utc};
use chrono_tz::Asia::Ho_Chi_Minh;

use actix::prelude::*;
use actix::Addr;
use diesel::prelude::*;

use gluesql::core::store::DataRow;
use gluesql::core::ast::ColumnDef;
use gluesql::core::data::Schema;
use gluesql::prelude::DataType;
use gluesql::prelude::Key;
use gluesql::prelude::Value;

use crate::actors::cron::CronResolver;
use crate::helpers::PgPool;

#[derive(Debug, Clone)]
pub struct TcbsError {
    message: String,
}

impl fmt::Display for TcbsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for TcbsError {}

pub struct TcbsActor {
    stocks: Vec<String>,
    timeout: u64,
    page_size: usize,
}

impl TcbsActor {
    fn new(stocks: Vec<String>) -> Self {
        Self {
            stocks,
            timeout: 60,
            page_size: 100,
        }
    }
}

impl Actor for TcbsActor {
    type Context = Context<Self>;
}

impl Handler<super::HealthCommand> for TcbsActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
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
    page: usize,
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

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct BalanceSheet {
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

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct IncomeStatement {
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

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CashFlow {
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

impl Handler<super::ListSchemaCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<Schema>>;

    fn handle(&mut self, msg: super::ListSchemaCommand, _: &mut Self::Context) -> Self::Result {
        // @TODO: hien thi tat ca schema
        Box::pin(async move {
            let mut result: Vec<Schema> = Vec::<Schema>::new();
            let balance_sheet_column_defs: Vec<ColumnDef> = vec![
                ColumnDef {
                    name: "session".to_string(),
                    data_type: DataType::Text,
                    unique: None,
                    default: None,
                    nullable: false,  // Should be false as quarter is u8, not Option<u8>
                    comment: None,
                },
                ColumnDef {
                    name: "shortAsset".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "cash".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "shortInvest".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "shortReceivable".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "inventory".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "longAsset".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "fixedAsset".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "asset".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "debt".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "shortDebt".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "longDebt".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "equity".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "capital".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "centralBankDeposit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherBankDeposit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherBankLoan".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "stockInvest".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "customerLoan".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "badLoan".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "provision".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "netCustomerLoan".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherAsset".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherBankCredit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "oweOtherBank".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "oweCentralBank".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "valuablePaper".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "payableInterest".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "receivableInterest".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "deposit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherDebt".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "fund".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "unDistributedIncome".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "minorShareHolderProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "payable".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
            ];
            let income_statement_column_defs: Vec<ColumnDef> = vec![
                ColumnDef {
                    name: "session".to_string(),
                    data_type: DataType::Text,
                    unique: None,
                    default: None,
                    nullable: false,
                    comment: None,
                },
                ColumnDef {
                    name: "revenue".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "yearRevenueGrowth".to_string(),
                    data_type: DataType::Decimal, // Use Decimal to match f64 type
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "quarterRevenueGrowth".to_string(),
                    data_type: DataType::Decimal,  // Use Decimal
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "costOfGoodSold".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "grossProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "operationExpense".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "operationProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "yearOperationProfitGrowth".to_string(),
                    data_type: DataType::Decimal,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "quarterOperationProfitGrowth".to_string(),
                    data_type: DataType::Decimal,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "interestExpense".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "preTaxProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "postTaxProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "shareHolderIncome".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "yearShareHolderIncomeGrowth".to_string(),
                    data_type: DataType::Decimal,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "quarterShareHolderIncomeGrowth".to_string(),
                    data_type: DataType::Decimal,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "investProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "serviceProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "otherProfit".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "provisionExpense".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "operationIncome".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "ebitda".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
            ];
            let cash_flow_column_defs = vec![
                ColumnDef {
                    name: "session".to_string(),
                    data_type: DataType::Text,
                    unique: None,
                    default: None,
                    nullable: false,
                    comment: None,
                },
                ColumnDef {
                    name: "investCost".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "fromInvest".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "fromFinancial".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "fromSale".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
                ColumnDef {
                    name: "freeCashFlow".to_string(),
                    data_type: DataType::Int32,
                    unique: None,
                    default: None,
                    nullable: true,
                    comment: None,
                },
            ];


            let stocks = super::vps::list_active_stocks().await;
            for stock_name in &stocks {
                result.push(Schema {
                    table_name: format!("balance_sheet_{}", stock_name),
                    column_defs: Some(balance_sheet_column_defs.clone()),
                    indexes: Vec::new(),
                    engine: None,
                    foreign_keys: Vec::new(),
                    comment: None,
                });
            }

            for stock_name in &stocks {
                result.push(Schema {
                    table_name: format!("cash_flow_{}", stock_name),
                    column_defs: Some(cash_flow_column_defs.clone()),
                    indexes: Vec::new(),
                    engine: None,
                    foreign_keys: Vec::new(),
                    comment: None,
                });
            }

            for stock_name in &stocks {
                result.push(Schema {
                    table_name: format!("income_statement_{}", stock_name),
                    column_defs: Some(income_statement_column_defs.clone()),
                    indexes: Vec::new(),
                    engine: None,
                    foreign_keys: Vec::new(),
                    comment: None,
                });
            }

            result
        }) 
    }
}

impl Handler<super::ScanDataCommand> for TcbsActor {
    type Result = ResponseFuture<BTreeMap<Key, DataRow>>;

    fn handle(&mut self, msg: super::ScanDataCommand, _: &mut Self::Context) -> Self::Result {
        let mut ret = BTreeMap::<Key, DataRow>::new();

        if msg.table.starts_with("balance_sheet_") {
            let timeout = self.timeout.clone();

            if let Some(rest) = msg.table.strip_prefix("balance_sheet_") {
                let parts = rest.split('_').collect::<Vec<&str>>();
                if parts.len() == 1 {
                    let client = Arc::new(HttpClient::default());
                    let stock = parts[0].to_string();

                    return Box::pin(async move {
                        match fetch_balance_sheet_per_stock(
                                client, 
                                &stock, 
                                timeout
                            ).await 
                        {
                            Ok(balance_sheet) => {
                                for detail in balance_sheet {
                                    let session = format!("{}{:02}", detail.year, detail.quarter);
                                    let key = Key::Str(session.clone());
                                    let row = DataRow::Vec(
                                        vec![
                                            Value::Str(session.clone()),
                                            Value::I32 (detail.shortAsset.unwrap_or(-1000000000)),
                                            Value::I32(detail.cash.unwrap_or(-1000000000)),
                                            Value::I32(detail.shortInvest.unwrap_or(-1000000000)),
                                            Value::I32(detail.shortReceivable.unwrap_or(-1000000000)),
                                            Value::I32(detail.inventory.unwrap_or(-1000000000)),
                                            Value::I32(detail.longAsset.unwrap_or(-1000000000)),
                                            Value::I32(detail.fixedAsset.unwrap_or(-1000000000)),
                                            Value::I32(detail.asset.unwrap_or(-1000000000)),
                                            Value::I32(detail.debt.unwrap_or(-1000000000)),
                                            Value::I32(detail.shortDebt.unwrap_or(-1000000000)),
                                            Value::I32(detail.longDebt.unwrap_or(-1000000000)),
                                            Value::I32(detail.equity.unwrap_or(-1000000000)),
                                            Value::I32(detail.capital.unwrap_or(-1000000000)),
                                            Value::I32(detail.centralBankDeposit.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherBankDeposit.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherBankLoan.unwrap_or(-1000000000)),
                                            Value::I32(detail.stockInvest.unwrap_or(-1000000000)),
                                            Value::I32(detail.customerLoan.unwrap_or(-1000000000)),
                                            Value::I32(detail.badLoan.unwrap_or(-1000000000)),
                                            Value::I32(detail.provision.unwrap_or(-1000000000)),
                                            Value::I32(detail.netCustomerLoan.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherAsset.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherBankCredit.unwrap_or(-1000000000)),
                                            Value::I32(detail.oweOtherBank.unwrap_or(-1000000000)),
                                            Value::I32(detail.oweCentralBank.unwrap_or(-1000000000)),
                                            Value::I32(detail.valuablePaper.unwrap_or(-1000000000)),
                                            Value::I32(detail.payableInterest.unwrap_or(-1000000000)),
                                            Value::I32(detail.receivableInterest.unwrap_or(-1000000000)),
                                            Value::I32(detail.deposit.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherDebt.unwrap_or(-1000000000)),
                                            Value::I32(detail.fund.unwrap_or(-1000000000)),
                                            Value::I32(detail.unDistributedIncome.unwrap_or(-1000000000)),
                                            Value::I32(detail.minorShareHolderProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.payable.unwrap_or(-1000000000)),
                                        ]);

                                    ret.insert(key, row);
                                } 
                            },
                            Err(error) => { 
                                println!("{}", error);
                            },
                        };

                        return ret;
                    });
                }
            }
        } else if  msg.table.starts_with("income_statement_") {
            let timeout = self.timeout.clone();

            if let Some(rest) = msg.table.strip_prefix("income_statement_") {
                let parts = rest.split('_').collect::<Vec<&str>>();
                if parts.len() == 1 {
                    let client = Arc::new(HttpClient::default());
                    let stock = parts[0].to_string();

                    return Box::pin(async move {
                        match fetch_income_statement_per_stock(
                                client, 
                                &stock, 
                                timeout
                            ).await 
                        {
                            Ok(income_statement) => {
                                for detail in income_statement {
                                    let session = format!("{}{:02}", detail.year, detail.quarter);
                                    let key = Key::Str(session.clone());
                                    let row = DataRow::Vec(
                                        vec![
                                            Value::Str(session.clone()),
                                            Value::I32(detail.revenue.unwrap_or(-1000000000)),
                                            Value::F64(detail.yearRevenueGrowth.unwrap_or(-1000000000.0)),
                                            Value::F64(detail.quarterRevenueGrowth.unwrap_or(-1000000000.0)),
                                            Value::I32(detail.costOfGoodSold.unwrap_or(-1000000000)),
                                            Value::I32(detail.grossProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.operationExpense.unwrap_or(-1000000000)),
                                            Value::I32(detail.operationProfit.unwrap_or(-1000000000)),
                                            Value::F64(detail.yearOperationProfitGrowth.unwrap_or(-1000000000.0)),
                                            Value::F64(detail.quarterOperationProfitGrowth.unwrap_or(-1000000000.0)),
                                            Value::I32(detail.interestExpense.unwrap_or(-1000000000)),
                                            Value::I32(detail.preTaxProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.postTaxProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.shareHolderIncome.unwrap_or(-1000000000)),
                                            Value::F64(detail.yearShareHolderIncomeGrowth.unwrap_or(-1000000000.0)),
                                            Value::F64(detail.quarterShareHolderIncomeGrowth.unwrap_or(-1000000000.0)),
                                            Value::I32(detail.investProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.serviceProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.otherProfit.unwrap_or(-1000000000)),
                                            Value::I32(detail.provisionExpense.unwrap_or(-1000000000)),
                                            Value::I32(detail.operationIncome.unwrap_or(-1000000000)),
                                            Value::I32(detail.ebitda.unwrap_or(-1000000000)),
                                        ]);

                                    ret.insert(key, row);
                                }
                            },
                            Err(error) => { 
                                println!("{}", error);
                            },
                        };

                        return ret;
                    });
                }
            }
        } else if msg.table.starts_with("cash_flow_") {
            let timeout = self.timeout.clone();

            if let Some(rest) = msg.table.strip_prefix("cash_flow_") {
                let parts = rest.split('_').collect::<Vec<&str>>();
                if parts.len() == 1 {
                    let client = Arc::new(HttpClient::default());
                    let stock = parts[0].to_string();

                    return Box::pin(async move {
                        match fetch_cash_flow_per_stock(
                                client, 
                                &stock, 
                                timeout
                            ).await 
                        {
                            Ok(cashflow) => {
                                for detail in cashflow {
                                    let session = format!("{}{:02}", detail.year, detail.quarter);
                                    let key = Key::Str(session.clone());
                                    let row = DataRow::Vec(
                                        vec![
                                            Value::Str(session.clone()),
                                            Value::I32(detail.investCost.unwrap_or(-1000000000)),
                                            Value::I32(detail.fromInvest.unwrap_or(-1000000000)),
                                            Value::I32(detail.fromFinancial.unwrap_or(-1000000000)),
                                            Value::I32(detail.fromSale.unwrap_or(-1000000000)),
                                            Value::I32(detail.freeCashFlow.unwrap_or(-1000000000)),
                                        ]);

                                    ret.insert(key, row);
                                }
                            },
                            Err(error) => { 
                                println!("{}", error);
                            },
                        };

                        return ret;
                    });
                }
            }
        }
        
        return Box::pin(async move { ret });
    }
}

pub fn connect_to_tcbs(
    resolver: &mut CronResolver,
    pool: Arc<PgPool>,
    stocks: Vec<String>,
) -> Addr<TcbsActor> {
    use crate::schemas::database::tbl_tcbs_orders::dsl::*;

    let actor = TcbsActor::new(stocks).start();
    let tcbs = actor.clone();

    resolver.resolve(
        "tcbs.get_order_command".to_string(),
        move |arguments, from, to| {
            let tcbs = tcbs.clone();
            let pool = pool.clone();

            async move {
                let mut dbconn = pool.get().unwrap();
                let mut page = 0;

                loop {
                    let datapoints = &match tcbs.send(GetOrderCommand { page }).await {
                        Ok(datapoints) => datapoints,
                        Err(error) => {
                            capture_error(&error);

                            // @NOTE: ignore this error, only return empty BTreeMap
                            Vec::<OrderResponse>::new()
                        }
                    };

                    for point in datapoints {
                        if point.size > 0 {
                            break;
                        }
                    }

                    let _ = datapoints
                        .iter()
                        .map(|response| {
                            let val_symbol = &response.ticker;

                            let rows = response
                                .data
                                .iter()
                                .map(move |point| {
                                    let mut val_side = 1;
                                    let hms = point.t.split(":").collect::<Vec<&str>>();
                                    let val_price = (point.p as f32) / 1000.0;
                                    let val_volume = point.v as i32;
                                    let val_ordered_at = Utc::now()
                                        .with_timezone(&Ho_Chi_Minh)
                                        .with_time(
                                            NaiveTime::from_hms_opt(
                                                hms[0].parse::<u32>().unwrap(),
                                                hms[1].parse::<u32>().unwrap(),
                                                hms[2].parse::<u32>().unwrap(),
                                            )
                                            .unwrap(),
                                        )
                                        .unwrap()
                                        .naive_utc();

                                    if point.a == "SD" {
                                        val_side = 2;
                                    } else if point.a == "" {
                                        val_side = 3;
                                    }

                                    (
                                        symbol.eq(val_symbol.clone()),
                                        side.eq(val_side),
                                        price.eq(val_price),
                                        volume.eq(val_volume),
                                        ordered_at.eq(val_ordered_at.clone()),
                                    )
                                })
                                .collect::<Vec<_>>();

                            diesel::insert_into(tbl_tcbs_orders)
                                .values(&rows)
                                .execute(&mut dbconn)
                        })
                        .collect::<Vec<_>>();

                    page += 1;
                }
            }
        },
    );

    return actor.clone();
}
