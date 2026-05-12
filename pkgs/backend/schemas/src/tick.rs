use crate::CandleStick;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Tick {
    #[schema(example = "binance")]
    pub broker: String,

    #[schema(example = "BTCUSDT")]
    pub symbol: String,

    #[schema(example = 60150.5)]
    pub price: f64,

    #[schema(example = 1.25)]
    pub quantity: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub candlestick: Option<CandleStick>,
}
