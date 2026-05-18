use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use vector_config_macro::source;
use vector_runtime::{Component, Identify, Message as VectorMessage, Outbound};

use crate::components::websocket::{WebSocketClient, WebSocketPolling};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BinanceTicker {
    #[serde(rename = "e")]
    pub event_type: String, // "1hTicker" hoặc "24hrTicker"

    #[serde(rename = "E")]
    pub event_time: i64,

    #[serde(rename = "s")]
    pub symbol: String, // "BNBBTC"

    #[serde(rename = "p")]
    pub price_change: String,

    #[serde(rename = "P")]
    pub price_change_percent: String,

    #[serde(rename = "o")]
    pub open_price: String,

    #[serde(rename = "h")]
    pub high_price: String,

    #[serde(rename = "l")]
    pub low_price: String,

    #[serde(rename = "c")]
    pub last_price: String,

    #[serde(rename = "w")]
    pub weighted_avg_price: String,

    #[serde(rename = "v")]
    pub total_traded_base_asset_volume: String,

    #[serde(rename = "q")]
    pub total_traded_quote_asset_volume: String,

    #[serde(rename = "O")]
    pub statistics_open_time: i64,

    #[serde(rename = "C")]
    pub statistics_close_time: i64,

    #[serde(rename = "F")]
    pub first_trade_id: i64,

    #[serde(rename = "L")]
    pub last_trade_id: i64,

    #[serde(rename = "n")]
    pub total_number_of_trades: u64,
}

// Cấu trúc Batch mới: chứa toàn bộ mảng data thay vì một object đơn lẻ
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BinanceTickerBatch {
    pub timestamp: i64,
    pub data: Vec<BinanceTicker>, // Gửi nguyên một mảng để tiết kiệm băng thông
}

#[source]
pub struct BinanceSource {
    pub id: String,
}

impl Clone for BinanceSource {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
        }
    }
}

impl PartialEq for BinanceSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[async_trait]
impl WebSocketPolling for BinanceSource {
    async fn on_start(&self) -> Result<Option<String>, Error> {
        Ok(None)
    }

    async fn on_send(&self) -> Result<Option<String>, Error> {
        Ok(None)
    }

    async fn on_receive(
        &self,
        message: WsMessage,
        txs: &[mpsc::Sender<VectorMessage>],
    ) -> Result<(), Error> {
        if let WsMessage::Text(text) = message {
            // 1. Parse toàn bộ mảng dữ liệu từ WebSocket
            if let Ok(tickers) = serde_json::from_str::<Vec<BinanceTicker>>(&text) {
                if tickers.is_empty() {
                    return Ok(());
                }

                // Lấy timestamp của phần tử đầu tiên làm đại diện cho cả Batch
                let batch_timestamp = tickers[0].event_time;

                // 2. Đóng gói toàn bộ danh sách tickers vào một cấu trúc Batch duy nhất
                let batch = BinanceTickerBatch {
                    timestamp: batch_timestamp,
                    data: tickers,
                };

                // 3. Serialize một lần duy nhất
                let payload = serde_json::to_value(batch)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

                // 4. Gửi một block payload duy nhất qua kênh truyền
                for tx in txs {
                    let _ = tx
                        .send(VectorMessage {
                            payload: payload.clone(),
                        })
                        .await;
                }
            }
        }
        Ok(())
    }
}

impl_binance_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<VectorMessage>,
        tx: Outbound,
    ) -> Result<(), std::io::Error> {
        WebSocketClient::new(
            "wss://stream.binance.us:9443/ws/!ticker_1h@arr".to_string(),
            1,
        )
        .run(self.clone(), id, &tx)
        .await
    }
);
