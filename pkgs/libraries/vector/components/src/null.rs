use tokio::sync::mpsc;
use vector_config_macro::sink;
use vector_runtime::{Component, Event, Identify, Message};

#[sink(derive(PartialEq))]
pub struct Null {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_null!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        _: &Vec<mpsc::Sender<Message>>,
        _: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        while let Some(_) = rx.recv().await {}

        Ok(())
    }
);
