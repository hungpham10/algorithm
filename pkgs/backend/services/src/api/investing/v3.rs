use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};

use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::channel;
use utoipa::{OpenApi, ToSchema};

use crate::api::{AppState, investing::InvestingHeaders};
use schemas::{CandleStick, Tick};
use vector_runtime::Message as VectorMessage;

#[derive(OpenApi)]
#[openapi(
    paths(into_websocket),
    components(schemas(SymbolInDetail, OhclRequest, OhclResponse, CandleStick,))
)]
pub struct InvestingV3Socket;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct SymbolInDetail {
    #[schema(example = "binance")]
    broker: String,

    #[schema(example = "BTCUSDT")]
    symbol: String,
}

#[derive(Deserialize, ToSchema, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
enum OhclRequest {
    Subscribe {
        #[schema(value_type = Vec<SymbolInDetail>, example = json!([{"broker": "binance", "symbol": "BTCUSDT"}]))]
        symbols: Vec<SymbolInDetail>,
    },
    Unsubscribe {
        #[schema(value_type = Vec<SymbolInDetail>, example = json!([{"broker": "binance", "symbol": "BTCUSDT"}]))]
        symbols: Vec<SymbolInDetail>,
    },
    Ping,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OhclResponse {
    Subscribed {
        #[schema(value_type = Vec<SymbolInDetail>, example = json!([{"broker": "binance", "symbol": "BTCUSDT"}]))]
        symbols: Vec<SymbolInDetail>,
    },
    Tick {
        #[serde(flatten)]
        data: Tick,
    },
    Error {
        #[schema(example = "Invalid symbol format")]
        message: String,
    },
    Pong,
}

#[utoipa::path(
    get,
    path = "/v3",
    tag = "WebSocket Gateway for Investing",
    summary = "Connect to Gateway Websocket",
    description = "Upgrade data to Websocket and stream data to process flow"
)]
pub async fn into_websocket(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let broadcast = app_state
        .runtime
        .broadcast(
            app_state
                .secret
                .get("MARKET_PIPELINE_OUTPUT", "/")
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(OhclResponse::Error {
                            message: "MARKET_PIPELINE_OUTPUT not set".into(),
                        }),
                    )
                })?,
        )
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OhclResponse::Error {
                    message: format!("Failed fetching broadcaster: {error}"),
                }),
            )
        })?;

    Ok(ws
        .on_failed_upgrade(|err| {
            tracing::error!("WebSocket upgrade failed: {:?}", err);
        })
        .on_upgrade(move |socket| handle_socket(app_state, socket, tenant_id.into(), broadcast)))
}

enum ControlFlow {
    Close,
    Reply(OhclResponse),
}

struct SessionState {
    subscribed_symbols: HashSet<(String, String)>,
}

async fn handle_socket(
    app_state: AppState,
    socket: WebSocket,
    tenant_id: i64,
    mut broadcast: Receiver<VectorMessage>,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = channel::<ControlFlow>(100);

    let session = Arc::new(RwLock::new(SessionState {
        subscribed_symbols: HashSet::new(),
    }));

    let mut recv_task = {
        let session = session.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    WsMessage::Text(text) => {
                        if let Ok(req) = serde_json::from_str::<OhclRequest>(&text) {
                            match req {
                                OhclRequest::Ping => {
                                    let _ = tx.send(ControlFlow::Reply(OhclResponse::Pong)).await;
                                }
                                OhclRequest::Subscribe { symbols } => {
                                    for item in &symbols {
                                        let is_enabled = app_state
                                            .investing_entity
                                            .is_broker_enabled(tenant_id, &item.broker)
                                            .await;

                                        match is_enabled {
                                            Ok(true) => {
                                                let mut state = session.write().await;

                                                state.subscribed_symbols.insert((
                                                    item.broker.clone(),
                                                    item.symbol.clone(),
                                                ));
                                            }
                                            Ok(false) => {
                                                let _ = tx
                                                    .send(ControlFlow::Reply(OhclResponse::Error {
                                                        message: format!(
                                                            "Broker '{}' is not enabled.",
                                                            item.broker
                                                        ),
                                                    }))
                                                    .await;
                                            }
                                            Err(error) => {
                                                let _ = tx
                                                    .send(ControlFlow::Reply(OhclResponse::Error {
                                                        message: format!(
                                                            "Internal error: {}",
                                                            error
                                                        ),
                                                    }))
                                                    .await;
                                            }
                                        }
                                    }

                                    let _ = tx
                                        .send(ControlFlow::Reply(OhclResponse::Subscribed {
                                            symbols: symbols.clone(),
                                        }))
                                        .await;
                                }
                                OhclRequest::Unsubscribe { symbols } => {
                                    let mut state = session.write().await;

                                    for item in symbols {
                                        state
                                            .subscribed_symbols
                                            .remove(&(item.broker, item.symbol));
                                    }
                                }
                            }
                        }
                    }
                    WsMessage::Close(_) => {
                        let _ = tx.send(ControlFlow::Close).await;
                        break;
                    }
                    _ => {}
                }
            }
        })
    };

    let mut send_task = tokio::spawn(async move {
        while let Some(control) = rx.recv().await {
            match control {
                ControlFlow::Reply(response) => {
                    if let Ok(json_str) = serde_json::to_string(&response)
                        && sender.send(WsMessage::Text(json_str.into())).await.is_err()
                    {
                        break;
                    }
                }
                ControlFlow::Close => {
                    break;
                }
            }
        }
    });

    let mut broadcast_task = {
        let session = session.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            loop {
                match broadcast.recv().await {
                    Ok(msg) => {
                        if let Ok(tick_data) = serde_json::from_value::<Tick>(msg.payload) {
                            let is_subscribed = {
                                let state = session.read().await; // <--- THÊM .await Ở ĐÂY
                                state
                                    .subscribed_symbols
                                    .contains(&(tick_data.broker.clone(), tick_data.symbol.clone()))
                            };

                            if is_subscribed {
                                let response = OhclResponse::Tick { data: tick_data };
                                if tx.send(ControlFlow::Reply(response)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "Client tennant {} bị chậm! Đã bỏ qua {} ticks",
                            tenant_id,
                            skipped
                        );
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
        })
    };

    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
            broadcast_task.abort();
        },
        _ = &mut recv_task => {
            send_task.abort();
            broadcast_task.abort();
        },
        _ = &mut broadcast_task => {
            send_task.abort();
            recv_task.abort();
        },
    }
}
