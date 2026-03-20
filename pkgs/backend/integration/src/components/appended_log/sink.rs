use std::io::{Error, ErrorKind};
use tokio::sync::mpsc;

use vector_config_macro::sink;
use vector_runtime::{Component, Event, Identify, Message};

use super::core::AppendedLog;

#[sink(derive(Clone, PartialEq))]
pub struct AppendedLogSink {
    dsn: String,
    id: String,
    inputs: Vec<String>,
}

impl_appended_log_sink!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        _: &'life2 [mpsc::Sender<Message>],
        _: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        let logger = AppendedLog::new(&self.dsn).map_err(|e| {
            Error::other(format!(
                "Failed to initialize SFTP connection for sink {}: {}",
                self.id, e
            ))
        })?;

        while let Some(message) = rx.recv().await {
            let data = message.payload.to_string().into_bytes();

            if data.is_empty() {
                continue;
            }

            if let Err(e) = logger.write_log_stream(data.as_slice()) {
                return Err(Error::new(
                    ErrorKind::WriteZero,
                    format!("Sink {} failed to write message: {}", self.id, e),
                ));
            }
        }

        Ok(())
    }
);
