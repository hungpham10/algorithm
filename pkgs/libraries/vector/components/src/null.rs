use tokio::sync::mpsc;
use vector_config_macro::sink;
use vector_runtime::{Component, Identify, Message, Outbound};

#[sink(derive(PartialEq, Default))]
pub struct Null {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_null!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        _: Outbound,
    ) -> Result<(), std::io::Error> {
        while rx.recv().await.is_some() {}
        Ok(())
    }
);
