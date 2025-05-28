use chrono::DateTime;
use std::error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::AUTHORIZATION;
use reqwest_middleware::ClientWithMiddleware as HttpClient;
use serde::{Deserialize, Serialize};

use actix::prelude::*;
use actix::Addr;

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
    fn new(token: String, limit: usize) -> Self {
        Self {
            limit,
            timeout: 60,
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
#[rtype(result = "Result<Vec<Post>, FireantError>")]
pub struct ScrapePostsCommand {
    pub from: i64,
    pub to: i64,
    pub symbol: String,
}

impl Handler<ScrapePostsCommand> for FireantActor {
    type Result = ResponseFuture<Result<Vec<Post>, FireantError>>;

    fn handle(&mut self, msg: ScrapePostsCommand, _: &mut Self::Context) -> Self::Result {
        let timeout = self.timeout;
        let token = self.token.clone();
        let limit = self.limit;
        let symbol = msg.symbol;
        let from = msg.from;
        let to = msg.to;

        Box::pin(async move {
            let client = Arc::new(HttpClient::default());
            let mut result = Vec::new();
            let mut offset: usize = 0;

            loop {
                let resp = fetch_batch_of_posts_from_fireant(
                    client.clone(),
                    offset,
                    limit,
                    timeout,
                    token.clone(),
                    symbol.clone(),
                )
                .await;

                match resp {
                    Ok(mut posts) => {
                        if posts.is_empty() {
                            break;
                        }

                        let len = posts.len();
                        let time_happen =
                            DateTime::parse_from_rfc3339(posts.last().unwrap().date.as_str())
                                .unwrap();

                        if time_happen.timestamp() > to {
                            continue;
                        }

                        result.append(&mut posts);

                        if time_happen.timestamp() < from || len < limit {
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

            Ok(result)
        })
    }
}

async fn fetch_batch_of_posts_from_fireant(
    client: Arc<HttpClient>,
    offset: usize,
    limit: usize,
    timeout: u64,
    token: String,
    symbol: String,
) -> Result<Vec<Post>, FireantError> {
    let url = if symbol.is_empty() {
        format!(
            "https://restv2.fireant.vn/posts?type=0&offset={}&limit={}",
            offset, limit,
        )
    } else {
        format!(
            "https://restv2.fireant.vn/posts?type=0&offset={}&limit={}&symbol={}",
            offset, limit, symbol,
        )
    };

    let resp = client
        .get(url)
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
    foreignOwnership: f64,
}

async fn fetch_detail_stock_information_from_fireant(
    client: Arc<HttpClient>,
    symbol: String,
    token: String,
    timeout: u64,
) -> Result<StockInformation, FireantError> {
    let resp = client
        .get(format!(
            "https://restv2.fireant.vn/symbols/{}/fundamental",
            symbol,
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

pub async fn connect_to_fireant(token: String, limit: usize) -> Addr<FireantActor> {
    FireantActor::new(format!("Bearer {}", token), limit).start()
}
