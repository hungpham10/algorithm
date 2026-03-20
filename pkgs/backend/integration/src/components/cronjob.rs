use tokio::sync::mpsc;
use vector_config_macro::source;

use vector_runtime::{Component, Event, Identify, Message};

#[source(derive(PartialEq, Clone))]
pub struct CronjobSource {
    pub id: String,
}

impl_cronjob_source!(
    async fn run(
        &self,
        _: usize,
        _: &mut mpsc::Receiver<Message>,
        _: &Vec<mpsc::Sender<Message>>,
        _: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        Ok(())
    }
);
