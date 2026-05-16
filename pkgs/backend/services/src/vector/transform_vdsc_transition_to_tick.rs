use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};
use tokio::sync::mpsc;

use schemas::Tick;
use integration::components::websocket::vdsc::Transition;
use vector_config_macro::transform;
use vector_runtime::{Component, Identify, Message, Outbound};

#[transform(derive(PartialEq))]
pub struct TransformVdscTransitionToTick {
    pub id: String,
    pub inputs: Vec<String>,
}

impl_transform_vdsc_transition_to_tick!(
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        while let Some(message) = rx.recv().await {
            let bytes = message.payload.to_string().into_bytes();

            let transitions =
                match serde_json::from_slice::<BTreeMap<String, Vec<Transition>>>(&bytes) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!(
                            "Transform [{id}/{}] failed to deserialize BTreeMap: {}",
                            self.id, err
                        );
                        continue;
                    }
                };

            for (symbol, transition_list) in transitions {
                if transition_list.is_empty() {
                    continue;
                }

                let mut price = None;
                let mut quantity = None;

                // @TODO: cần xem lại đoạn này
                for trans in transition_list {
                    match trans.index {
                        11 => price = Some(trans.change),
                        12 => quantity = Some(trans.change),
                        _ => {}
                    }

                    if price.is_some() || quantity.is_some() {
                        let tick = Tick {
                            broker: "vdsc".to_string(),
                            symbol: symbol.clone(),
                            price: price.unwrap_or(0.0),
                            quantity: quantity.unwrap_or(0.0),
                            ..Default::default()
                        };

                        if let Ok(payload) = serde_json::to_value(&tick) {
                            for stream in &tx.streams {
                                if let Err(error) = stream
                                    .send(Message {
                                        payload: payload.clone(),
                                    })
                                    .await
                                {
                                    return Err(Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("Failed to send tick for {symbol}: {error}"),
                                    ));
                                }
                            }
                        }

                        price = None;
                        quantity = None;
                    }
                }
            }
        }

        Ok(())
    }
);
