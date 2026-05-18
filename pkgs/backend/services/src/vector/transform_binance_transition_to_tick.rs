use std::io::{Error, ErrorKind};
use tokio::sync::mpsc;

use integration::components::websocket::binance::BinanceTickerBatch;
use schemas::Tick;
use vector_config_macro::transform;
use vector_runtime::{Component, Identify, Message, Outbound};

#[transform(derive(PartialEq))]
pub struct TransformBinanceTradeToTick {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_transform_binance_trade_to_tick!(
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        while let Some(message) = rx.recv().await {
            let bytes = message.payload.to_string().into_bytes();

            let batch = match serde_json::from_slice::<BinanceTickerBatch>(&bytes) {
                Ok(data) => data,
                Err(err) => {
                    eprintln!(
                        "Transform [{id}/{}] failed to deserialize BinanceTickerBatch: {}",
                        self.id, err
                    );
                    continue;
                }
            };

            for ticker in batch.data {
                let price: f64 = match ticker.last_price.parse() {
                    Ok(val) => val,
                    Err(_) => {
                        eprintln!(
                            "Transform [{id}/{}] failed to parse price: {}",
                            self.id, ticker.last_price
                        );
                        continue;
                    }
                };

                let tick = Tick {
                    broker: "binance".to_string(),
                    symbol: ticker.symbol.clone(),
                    price,
                    timestamp: ticker.event_time,
                    ..Default::default()
                };

                if let Ok(payload) = serde_json::to_value(&tick) {
                    for stream in &tx.streams {
                        if let Err(error) = stream
                            .send(Message {
                                payload: payload.clone(),
                            })
                            .await
                        {
                            return Err(Error::new(
                                ErrorKind::BrokenPipe,
                                format!("Failed to send binance tick for {}: {error}", tick.symbol),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
);
