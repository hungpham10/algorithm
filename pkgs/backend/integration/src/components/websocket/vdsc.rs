use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use vector_config_macro::source;
use vector_runtime::{Component, Event, Identify, Message as VectorMessage};

use crate::components::websocket::{WebSocketClient, WebSocketPolling};

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum VdscBoard {
    Unknown,
    Future,
    Stock,
}

impl From<i32> for VdscBoard {
    fn from(value: i32) -> Self {
        match value {
            1 => VdscBoard::Future,
            2 => VdscBoard::Stock,
            _ => VdscBoard::Unknown,
        }
    }
}

impl From<String> for VdscBoard {
    fn from(value: String) -> Self {
        match value.as_str() {
            "fos" => VdscBoard::Future,
            "mix" => VdscBoard::Stock,
            _ => VdscBoard::Unknown,
        }
    }
}

impl Display for VdscBoard {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            VdscBoard::Unknown => write!(f, "Unknown"),
            VdscBoard::Future => write!(f, "fos"),
            VdscBoard::Stock => write!(f, "mix"),
        }
    }
}

impl<'de> Deserialize<'de> for VdscBoard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(VdscBoard::from(s))
    }
}

impl serde::Serialize for VdscBoard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VdscWebSocketRequest {
    #[serde(rename = "type")]
    client_type: String,

    #[serde(rename = "clientVersion")]
    client_version: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VdscWebSocketResponse {
    success: bool,
    time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    list: Option<Vec<Vec<String>>>,

    #[serde(rename = "isReload")]
    is_reload: bool,

    #[serde(rename = "serverVersion")]
    server_version: i32,

    #[serde(rename = "responseVersion")]
    response_version: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Transition {
    pub index: usize,
    pub change: f64,
}

#[source]
pub struct VdscSource {
    pub id: String,
    pub board: VdscBoard,

    #[serde(skip)]
    pub current_version: Arc<AtomicI32>,
}

impl PartialEq for VdscSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.board == other.board
    }
}

impl Clone for VdscSource {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            board: self.board.clone(),
            current_version: Arc::clone(&self.current_version),
        }
    }
}

impl VdscSource {
    fn process_future_from_vdsc(
        &self,
        resp: &VdscWebSocketResponse,
    ) -> Result<BTreeMap<String, Vec<Transition>>, Error> {
        let mut symbols = BTreeMap::new();

        if let Some(list) = &resp.list {
            for item in list {
                if let Some(symbol) = item[0].get(0..9) {
                    let index = match item[0].get(9..) {
                        Some(id) => id.parse::<usize>().unwrap_or_default(),
                        None => 0,
                    };

                    let row = symbols.entry(symbol.to_string()).or_insert_with(Vec::new);

                    let cleaned = item[1]
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                        .collect::<String>();

                    let change = if cleaned.is_empty() {
                        0.0
                    } else {
                        cleaned.parse::<f64>().map_err(|e| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Cannot parse `{}` (after cleaning: `{}`) at index {}: {}",
                                    item[1], cleaned, index, e
                                ),
                            )
                        })?
                    };
                    row.push(Transition { index, change });

                    // match index {
                    //     4 | 5 | 6 | 7 | 8 | 9 | 12 | 13 | 16 | 17 | 18 | 19 | 20 | 21 | 26 | 27 => {
                    //         row.push(Transition { index, change });
                    //     }
                    //     _ => {}
                    // };
                }
            }
        }

        Ok(symbols)
    }

    fn process_stock_from_vdsc(
        &self,
        resp: &VdscWebSocketResponse,
    ) -> Result<BTreeMap<String, Vec<Transition>>, Error> {
        let mut symbols = BTreeMap::new();

        if let Some(list) = &resp.list {
            for item in list {
                if item.len() == 3 {
                    let symbol = if item[0].len() < 8 {
                        item[0][0..3].to_string()
                    } else {
                        item[0][0..8].to_string()
                    };

                    let index = if item[0].len() < 8 {
                        item[0][3..].parse::<usize>().map_err(|error| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!("Failed to parse {}: {}", item[0], error),
                            )
                        })?
                    } else {
                        item[0][8..].parse::<usize>().map_err(|error| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!("Failed to parse {}: {}", item[0], error),
                            )
                        })?
                    };

                    let row = symbols.entry(symbol.to_string()).or_insert_with(Vec::new);

                    let cleaned = item[1]
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                        .collect::<String>();

                    let change = if cleaned.is_empty() {
                        0.0
                    } else {
                        cleaned.parse::<f64>().map_err(|e| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Cannot parse `{}` (after cleaning: `{}`) at index {}: {}",
                                    item[1], cleaned, index, e
                                ),
                            )
                        })?
                    };

                    row.push(Transition { index, change });

                    // match index {
                    //     4 | 5 | 6 | 7 | 8 | 9 | 12 | 13 | 16 | 17 | 18 | 19 | 20 | 21 | 26 | 27 => {
                    //         row.push(Transition { index, change });
                    //     }
                    //     _ => {}
                    // };
                }
            }
        }

        Ok(symbols)
    }
}

#[async_trait]
impl<'life2> WebSocketPolling<'life2> for VdscSource {
    async fn on_send(&self) -> Result<Option<String>, Error> {
        let current_version = self.current_version.load(Ordering::SeqCst);

        let request = VdscWebSocketRequest {
            client_type: format!("{}board", self.board),
            client_version: current_version,
        };

        let json = serde_json::to_string(&request).map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Fail to serialize init request: {}", error),
            )
        })?;

        Ok(Some(json))
    }

    async fn on_receive(
        &self,
        message: WsMessage,
        txs: &'life2 [mpsc::Sender<VectorMessage>],
    ) -> Result<(), Error> {
        match message {
            WsMessage::Text(text) => match serde_json::from_str::<VdscWebSocketResponse>(&text) {
                Ok(response) => {
                    let old_version = self.current_version.load(Ordering::SeqCst);

                    if old_version != response.server_version {
                        let transitions = match self.board {
                            VdscBoard::Future => self.process_future_from_vdsc(&response)?,
                            VdscBoard::Stock => self.process_stock_from_vdsc(&response).unwrap(),
                            _ => return Ok(()),
                        };

                        for (symbol, transition) in transitions {
                            if transition.is_empty() {
                                continue;
                            }

                            for tx in txs {
                                tx.send(VectorMessage {
                                    payload: json!({
                                        "symbol": symbol,
                                        "transition": serde_json::to_value(&transition).map_err(
                                            |error| {
                                                Error::new(
                                                    ErrorKind::InvalidData,
                                                    format!("Failed to build payload: {}", error),
                                                )
                                            },
                                        )?,
                                    }),
                                })
                                .await
                                .map_err(|error| {
                                    Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("Failed to send data {:?}: {}", transition, error),
                                    )
                                })?;
                            }
                        }

                        self.current_version
                            .store(response.server_version, Ordering::SeqCst);
                    }
                    Ok(())
                }
                Err(error) => Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Failed to when parsing {}: {}", text, error,),
                )),
            },
            _ => Ok(()),
        }
    }
}

impl_vdsc_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<VectorMessage>,
        txs: &'life2 [mpsc::Sender<VectorMessage>],
        err: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        WebSocketClient::new(format!("wss://livedragon.vdsc.com.vn/{}wss", self.board), 5)
            .run(self.clone(), id, txs, err)
            .await
    }
);
