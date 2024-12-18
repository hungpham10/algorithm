use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::AUTHORIZATION;
use chrono::{DateTime, Utc};
use log::{error, info};
use reqwest_middleware::ClientWithMiddleware as HttpClient;
use sentry::capture_error;
use serde::{Deserialize, Serialize};

use diesel::prelude::*;
use actix::prelude::*;
use actix::Addr;

use gluesql::core::store::DataRow;
use gluesql::core::ast::ColumnDef;
use gluesql::core::data::Schema;
use gluesql::prelude::DataType;
use gluesql::prelude::Key;
use gluesql::prelude::Value;

use crate::actors::cron::CronResolver;
use crate::actors::redis::RedisActor;
use crate::analytic::mention::Mention;
use crate::analytic::Sentiment;
use crate::helpers::PgPool;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub name: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub price: Option<f32>,
    pub change: Option<f32>,
    pub percentChange: Option<f32>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Post {
    // @NOTE: data for analysis
    pub originalContent: String,
    pub taggedSymbols: Vec<Symbol>,

    // @NOTE: post profile
    pub postID: u64,
    pub user: User,
    pub link: Option<String>,
    pub date: String,
    pub priority: i32,
    pub sentiment: i32,

    // @NOTE: flags
    pub isTop: bool,
    pub hasImage: bool,
    pub hasFile: bool,

    // @NOTE: counters
    pub totalLikes: u32,
    pub totalShares: u32,
    pub totalReplies: u32,
}

pub struct FireantActor {
    timeout: u64,
    limit: usize,
    token: String,
}

impl FireantActor {
    fn new(token: String) -> Self {
        Self {
            timeout: 60,
            limit: 100,
            token: token.clone(),
        }
    }
}

impl Actor for FireantActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct FireantError {
    message: String,
}

impl fmt::Display for FireantError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for FireantError {}

impl Handler<super::HealthCommand> for FireantActor {
    type Result = ResponseFuture<bool>;

