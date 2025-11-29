use std::collections::BTreeMap;
use std::net::TcpStream;

use actix::prelude::*;

use serde::{Deserialize, Serialize};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Bytes, Message, WebSocket};

use crate::actors::ActorError;

struct State {
    future: i32,
}

pub struct DragonActor {
    symbols: BTreeMap<String, Row>,
    future: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    state: State,
}

pub struct Row {
    pub price_plus1: f64,
    pub price_plus2: f64,
    pub price_plus3: f64,
    pub volume_plus1: f64,
    pub volume_plus2: f64,
    pub volume_plus3: f64,
    pub price_minus1: f64,
    pub price_minus2: f64,
    pub price_minus3: f64,
    pub volume_minus1: f64,
    pub volume_minus2: f64,
    pub volume_minus3: f64,
    pub buy_wait: f64,
    pub sell_wait: f64,
}

impl DragonActor {
    pub fn new() -> Result<Self, ActorError> {
        let (future, _) =
            connect("wss://livedragon.vdsc.com.vn/foswss").map_err(|error| ActorError {
                message: format!("Failed to connect to future vdsc websocket: {}", error),
            })?;

        Ok(Self {
            symbols: BTreeMap::new(),
            future: Some(future),
            state: State { future: 0 },
        })
    }

    fn reconnect(&mut self) -> Result<(), ActorError> {
        let (future, _) =
            connect("wss://livedragon.vdsc.com.vn/foswss").map_err(|error| ActorError {
                message: format!("Failed to connect to future vdsc websocket: {}", error),
            })?;

        self.future = Some(future);
        Ok(())
    }

    fn process_fos_response_from_vdsc(
        &mut self,
        resp: &FosWebSocketResponse,
    ) -> Result<(), ActorError> {
        if let Some(list) = &resp.list {
            for item in list {
                if let Some(symbol) = item[0].get(0..9) {
                    let index = match item[0].get(9..) {
                        Some(id) => match id.parse::<usize>() {
                            Ok(id) => id,
                            Err(_) => 0,
                        },
                        None => 0,
                    };

                    let row = self
                        .symbols
                        .entry(symbol.to_string())
                        .or_insert_with(|| Row {
                            price_plus1: 0.0,
                            volume_plus1: 0.0,
                            price_plus2: 0.0,
                            volume_plus2: 0.0,
                            price_plus3: 0.0,
                            volume_plus3: 0.0,
                            price_minus1: 0.0,
                            volume_minus1: 0.0,
                            price_minus2: 0.0,
                            volume_minus2: 0.0,
                            price_minus3: 0.0,
                            volume_minus3: 0.0,
                            buy_wait: 0.0,
                            sell_wait: 0.0,
                        });

                    match index {
                        // @NOTE: over buy
                        4 => {
                            row.price_minus3 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        5 => {
                            row.volume_minus3 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        6 => {
                            row.price_minus2 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        7 => {
                            row.volume_minus2 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        8 => {
                            row.price_minus1 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        9 => {
                            row.volume_minus1 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }

                        // @NOTE: over sell
                        16 => {
                            row.price_plus1 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        17 => {
                            row.volume_plus1 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        18 => {
                            row.price_plus2 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        19 => {
                            row.volume_plus2 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        20 => {
                            row.price_plus3 = item[1]
                                .trim()
                                .replace(",", "")
                                .parse::<f64>()
                                .map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }
                        21 => {
                            row.volume_plus3 =
                                item[1].parse::<f64>().map_err(|error| ActorError {
                                    message: format!("Failed to parse: {}", error),
                                })?
                        }

                        // @NOTE: Volume in waiting
                        26 => {
                            row.buy_wait = item[1].parse::<f64>().map_err(|error| ActorError {
                                message: format!("Failed to parse: {}", error),
                            })?
                        }
                        27 => {
                            row.sell_wait = item[1].parse::<f64>().map_err(|error| ActorError {
                                message: format!("Failed to parse: {}", error),
                            })?
                        }

                        _ => {}
                    };
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl Actor for DragonActor {
    type Context = Context<Self>;
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), ActorError>")]
struct ReconnectCommand;

impl Handler<ReconnectCommand> for DragonActor {
    type Result = ResponseFuture<Result<(), ActorError>>;

    fn handle(&mut self, _: ReconnectCommand, _: &mut Self::Context) -> Self::Result {
        match self.reconnect() {
            Ok(_) => Box::pin(async move { Ok(()) }),
            Err(error) => Box::pin(async move {
                Err(ActorError {
                    message: format!("Failed reconnect to vdsc: {}", error),
                })
            }),
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), ActorError>")]
struct FetchFutureWebsocketCommand;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FosWebSocketRequest {
    #[serde(rename = "type")]
    client_type: String,

    #[serde(rename = "clientVersion")]
    client_version: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FosWebSocketResponse {
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

impl Handler<FetchFutureWebsocketCommand> for DragonActor {
    type Result = ResponseFuture<Result<(), ActorError>>;

    fn handle(&mut self, _: FetchFutureWebsocketCommand, _: &mut Self::Context) -> Self::Result {
        if let Some(socket) = self.future.as_mut() {
            if let Err(error) = socket.send(Message::Text(
                serde_json::to_string(&FosWebSocketRequest {
                    client_type: "fosboard".to_string(),
                    client_version: self.state.future,
                })
                .unwrap()
                .into(),
            )) {
                return Box::pin(async move {
                    Err(ActorError {
                        message: format!("Fail to send tick request to server: {}", error),
                    })
                });
            }

            match socket.read() {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<FosWebSocketResponse>(&text) {
                        Ok(response) => {
                            if self.state.future != response.server_version {
                                if let Err(error) = self.process_fos_response_from_vdsc(&response) {
                                    return Box::pin(async move {
                                        Err(ActorError {
                                            message: format!(
                                                "Fail to update data to memory: {}",
                                                error
                                            ),
                                        })
                                    });
                                }
                            }

                            self.state.future = response.server_version;
                        }
                        Err(error) => {
                            return Box::pin(async move {
                                Err(ActorError {
                                    message: format!(
                                        "Fail to parse response from server: {}",
                                        error
                                    ),
                                })
                            });
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    if let Err(error) = socket.send(Message::Pong(Bytes::new())) {
                        return Box::pin(async move {
                            Err(ActorError {
                                message: format!("Fail to send pong to server: {}", error),
                            })
                        });
                    }
                }
                Ok(Message::Close(_)) => {
                    // @NOTE: reconnect
                }
                Ok(Message::Binary(_)) | Ok(Message::Frame(_)) => {}
                Ok(_) => {}
                Err(error) => {
                    return Box::pin(async move {
                        Err(ActorError {
                            message: format!("WebSocket is closed duo to issue: {}", error),
                        })
                    });
                }
            }

            return Box::pin(async move { Ok(()) });
        } else {
            return Box::pin(async move {
                Err(ActorError {
                    message: format!("Failed to read 3rd API socket, please open it first"),
                })
            });
        }
    }
}
