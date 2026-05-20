use std::io::Error;

use tokio::sync::mpsc;
use vector_config_macro::output;
use vector_runtime::{Component, Identify, Message, Outbound};

#[output(derive(PartialEq))]
pub struct Output {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_output!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        while let Some(msg) = rx.recv().await {
            if let Some(ref broadcast) = tx.broadcast {
                let _ = broadcast.send(msg);
            }
        }
        Ok(())
    }
);
