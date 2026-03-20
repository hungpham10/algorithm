use std::io::{Error, ErrorKind};
use tokio::sync::mpsc;
use vector_config_macro::source;
use vector_runtime::{Component, Event, Identify, Message};

#[source(derive(PartialEq))]
pub struct Input {
    pub id: String,
}

impl_input!(
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        txs: &'life2 [mpsc::Sender<Message>],
        err: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        while let Some(msg) = rx.recv().await {
            let mut failed = true;

            for tx in txs {
                if let Err(error) = tx.send(msg.clone()).await {
                    err.send(Event::Minor((
                        id,
                        Error::other(format!(
                            "Failed to send data to one specific output: {error}"
                        )),
                    )))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to report error: {error}"),
                        )
                    })?;
                } else {
                    failed = false;
                }
            }

            if failed {
                err.send(Event::Major((
                    id,
                    Error::other("Failed to send data to every node"),
                )))
                .await
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to report error: {error}"),
                    )
                })?;
            }
        }

        Ok(())
    }
);
