use std::io::{Error, ErrorKind};

use tokio::sync::mpsc;
use vector_config_macro::output;
use vector_runtime::{Component, Event, Identify, Message, Outbound};

#[output(derive(PartialEq))]
pub struct PriceGatherOutput {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_price_gather_output!(
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        while let Some(msg) = rx.recv().await {
            if let Some(ref broadcast) = tx.broadcast
                && let Err(error) = broadcast.send(msg)
            {
                tx.event
                    .send(Event::Minor((
                        id,
                        Error::other(format!("Failed to send msg to boardcast: {error}")),
                    )))
                    .await
                    .map_err(|error| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to send issue: {error}"),
                        )
                    })?;
            }
        }
        Ok(())
    }
);
