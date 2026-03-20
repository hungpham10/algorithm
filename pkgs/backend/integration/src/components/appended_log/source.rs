use std::io::{Error, ErrorKind};

use serde_json::Value;

use tokio::sync::mpsc;
use tokio::time::sleep;

use vector_config_macro::source;
use vector_runtime::{Component, Event, Identify, Message};

use super::core::AppendedLog;

#[source(derive(Clone, PartialEq))]
pub struct AppendedLogSource {
    dsn: String,
    id: String,

    #[serde(default)]
    offset: u64,
}

impl_appended_log_source!(
    async fn run(
        &self,
        id: usize,
        _: &mut mpsc::Receiver<Message>,
        txs: &Vec<mpsc::Sender<Message>>,
        err: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        let logger = AppendedLog::new(&self.dsn).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Source {} failed to connect: {}", self.id, e),
            )
        })?;

        let mut current_offset = self.offset;

        loop {
            let log_entries_result = logger.read_log_stream(current_offset);

            match log_entries_result {
                Ok(log_entries) => {
                    let mut read_any = false;

                    for (data, next_offset) in log_entries {
                        read_any = true;
                        let payload_str = String::from_utf8_lossy(&data).into_owned();

                        let message = Message {
                            payload: Value::from(payload_str),
                        };

                        for tx in txs {
                            if let Err(error) = tx.send(message.clone()).await {
                                let _ = err
                                    .send(Event::Minor((
                                        id,
                                        Error::new(
                                            ErrorKind::Other,
                                            format!("Failed to send event: {}", error),
                                        ),
                                    )))
                                    .await;
                            }
                        }

                        current_offset = next_offset;
                    }

                    if !read_any {
                        sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
                Err(e) => {
                    let _ = err
                        .send(Event::Minor((
                            id,
                            Error::new(
                                ErrorKind::Other,
                                format!("Read error at offset {}: {}", current_offset, e),
                            ),
                        )))
                        .await;

                    sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
);
