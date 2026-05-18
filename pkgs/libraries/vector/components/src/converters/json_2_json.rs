use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use serde_json::{Map, Value};
use tokio::sync::mpsc;

use algorithm::JsonQuery;
use vector_config_macro::transform;
use vector_runtime::{Component, Identify, Message, Outbound};

#[transform(derive(PartialEq))]
pub struct Json2Json {
    pub id: String,
    pub inputs: Vec<String>,
    pub transforms: HashMap<String, String>,
}

impl_json_2_json!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        let root_array_query = if let Some(root_path) = self.inputs.first() {
            if !root_path.is_empty() {
                Some(JsonQuery::parse(root_path)?)
            } else {
                None
            }
        } else {
            None
        };

        let mut pipelines = HashMap::new();

        for (output, query) in &self.transforms {
            pipelines.insert(
                output,
                JsonQuery::parse(query).map_err(|error| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!("Fail parsing using {query}: {error}"),
                    )
                })?,
            );
        }

        while let Some(message) = rx.recv().await {
            let bytes = message.payload.to_string().into_bytes();

            let raw_json: Value = match serde_json::from_slice(&bytes) {
                Ok(val) => val,
                Err(_) => {
                    continue;
                }
            };

            let elements = match &root_array_query {
                Some(query) => query.pick(&raw_json),
                None => vec![&raw_json],
            };

            let mut batched_elements = Vec::with_capacity(elements.len());

            for item in elements {
                let mut output_map = Map::new();
                let mut skip_item = false;

                for (&output, query) in &pipelines {
                    if let Some(&node) = query.pick(item).first() {
                        output_map.insert(output.clone(), node.clone());
                    } else {
                        skip_item = true;
                        break;
                    }
                }

                if skip_item {
                    continue;
                }

                batched_elements.push(Value::Object(output_map));
            }

            if batched_elements.is_empty() {
                continue;
            }

            let output_payload = Value::Array(batched_elements);

            for stream in &tx.streams {
                if let Err(error) = stream
                    .send(Message {
                        payload: output_payload.clone(),
                    })
                    .await
                {
                    return Err(Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to forward dynamic json batch downstream: {error}"),
                    ));
                }
            }
        }

        Ok(())
    }
);
