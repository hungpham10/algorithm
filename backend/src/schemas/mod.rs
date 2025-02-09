use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

pub mod database;
pub mod tsdb;

#[derive(GraphQLObject, Debug)]
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

#[derive(GraphQLObject, Deserialize, Serialize, Debug)]
#[graphql(description = "Information about order")]
pub struct Order {
    #[graphql(description = "order id")]
    pub id: String,

    #[graphql(description = "stock name")]
    pub stock: String,

    #[graphql(description = "order state")]
    pub state: i32,

    #[graphql(description = "order open date")]
    pub order_open_date: Option<i32>,

    #[graphql(description = "order close date")]
    pub order_close_date: Option<i32>,

    #[graphql(description = "earning income")]
    pub earn: Option<f64>,

    #[graphql(description = "price of order")]
    pub price_order: f64,

    #[graphql(description = "stop loss")]
    pub stop_loss: f64,

    #[graphql(description = "take profit")]
    pub take_profit: f64,

    #[graphql(description = "number of stocks")]
    pub number_of_stocks: i32,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Argument to configure jobs")]
pub struct Argument {
    #[graphql(description = "argument in which will be used for jobs")]
    pub argument: String,

    #[graphql(description = "value of the argument which you are configuring")]
    pub value: String,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Information about cronjob")]
pub struct CronJob {
    #[graphql(description = "timeout")]
    pub timeout: i32,

    #[graphql(description = "interval when cronjob run")]
    pub interval: String,

    #[graphql(description = "which job will be perform to resolve tasks")]
    pub resolver: String,

    #[graphql(description = "arguments for this job")]
    pub arguments: Option<Vec<Argument>>,
}

#[derive(GraphQLObject, Debug)]
#[graphql(description = "Information about job")]
pub struct SingleJob {
    #[graphql(description = "timeout")]
    pub timeout: i32,

    #[graphql(description = "which job will be perform to resolve tasks")]
    pub resolver: String,

    #[graphql(description = "arguments for this job")]
    pub arguments: Option<Vec<Argument>>,

    #[graphql(description = "start of time range where timeserie data will be taken")]
    pub from: Option<i32>,

    #[graphql(description = "end of time range where timeserie data will be taken")]
    pub to: Option<i32>,
}

