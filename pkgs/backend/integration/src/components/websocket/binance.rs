use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use vector_config_macro::source;
use vector_runtime::{Component, Identify, Message as VectorMessage, Outbound};

use crate::components::websocket::{WebSocketClient, WebSocketPolling};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BinanceTrade {
    #[serde(rename = "e")]
    pub event_type: String, // "trade"

    #[serde(rename = "E")]
    pub event_time: i64, // Event timestamp

    #[serde(rename = "s")]
    pub symbol: String, // Ví dụ: BTCUSDT

    #[serde(rename = "t")]
    pub trade_id: u64, // Trade ID

    #[serde(rename = "p")]
    pub price: String, // Giá khớp lệnh

    #[serde(rename = "q")]
    pub quantity: String, // Số lượng khớp

    #[serde(rename = "T")]
    pub trade_time: u64, // Thời gian khớp lệnh chính xác

    #[serde(rename = "m")]
    pub is_buyer_market_maker: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BinanceTradeBatch {
    pub timestamp: i64,
    pub data: BinanceTrade,
}

#[source]
pub struct BinanceSource {
    pub id: String,
    pub symbols: Vec<String>,
}

impl Clone for BinanceSource {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            symbols: self.symbols.clone(),
        }
    }
}

impl PartialEq for BinanceSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.symbols == other.symbols
    }
}

#[async_trait]
impl WebSocketPolling for BinanceSource {
    async fn on_start(&self) -> Result<Option<String>, Error> {
        let streams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect();

        let subscribe_payload = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        });

        Ok(Some(subscribe_payload.to_string()))
    }

    async fn on_send(&self) -> Result<Option<String>, Error> {
        Ok(None)
    }

    async fn on_receive(
        &self,
        message: WsMessage,
        txs: &[mpsc::Sender<VectorMessage>],
    ) -> Result<(), Error> {
        match message {
            WsMessage::Text(text) => {
                if text.contains("\"result\"") {
                    return Ok(());
                }

                match serde_json::from_str::<BinanceTrade>(&text) {
                    Ok(trade) => {
                        let batch = BinanceTradeBatch {
                            timestamp: trade.event_time,
                            data: trade,
                        };

                        let payload = serde_json::to_value(batch)
                            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

                        for tx in txs {
                            let _ = tx
                                .send(VectorMessage {
                                    payload: payload.clone(),
                                })
                                .await;
                        }

                        Ok(())
                    }
                    Err(e) => Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Failed to parse Binance Trade raw event: {}. Raw text: {}",
                            e, text
                        ),
                    )),
                }
            }
            WsMessage::Binary(_) | WsMessage::Ping(_) | WsMessage::Pong(_) => Ok(()),
            _ => Ok(()),
        }
    }
}

impl_binance_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<VectorMessage>,
        tx: Outbound,
    ) -> Result<(), std::io::Error> {
        WebSocketClient::new("wss://stream.binance.us:9443/ws".to_string(), 1)
            .run(self.clone(), id, &tx)
            .await
    }
);
