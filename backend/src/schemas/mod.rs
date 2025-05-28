use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(GraphQLObject, Deserialize, Serialize, Debug, Clone)]
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
    pub v: f64,
}

#[derive(GraphQLObject, Deserialize, Serialize, Debug, Clone)]
#[graphql(description = "Information about order")]
pub struct Order {
    #[graphql(description = "order id")]
    pub id: String,

    #[graphql(description = "stock name")]
    pub stock: String,

    #[graphql(description = "price of order")]
    pub price_order: f64,

    #[graphql(description = "number of stocks")]
    pub number_of_stocks: i32,

    #[graphql(description = "price where i will abandon the order")]
    pub stop_lost: Option<f64>,

    #[graphql(description = "price where i will close order and earn revenew")]
    pub take_profit: Option<f64>,

    #[graphql(description = "how many money i have earn")]
    pub earn: Option<f64>,
}
