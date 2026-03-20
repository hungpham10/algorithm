use tokio::sync::mpsc;
use vector_config_macro::sink;

use vector_runtime::{Component, Event, Identify, Message};

#[sink(derive(PartialEq))]
pub struct Print {
    pub id: String,
    pub inputs: Vec<String>,

    #[serde(default = "default_prefix")]
    pub prefix: String,
}

fn default_prefix() -> String {
    "PRINT-SINK".to_string()
}

impl_print!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        _: &Vec<mpsc::Sender<Message>>,
        _: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        while let Some(message) = rx.recv().await {
            println!("{}: Received data: {}", self.prefix, message.payload);
        }

        Ok(())
    }
);
