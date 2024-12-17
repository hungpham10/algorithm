use std::collections::BTreeMap;
use std::fmt;
use std::error;
use std::sync::Arc;

use actix::prelude::*;
use actix::Addr;

use gluesql::core::ast::ColumnDef;
use gluesql::core::data::Schema;
use gluesql::core::store::DataRow;
use gluesql::prelude::DataType;
use gluesql::prelude::Key;
use gluesql::prelude::Value;

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct VietcapError {
    message: String,
}

impl fmt::Display for VietcapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for VietcapError {}

pub struct VietcapActor {
}

impl VietcapActor {
    fn new() -> Self {
        Self { }
    }
}

impl Actor for VietcapActor {
    type Context = Context<Self>;
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompanyFinancialRatio {
    pub ticker:              String,
    pub yearReport:          u16,
    pub lengthReport:        u8,
    pub updateDate:          u32,
    pub revenue:             i64,
    pub revenueGrowth:       f64,
    pub netProfit:           i64,
    pub netProfitGrowth:     f64, 
    pub ebitMargin:          f64, 
    pub roe:                 f64,
    pub roic:                f64,
    pub roa:                 f64,
    pub pe:                  f64,
    pub pb:                  f64,
    pub eps:                 f64,
    pub currentRatio:        f64,
    pub cashRatio:           f64,
    pub quickRatio:          f64,
    pub interestCoverage:    f64,
    pub ae:                  f64,
    pub netProfitMargin:     f64,
    pub grossMargin:         f64,
    pub ev:                  i64, 
    pub issueShare:          i32,
    pub ps:                  f64,
    pub pcf:                 f64,
    pub bvps:                f64,
    pub evPerEbitda:         f64,
    pub at:                  f64,
    pub fat:                 f64,
    pub acp:                 f64,
    pub dso:                 f64,
    pub dpo:                 f64,
    pub ccc:                 f64,
    pub de:                  f64,
    pub le:                  f64,
    pub ebitda:              f64,
    pub ebit:                i64,
    pub dividend:            f64,
    pub RTQ10:               f64,
    pub charterCapitalRatio: f64,
    pub RTQ4:                f64,
    pub epsTTM:              f64,
    pub charterCapital:      i32,
    pub fae:                 f64,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompanyFinancialRatioDataplan {
    pub ratio: Vec<CompanyFinancialRatio>,
    pub periot: Vec<String>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompanyFinancialRatioResponse {
    pub data: CompanyFinancialRatioDataplan,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompanyFinancialRatioVariables {
    pub ticker: String,
    pub period: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompanyFinancialRatioRequest {
    pub query: String,
    pub variables: CompanyFinancialRatioVariables,
}

async fn fetch_company_financial_ratio(
    client: Arc<HttpClient>,
    stock: String, 
    period: String,
) -> Result<Vec<CompanyFinancialRatio>, VietcapError> {
    let query = "fragment Ratios on CompanyFinancialRatio {\n ticker\n  yearReport\n  lengthReport\n  updateDate\n  revenue\n  revenueGrowth\n  netProfit\n  netProfitGrowth\n  ebitMargin\n  roe\n  roic\n  roa\n  pe\n  pb\n  eps\n  currentRatio\n  cashRatio\n  quickRatio\n  interestCoverage\n  ae\n  netProfitMargin\n  grossMargin\n  ev\n  issueShare\n  ps\n  pcf\n  bvps\n  evPerEbitda\n at\n  fat\n  acp\n  dso\n  dpo\n  ccc\n  de\n  le\n  ebitda\n  ebit\n  dividend\n  RTQ10\n  charterCapitalRatio\n  RTQ4\n  epsTTM\n  charterCapital\n  fae\n  __typename\n}\n\nquery Query($ticker: String!, $period: String!) {\n  CompanyFinancialRatio(ticker: $ticker, period: $period) {\n    ratio {\n      ...Ratios\n      __typename\n    }\n    period\n    __typename\n  }\n}";
    let resp = client.post("https://api.vietcap.com.vn/data-mt/graphq")
        .json(&CompanyFinancialRatioRequest{
            query: query.to_string(),
            variables: CompanyFinancialRatioVariables { 
                ticker: stock, 
                period: period,
            },
        })
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<CompanyFinancialRatioResponse>().await {
            Ok(resp) => Ok(resp.data.ratio),
            Err(error) => Err(VietcapError{
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(VietcapError{
            message: format!("{:?}", error),
        }),
    }
}

impl Handler<super::ListSchemaCommand> for VietcapActor {
    type Result = ResponseFuture<Vec<Schema>>;

    fn handle(&mut self, _: super::ListSchemaCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { Vec::<Schema>::new() })
    }
}

impl Handler<super::FetchDataCommand> for VietcapActor {
    type Result = ResponseFuture<Option<DataRow>>;

    fn handle(&mut self, msg: super::FetchDataCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { None })
    }
}

impl Handler<super::ScanDataCommand> for VietcapActor {
    type Result = ResponseFuture<BTreeMap<Key, DataRow>>;

    fn handle(&mut self, msg: super::ScanDataCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { BTreeMap::<Key, DataRow>::new() })
    }
}

pub fn connect_to_vietcap() -> Addr<VietcapActor> {
    VietcapActor::new().start()
}
