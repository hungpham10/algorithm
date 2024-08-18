use std::sync::Arc;
use std::fmt;

use serde::{Deserialize, Serialize};
use reqwest::{Client, Error};
use futures::future;

use actix::prelude::*;
use actix::Addr;


#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub mp: String,
    pub CWUnderlying: String,
    pub CWType: String,
    pub CWLastTradingDate: String,
    pub CWExcersisePrice: String,
    pub CWExerciseRatio: String,
    pub CWListedShare: String,
    pub sType: String,
    pub sBenefit: String,
}

#[derive(Debug, Clone)]
pub struct VpsError {
    message: String
}

impl fmt::Display for VpsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct VpsActor {
    stocks: Vec<String>,
}

impl VpsActor {
    fn new(stocks: Vec<String>) -> Self {
        VpsActor{
            stocks: stocks,
        }
    }
}

impl Actor for VpsActor {
    type Context = Context<Self>;
}

pub fn connect_to_vps(stocks: Vec<String>) -> Addr<VpsActor> {
    VpsActor::new(stocks).start()
}

#[derive(Message, Debug)]
#[rtype(result = "Vec<Price>")]
pub struct GetPriceCommand;

impl Handler<GetPriceCommand> for VpsActor {
    type Result = ResponseFuture<Vec<Price>>;

    fn handle(&mut self, _msg: GetPriceCommand, _: &mut Self::Context) -> Self::Result { 
        let stocks = self.stocks.clone();

        Box::pin(async move {
            fetch_price_depth(
                    &stocks,
                )
                .await
        })
    }
}

async fn fetch_price_depth(
    stocks: &Vec<String>,
) -> Vec<Price> {
    let client = Arc::new(Client::default());
    let blocks: Vec<Vec<String>>  = (*stocks).chunks(100)
        .map(|block| {
            block.iter()
                .map(|stock| (*stock).clone())
                .collect()
        })
        .collect();

    future::try_join_all(
        blocks.iter()
            .map(move |block| {
            fetch_price_depth_per_block(client.clone(), block)
        })
    )
    .await
    .unwrap()
    .into_iter()
    .flatten()
    .collect()
}

async fn fetch_price_depth_per_block(
    client: Arc<Client>,
    block:  &Vec<String>,
) -> Result<Vec<Price>, Error> {
    let mut resp = client.get(format!(
            "https://bgapidatafeed.vps.com.vn/getliststockdata/{}",
            (*block).join(","),
        ))
        .send()
        .await
        .unwrap();

    resp.json::<Vec<Price>>().await
}
