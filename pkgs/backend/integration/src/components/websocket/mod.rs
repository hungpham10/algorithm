use std::io::{Error, ErrorKind};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;

use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

use vector_runtime::{Event, Message as VectorMessage};

#[cfg(feature = "vdsc")]
pub mod vdsc;

#[async_trait]
pub trait WebSocketPolling<'life2> {
    async fn on_send(&self) -> Result<Option<String>, Error>;
    async fn on_receive(
        &self,
        message: WsMessage,
        txs: &'life2 [mpsc::Sender<VectorMessage>],
    ) -> Result<(), Error>;
}

struct WebSocketClient {
    pub url: String,
    pub reconnect_interval_sec: u64,
}

impl<'life2> WebSocketClient {
    fn new(url: String, reconnect_interval_sec: u64) -> Self {
        Self {
            url,
            reconnect_interval_sec,
        }
    }

    async fn run<Handler: WebSocketPolling<'life2> + Send + Sync + 'static>(
        &self,
        handler: Handler,
        id: usize,
        txs: &'life2 [mpsc::Sender<VectorMessage>],
        err: &mpsc::Sender<Event>,
    ) -> Result<(), Error> {
        loop {
            match connect_async(&self.url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    if let Ok(Some(msg)) = handler.on_send().await {
                        use futures_util::SinkExt;

                        if let Err(error) = write.send(WsMessage::Text(msg.into())).await {
                            err.send(Event::Minor((
                                id,
                                Error::other(format!("Failed to send msg to websocket: {}", error)),
                            )))
                            .await
                            .map_err(|error| {
                                Error::new(
                                    ErrorKind::BrokenPipe,
                                    format!("Failed to send issue: {}", error,),
                                )
                            })?;
                            continue;
                        }
                    }

                    while let Some(message) = read.next().await {
                        match message {
                            Ok(msg) => {
                                if let Err(error) = handler.on_receive(msg, txs).await {
                                    err.send(Event::Minor((id, error))).await.map_err(|error| {
                                        Error::new(
                                            ErrorKind::BrokenPipe,
                                            format!("Failed to send issue: {}", error,),
                                        )
                                    })?;
                                }
                            }
                            Err(error) => {
                                err.send(Event::Minor((
                                    id,
                                    Error::other(format!(
                                        "Failed to read data from websocket: {}",
                                        error
                                    )),
                                )))
                                .await
                                .map_err(|error| {
                                    Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("Failed to send issue: {}", error,),
                                    )
                                })?;
                                break;
                            }
                        }

                        if let Ok(Some(msg)) = handler.on_send().await {
                            use futures_util::SinkExt;

                            if let Err(error) = write.send(WsMessage::Text(msg.into())).await {
                                err.send(Event::Minor((
                                    id,
                                    Error::other(format!(
                                        "Failed sending msg to websocket: {}",
                                        error,
                                    )),
                                )))
                                .await
                                .map_err(|error| {
                                    Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("Failed to send issue: {}", error,),
                                    )
                                })?;
                                break;
                            }

                            sleep(Duration::from_secs(self.reconnect_interval_sec)).await;
                        }
                    }
                }
                Err(error) => {
                    err.send(Event::Minor((
                        id,
                        Error::other(format!("Failed when setup websocket connection: {}", error)),
                    )))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to send issue: {}", error,),
                        )
                    })?;
                }
            }
        }
    }
}
