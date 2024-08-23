use std::time::Duration;
use std::sync::Arc;
use std::fmt;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use reqwest::{
    Client as HttpClient, 
    Error as HttpError,

    header::AUTHORIZATION, 
};
use diesel::prelude::*;
use actix::prelude::*;
use actix::Addr;

use crate::helpers::{PgConn, PgPool};
use crate::actors::redis::RedisActor;
use crate::actors::cron::CronResolver;
use crate::analytic::Sentiment;
use crate::analytic::mention::Mention;


#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub name: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub price:  Option<f32>,
    pub change: Option<f32>,
    pub percentChange: f32,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Post {
    // @NOTE: data for analysis
    pub originalContent: String,
    pub taggedSymbols:   Vec<Symbol>,

    // @NOTE: post profile
    pub postID:    u64,
    pub user:      User,
    pub link:      Option<String>,
    pub date:      String,
    pub priority:  i32,
    pub sentiment: i32,

    // @NOTE: flags
    pub isTop:    bool,
    pub hasImage: bool,
    pub hasFile:  bool,

    // @NOTE: counters
    pub totalLikes:   u32,
    pub totalShares:  u32,
    pub totalReplies: u32,    
}

pub struct FireantActor {
    timeout: u64,
    limit:   usize,
    token:   String,
}

impl FireantActor {
    fn new(token: String) -> Self {
        Self {
            timeout: 60,
            limit:   100,
            token:   token.clone(),
        }
    }
}

impl Actor for FireantActor {
    type Context = Context<Self>;
}

#[derive(Debug, Clone)]
pub struct FireantError {
    message: String
}

impl fmt::Display for FireantError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<BTreeMap<String, Sentiment>, HttpError>")]
pub struct CountSentimentPerStockCommand {
    from: i64,
    to:   i64,
}

impl Handler<CountSentimentPerStockCommand> for FireantActor {
    type Result = ResponseFuture<Result<BTreeMap<String, Sentiment>, HttpError>>;

    fn handle(&mut self, msg: CountSentimentPerStockCommand, _: &mut Self::Context) -> Self::Result { 
        let timeout = self.timeout;
        let limit   = self.limit;
        let token   = self.token.clone();
        let from    = msg.from;
        let to      = msg.to;

        Box::pin(async move {
            let client     = Arc::new(HttpClient::default());
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
    client:  Arc<HttpClient>,
    from:    i64,
    to:      i64,
    timeout: u64,
    limit:   usize,
    token:   String,
) -> Result<BTreeMap<String, Sentiment>, HttpError> {
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
        ).await;

        match resp {
            Ok(posts) => {
                if posts.len() == 0 {
                    println!("empty");
                    break;
                }

                let time_happen = DateTime::parse_from_rfc3339(
                    posts.last().unwrap().date.as_str(),
                ).unwrap();

                model.count_mention_by_symbol(&mut statistic, &posts);
                model.count_sentiment_vote_by_symbol(&mut statistic, &posts);

                if time_happen.timestamp() < from || posts.len() < limit {
                    break;
                }

                offset += limit;
            },
            Err(error) => {
                return Err(error);
            }
        }
    }

    return Ok(statistic);
}

async fn fetch_batch_of_posts_from_fireant(
    client:  Arc<HttpClient>,
    offset:  usize,
    limit:   usize,
    timeout: u64,
    token:   String,
) -> Result<Vec<Post>, HttpError> {
    let resp = client.get(format!(
            "https://restv2.fireant.vn/posts?type=0&offset={}&limit={}",
            offset,
            limit
        ))
        .header(AUTHORIZATION, token)
        .timeout(Duration::from_secs(timeout))
        .send()
        .await;

    match resp {
        Ok(resp) => {
            match resp.json::<Vec<Post>>().await {
                Ok(posts) => Ok(posts),
                Err(error) => {
                    println!("{:?}", error);
                    Ok(Vec::<Post>::new())
                },
            }
        }
        Err(error) => {
            println!("{:?}", error);
            Err(error)
        }
    }
}

pub fn connect_to_fireant(
    resolver: &mut CronResolver,
    pool:     Arc<PgPool>,
    cache:    Arc<Addr<RedisActor>>, 
    token:    String,
) -> Addr<FireantActor> {
    use crate::schemas::database::tbl_fireant_mention::dsl::*;

    let actor   = FireantActor::new(format!("Bearer {}", token)).start();
    let fireant = actor.clone();

    resolver.resolve("fireant.count_sentiment_per_stock".to_string(), move || {
        let fireant = fireant.clone();
        let pool    = pool.clone();
        let time    = Utc::now().timestamp();

        async move {
            let mut dbconn = pool.get().unwrap();
            let from       = time - 24*60*60;
            let to         = time;
            let sentiments = fireant.send(CountSentimentPerStockCommand{
                    from: from,
                    to:   to,
                })
                .await
                .unwrap().unwrap(); 

            let rows = sentiments.iter()
                .map(|(name, value)| {
                    (
                        symbol.eq(name), 
                        mention.eq(value.mention), 
                        positive.eq(value.votes.positive), 
                        negative.eq(value.votes.negative),
                    )
                })
                .collect::<Vec<_>>();
                
            diesel::insert_into(tbl_fireant_mention)
                .values(&rows)
                .execute(&mut dbconn);
        }
    });

    return actor;
}

