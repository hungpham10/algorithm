use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use gluesql::core::ast::ColumnDef;
use gluesql::core::data::Schema;
use gluesql::core::store::DataRow;
use gluesql::prelude::DataType;

use gluesql::prelude::Key;
use gluesql::prelude::Value;
use juniper::GraphQLObject;

use reqwest::{Client as HttpClient, Error as HttpError};
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

use crate::algorithm::lru::LruCache;
use super::lru_cache_generate_key;

pub struct DnseActor {
    timeout: u64,

    data_row_cache: LruCache<String, DataRow>,
}

impl DnseActor {
    fn new() -> Self {
        Self { 
            timeout: 60,
            data_row_cache: LruCache::new(100),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Ohcl {
    pub t: Option<Vec<i32>>,
    pub o: Option<Vec<f64>>,
    pub c: Option<Vec<f64>>,
    pub h: Option<Vec<f64>>,
    pub l: Option<Vec<f64>>,
    pub v: Option<Vec<i32>>,
    pub nextTime: i64,
}

#[derive(GraphQLObject)]
#[graphql(description = "Information about japaness candle stick")]
pub struct CandleStick {
    #[graphql(description = "timestamp")]
    pub t: i32,

    #[graphql(description = "open price")]
    pub o: f64,

    #[graphql(description = "highest price")]
    pub h: f64,

    #[graphql(description = "close price")]
    pub c: f64,

    #[graphql(description = "lowest price")]
    pub l: f64,

    #[graphql(description = "volume")]
    pub v: i32,
}

impl Actor for DnseActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct DnseError {
    message: String,
}

impl fmt::Display for DnseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Handler<super::HealthCommand> for DnseActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Vec<CandleStick>, HttpError>")]
pub struct GetOHCLCommand {
    pub resolution: String,
    pub stock: String,
    pub from: i64,
    pub to: i64,
}

impl Handler<GetOHCLCommand> for DnseActor {
    type Result = ResponseFuture<Result<Vec<CandleStick>, HttpError>>;

    fn handle(&mut self, msg: GetOHCLCommand, _: &mut Self::Context) -> Self::Result {
        let resolution = msg.resolution.clone();
        let stock = msg.stock.clone();
        let from = msg.from;
        let to = msg.to;
        let timeout = self.timeout;

        Box::pin(async move {
            let client = Arc::new(HttpClient::default());
            let datapoints =
                fetch_ohcl_by_stock(client.clone(), &stock, &resolution, from, to, timeout).await;

            return datapoints;
        })
    }
}

async fn fetch_ohcl_by_stock(
    client: Arc<HttpClient>,
    stock: &String,
    resolution: &String,
    from: i64,
    to: i64,
    timeout: u64,
) -> Result<Vec<CandleStick>, HttpError> {
    let resp = client.get(format!(
            "https://services.entrade.com.vn/chart-api/v2/ohlcs/stock?from={}&to={}&symbol={}&resolution={}",
            from,
            to,
            (*stock),
            (*resolution),
        ))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => {
            let mut candles = Vec::<CandleStick>::new();
            let ohcl = resp.json::<Ohcl>().await.unwrap();

            if let Some(t) = ohcl.t {
                for i in 0..t.len() {
                    candles.push(CandleStick {
                        t: t[i],
                        o: match ohcl.o.as_ref() {
                            Some(o) => o[i],
                            None => 0.0,
                        },
                        h: match ohcl.h.as_ref() {
                            Some(h) => h[i],
                            None => 0.0,
                        },
                        c: match ohcl.c.as_ref() {
                            Some(c) => c[i],
                            None => 0.0,
                        },
                        l: match ohcl.l.as_ref() {
                            Some(l) => l[i],
                            None => 0.0,
                        },
                        v: match ohcl.v.as_ref() {
                            Some(v) => v[i],
                            None => 0,
                        },
                    })
                }
            }

            Ok(candles)
        }
        Err(error) => Err(error),
    }
}

pub fn list_of_resolution() -> Vec<String> {
    return vec!["1D".to_string(), "1M".to_string(), "1W".to_string()];
}

impl Handler<super::ListSchemaCommand> for DnseActor {
    type Result = ResponseFuture<Vec<Schema>>;

    fn handle(&mut self, _: super::ListSchemaCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { 
            let mut result: Vec<Schema> = Vec::<Schema>::new();

            for stock_name in super::vps::list_of_vn30().await {
                for resolution in list_of_resolution() {
                    // @TODO: cấu hình cột để hiển thị

                    let colume_defs = vec![
                        ColumnDef{
                            name: "timestamp".to_string(),
                            data_type: DataType::Int32,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                        ColumnDef {
                            name: "open".to_string(),
                            data_type: DataType::Float,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                        ColumnDef {
                            name: "high".to_string(),
                            data_type: DataType::Float,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                        ColumnDef {
                            name: "close".to_string(),
                            data_type: DataType::Float,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                        ColumnDef {
                            name: "low".to_string(),
                            data_type: DataType::Float,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                        ColumnDef {
                            name: "volume".to_string(),
                            data_type: DataType::Int32,
                            unique: None,
                            default: None,
                            nullable: false,
                            comment: None,
                        },
                    ];
                            
                    result.push(Schema{
                        table_name: format!("ohcl_{}_{}", stock_name, resolution) ,
                        column_defs: Some(colume_defs),
                        indexes: Vec::new(),
                        engine: None,
                        foreign_keys: Vec::new(),
                        comment: None,
                    }); 
                }
            }

            return result;
        })
    }
}

impl Handler<super::FetchDataCommand> for DnseActor {
    type Result = ResponseFuture<Option<DataRow>>;

    fn handle(&mut self, msg: super::FetchDataCommand, _: &mut Self::Context) -> Self::Result {
        let table = msg.table.clone();
        let target = msg.target.clone();
        let cache = &mut self.data_row_cache;

        match target {
            Key::I32(timestamp) => {
                // @TODO: calculate timestamp of first second of this month

                if let Ok(key_name) = lru_cache_generate_key("dnse", table.as_str(), &Key::I32(timestamp)) {
                    match cache.get(&key_name) {
                        Some(result) => {
                            let result = result.clone();

                            Box::pin(async move { Some(result) })
                        },
                        None => Box::pin(async move { None }),
                    }
                } else {
                    Box::pin(async move { None })
                }
            }
            _ => {
                Box::pin(async move { None })
            }
        } 
    }
}

impl Handler<super::ScanDataCommand> for DnseActor {
    type Result = ResponseFuture<BTreeMap<Key, DataRow>>;

    fn handle(&mut self, msg: super::ScanDataCommand, _: &mut Self::Context) -> Self::Result {
        let ret = BTreeMap::<Key, DataRow>::new();

        if msg.table.starts_with("ohcl_") {
            let client = Arc::new(HttpClient::default());
            
            if let Some(rest) = msg.table.strip_prefix("ohcl_") {
                let parts = rest.split('_').collect::<Vec<&str>>();

                if parts.len() == 2 {
                    let to = chrono::Utc::now().timestamp();
                    let timeout = self.timeout.clone();
                    let from = 0;
                    let stock = parts[0].to_string();
                    let resolution = parts[1].to_string();

                    return Box::pin(async move {
                        fetch_ohcl_by_stock(
                            client.clone(),
                            &stock,
                            &resolution,
                            from,
                            to,
                            timeout,
                        ).await
                        .unwrap()
                        .into_iter()
                        .map(|candle| {
                            let key = Key::I32(candle.t);
                            let row = DataRow::Vec(
                                vec![
                                    Value::I32(candle.t),
                                    Value::F64(candle.o), 
                                    Value::F64(candle.h), 
                                    Value::F64(candle.c), 
                                    Value::F64(candle.l), 
                                    Value::I32(candle.v)
                                    ]
                                );

                            (key, row)
                        })
                        .collect::<BTreeMap<Key, DataRow>>()
                    });
                }
            }
            
        }
            
        Box::pin(async move { ret })
    }
}

pub fn connect_to_dnse() -> Addr<DnseActor> {
    DnseActor::new().start()
}
