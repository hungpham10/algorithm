use std::sync::Arc;
use std::time::Duration;
use std::fmt;

use serde::{Deserialize, Serialize};
use reqwest::{
    Client as HttpClient, 
    Error as HttpError,
};
use futures::future;

use chrono::{Utc, NaiveTime};
use chrono_tz::{Asia::Ho_Chi_Minh, Tz};

use diesel::prelude::*;
use actix::prelude::*;
use actix::Addr;

use influxdb::{Client as InfluxClient, InfluxDbWriteable};

use crate::helpers::{PgConn, PgPool};
use crate::actors::cron::CronResolver;

#[derive(Debug, Clone)]
pub struct TcbsError {
    message: String
}

impl fmt::Display for TcbsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct TcbsActor {
    stocks:  Vec<String>,
    timeout: u64,
}


impl TcbsActor {
    fn new(stocks: Vec<String>) -> Self {
        Self {
            stocks:  stocks,
            timeout: 60,
        }
    }
}

impl Actor for TcbsActor {
    type Context = Context<Self>;
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
pub struct GetOrderCommand{
    page: usize,
}

impl Handler<GetOrderCommand> for TcbsActor {
    type Result = ResponseFuture<Vec<OrderResponse>>;

    fn handle(&mut self, msg: GetOrderCommand, _: &mut Self::Context) -> Self::Result { 
        let stocks = self.stocks.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            let datapoints = fetch_orders(&stocks, timeout, msg.page, 100)
                .await;

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
    let client = Arc::new(HttpClient::default());

    future::try_join_all(
            stocks.iter().map(move |stock| {
                fetch_order_per_stock(client.clone(), stock, timeout, page, page_size)
            })
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
) -> Result<OrderResponse, HttpError> {
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
        Ok(resp) => resp.json::<OrderResponse>().await,
        Err(error) => Err(error),
    }
}

pub fn connect_to_tcbs(
    resolver: &mut CronResolver,
    pool:     Arc<PgPool>,
    stocks:   Vec<String>,
) -> Addr<TcbsActor> {
    use crate::schemas::database::tbl_tcbs_orders::dsl::*;

    let actor = TcbsActor::new(stocks).start();
    let tcbs = actor.clone();

    resolver.resolve("tcbs.get_order_command".to_string(), move || {
        let tcbs = tcbs.clone();
        let pool = pool.clone();

        async move {
            let mut dbconn = pool.get().unwrap();
            let mut page = 0;

            loop {
                let datapoints = tcbs.send(GetOrderCommand{ page: page })
                    .await
                    .unwrap();

                if datapoints.len() == 0 {
                    break;
                }

                let _ = datapoints.iter()
                    .map(|response| {
                        let val_symbol = &response.ticker;

                        let rows = response.data.iter()
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
                                        ).unwrap(),
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
    });
 
    return actor.clone();
}