    fn handle(&mut self, _msg: super::HealthCommand, _: &mut Self::Context) -> Self::Result {
        Box::pin(async move { true })
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<BTreeMap<String, Sentiment>, FireantError>")]
pub struct CountSentimentPerStockCommand {
    from: i64,
    to: i64,
}

impl Handler<CountSentimentPerStockCommand> for FireantActor {
    type Result = ResponseFuture<Result<BTreeMap<String, Sentiment>, FireantError>>;

    fn handle(
        &mut self,
        msg: CountSentimentPerStockCommand,
        _: &mut Self::Context,
    ) -> Self::Result {
        let timeout = self.timeout;
        let limit = self.limit;
        let token = self.token.clone();
        let from = msg.from;
        let to = msg.to;

        Box::pin(async move {
            let client = Arc::new(HttpClient::default());
            let datapoints = statistic_posts_by_stock_in_timerange(
                client.clone(),
                from,
                to,
                timeout,
                limit,
                token.clone(),
            )
            .await;

            return datapoints;
        })
    }
}

async fn statistic_posts_by_stock_in_timerange(
    client: Arc<HttpClient>,
    from: i64,
    to: i64,
    timeout: u64,
    limit: usize,
    token: String,
) -> Result<BTreeMap<String, Sentiment>, FireantError> {
    let mut statistic = BTreeMap::<String, Sentiment>::new();
    let mut offset: usize = 0;

    let model = Mention::new();

    loop {
        let resp = fetch_batch_of_posts_from_fireant(
            client.clone(),
            offset,
            limit,
            timeout,
            token.clone(),
        )
        .await;

        match resp {
            Ok(posts) => {
                if posts.len() == 0 {
                    println!("empty");
                    break;
                }

                let time_happen =
                    DateTime::parse_from_rfc3339(posts.last().unwrap().date.as_str()).unwrap();

                model.count_mention_by_symbol(&mut statistic, &posts);
                model.count_sentiment_vote_by_symbol(&mut statistic, &posts);
                model.count_youtube_link_by_symbol(&mut statistic, &posts);

                if time_happen.timestamp() < from || posts.len() < limit {
                    break;
                }

                offset += limit;
            }
            Err(error) => {
                return Err(FireantError {
                    message: format!("{:?}", error),
                });
            }
        }
    }

    return Ok(statistic);
}

async fn fetch_batch_of_posts_from_fireant(
    client: Arc<HttpClient>,
    offset: usize,
    limit: usize,
    timeout: u64,
    token: String,
) -> Result<Vec<Post>, FireantError> {
    let resp = client
        .get(format!(
            "https://restv2.fireant.vn/posts?type=0&offset={}&limit={}",
            offset, limit
        ))
        .header(AUTHORIZATION, token)
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<Vec<Post>>().await {
            Ok(posts) => Ok(posts),
            Err(error) => Err(FireantError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(FireantError {
            message: format!("{:?}", error),
        }),
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct StockInformation {
    companyType: i32,
    sharesOutstanding: f64,
    freeShares: f64,
    beta: f64,
    dividend: f64,
    dividendYield: f64,
    marketCap: f64,
    low52Week: f64,
    high52Week: f64,
    priceChange1y: f64,
    avgVolume10d: f64,
    avgVolume3m: f64,
    pe: f64,
    eps: f64,
    sales_TTM: f64,
    netProfit_TTM: f64,
    insiderOwnership: f64,
    institutionOwnership: f64,
    foreignOwnership: f64
}

async fn fetch_detail_stock_information_from_fireant(
    client: Arc<HttpClient>,
    symbol: String,
    token: String,
    timeout: u64,
) -> Result<StockInformation, FireantError> {
    let resp = client
        .get(format!(
            "https://restv2.fireant.vn/symbols/{}/fundamental", symbol,
        ))
        .header(AUTHORIZATION, token)
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => match resp.json::<StockInformation>().await {
            Ok(detail) => Ok(detail),
            Err(error) => Err(FireantError {
                message: format!("{:?}", error),
            }),
        },
        Err(error) => Err(FireantError {
            message: format!("{:?}", error),
        }),
    }
}

impl Handler<super::ListSchemaCommand> for FireantActor {
    type Result = ResponseFuture<Vec<Schema>>;

    fn handle(&mut self, _: super::ListSchemaCommand, _: &mut Self::Context) -> Self::Result {
        // @TODO: hien thi tat ca schema

        Box::pin(async move { 
            let mut result: Vec<Schema> = Vec::<Schema>::new();
 
            result.push(Schema{
                table_name: "stocks_fudamential_analysis".to_string(),
                column_defs: Some(vec![
                    ColumnDef{
                        name: "symbol".to_string(),
                        data_type: DataType::Text,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    },
                    ColumnDef{
                        name: "type".to_string(),
                        data_type: DataType::Int32,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    },
                    ColumnDef{
                        name: "marketcap".to_string(),
                        data_type: DataType::Decimal,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    }, 
                    ColumnDef{
                        name: "eps".to_string(),
                        data_type: DataType::Decimal,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    }, 
                    ColumnDef{
                        name: "beta".to_string(),
                        data_type: DataType::Decimal,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    }, 
                    ColumnDef{
                        name: "freeload".to_string(),
                        data_type: DataType::Decimal,
                        unique: None,
                        default: None,
                        nullable: false,
                        comment: None,
                    }, 
                ]),
                indexes: Vec::new(),
                engine: None,
                foreign_keys: Vec::new(),
                comment: None,
            });

            return result;
        })
    }
}

impl Handler<super::ScanDataCommand> for FireantActor {
    type Result = ResponseFuture<BTreeMap<Key, DataRow>>;

    fn handle(&mut self, msg: super::ScanDataCommand, _: &mut Self::Context) -> Self::Result {
        let table = msg.table.clone();

        println!("{}", table);
        if table == "stocks_fudamential_analysis".to_string() {
            let token = self.token.clone();
            let timeout = self.timeout.clone();

            return Box::pin(async move {
                let client = Arc::new(HttpClient::default());
                let token = token.clone();
                let mut ret = BTreeMap::<Key, DataRow>::new();

                for symbol in super::vps::list_active_stocks().await {
                    let detail = fetch_detail_stock_information_from_fireant(
                            client.clone(),
                            symbol.clone(),
                            token.clone(),
                            timeout,
                        )
                        .await;

                    match detail {
                        Ok(detail) => {
                            let key = Key::Str(symbol.clone());
                            let row = DataRow::Vec(
                                vec![
                                    Value::Str(symbol.clone()),
                                    Value::I32(detail.companyType),
                                    Value::F64(detail.marketCap),
                                    Value::F64(detail.eps),
                                    Value::F64(detail.beta),
                                    Value::F64(detail.sharesOutstanding),
                                ]);

                            ret.insert(key, row);
                        }
                        Err(err) => {
                            capture_error(&err);
                            error!("{}", err);
                        }
                    }

                }
                return ret;
            });
        }

        Box::pin(async move { BTreeMap::<Key, DataRow>::new() })
    }
}

pub fn connect_to_fireant(
    resolver: &mut CronResolver,
    pool: Arc<PgPool>,
    cache: Arc<Addr<RedisActor>>,
    token: String,
) -> Addr<FireantActor> {
    use crate::schemas::database::tbl_fireant_mention::dsl::*;

    let actor = FireantActor::new(format!("Bearer {}", token)).start();
    let fireant = actor.clone();

    resolver.resolve(
        "fireant.count_sentiment_per_stock".to_string(),
        move |arguments, mut from, mut to| {
            let fireant = fireant.clone();
            let pool = pool.clone();

            if to < 0 {
                to = Utc::now().timestamp() as i32;
            }

            if from < 0 || from >= to {
                from = to - 24 * 60 * 60;
            }

            async move {
                let mut dbconn = pool.get().unwrap();
                let sentiments = match fireant
                    .send(CountSentimentPerStockCommand {
                        from: from as i64,
                        to: to as i64,
                    })
                    .await
                {
                    Ok(resp) => match resp {
                        Ok(sentiments) => sentiments,
                        Err(err) => {
                            capture_error(&err);
                            error!("{}", err);

                            // @NOTE: ignore this error, only return empty BTreeMap
                            BTreeMap::<String, Sentiment>::new()
                        }
                    },
                    Err(err) => {
                        capture_error(&err);
                        error!("{}", err);

                        // @NOTE: ignore this error, only return empty BTreeMap
                        BTreeMap::<String, Sentiment>::new()
                    }
                };

                let rows = sentiments
                    .iter()
                    .map(|(name, value)| {
                        (
                            symbol.eq(name),
                            mention.eq(value.mention),
                            positive.eq(value.votes.positive),
                            negative.eq(value.votes.negative),
                            promotion.eq(value.promotion),
                        )
                    })
                    .collect::<Vec<_>>();

                match diesel::insert_into(tbl_fireant_mention)
                    .values(&rows)
                    .execute(&mut dbconn)
                {
                    Ok(cnt) => {
                        info!("Fireant: Insert {} to tbl_fireant_mention", cnt);
                    }
                    Err(error) => {
                        capture_error(&error);
                    }
                }
            }
        },
    );

    return actor;
}
