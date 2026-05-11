use std::io::{Error, ErrorKind};

use schemas::CandleStick;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use utoipa::ToSchema;
use vector_config_macro::output;
use vector_runtime::{Component, Event, Identify, Message, Outbound};

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

#[output(derive(PartialEq))]
pub struct PriceGatherOutput {
    pub id: String,
}

impl_price_gather_output!(
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        while let Some(msg) = rx.recv().await {
            if let Some(ref broadcast) = tx.broadcast
                && let Err(error) = broadcast.send(msg)
            {
                tx.event
                    .send(Event::Minor((
                        id,
                        Error::other(format!("Failed to send msg to boardcast: {error}")),
                    )))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to send issue: {error}"),
                        )
                    })?;
            }
        }
        Ok(())
    }
);
