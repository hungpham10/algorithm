use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use tokio::sync::mpsc;

use algorithm::{JsonQuery, Operator};
use vector_config_macro::transform;
use vector_runtime::{Component, Identify, Message, Outbound};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CastType {
    U64,
    U32,
    I64,
    I32,
    F64,
    F32,
    String,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformConfig {
    query: Vec<Operator>,
    cast_to: Option<CastType>,
}

#[transform(derive(PartialEq))]
pub struct Json2Json {
    pub id: String,
    pub inputs: Vec<String>,
    pub constants: Option<HashMap<String, Value>>,
    pub transforms: HashMap<String, TransformConfig>,
}

fn cast_value(val: Value, target_type: &CastType) -> Option<Value> {
    match target_type {
        CastType::U64 => match val {
            Value::Number(n) => n.as_u64().map(|v| Value::Number(Number::from(v))),
            Value::String(s) => s
                .trim()
                .parse::<u64>()
                .ok()
                .map(|v| Value::Number(Number::from(v))),
            Value::Bool(b) => Some(Value::Number(Number::from(if b { 1u64 } else { 0u64 }))),
            _ => None,
        },
        CastType::U32 => match val {
            Value::Number(n) => n
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .map(|v| Value::Number(Number::from(v))),
            Value::String(s) => s
                .trim()
                .parse::<u32>()
                .ok()
                .map(|v| Value::Number(Number::from(v))),
            Value::Bool(b) => Some(Value::Number(Number::from(if b { 1u32 } else { 0u32 }))),
            _ => None,
        },
        CastType::I64 => match val {
            Value::Number(n) => n.as_i64().map(|v| Value::Number(Number::from(v))),
            Value::String(s) => s
                .trim()
                .parse::<i64>()
                .ok()
                .map(|v| Value::Number(Number::from(v))),
            Value::Bool(b) => Some(Value::Number(Number::from(if b { 1i64 } else { -0i64 }))),
            _ => None,
        },
        CastType::I32 => match val {
            Value::Number(n) => n
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .map(|v| Value::Number(Number::from(v))),
            Value::String(s) => s
                .trim()
                .parse::<i32>()
                .ok()
                .map(|v| Value::Number(Number::from(v))),
            Value::Bool(b) => Some(Value::Number(Number::from(if b { 1i32 } else { 0i32 }))),
            _ => None,
        },
        CastType::F64 => match val {
            Value::Number(n) => n.as_f64().map(|v| {
                Number::from_f64(v)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }),
            Value::String(s) => s
                .trim()
                .parse::<f64>()
                .ok()
                .and_then(|v| Number::from_f64(v).map(Value::Number)),
            Value::Bool(b) => Number::from_f64(if b { 1.0 } else { 0.0 }).map(Value::Number),
            _ => None,
        },
        CastType::F32 => match val {
            Value::Number(n) => n.as_f64().map(|v| {
                Number::from_f64(v)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }),
            Value::String(s) => s
                .trim()
                .parse::<f32>()
                .ok()
                .and_then(|v| Number::from_f64(v as f64).map(Value::Number)),
            Value::Bool(b) => Number::from_f64(if b { 1.0f64 } else { 0.0f64 }).map(Value::Number),
            _ => None,
        },
        CastType::String => match val {
            Value::String(s) => Some(Value::String(s)),
            Value::Number(n) => Some(Value::String(n.to_string())),
            Value::Bool(b) => Some(Value::String(b.to_string())),
            Value::Null => Some(Value::String("null".to_string())),
            _ => Some(Value::String(val.to_string())),
        },
        CastType::Bool => match val {
            Value::Bool(b) => Some(Value::Bool(b)),
            Value::String(s) => match s.trim().to_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(Value::Bool(true)),
                "false" | "0" | "off" | "no" => Some(Value::Bool(false)),
                _ => None,
            },
            Value::Number(n) => Some(Value::Bool(n.as_f64().is_some_and(|v| v != 0.0))),
            Value::Null => Some(Value::Bool(false)),
            _ => None,
        },
    }
}

impl_json_2_json!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error> {
        let pipelines = self
            .transforms
            .iter()
            .map(|(output, config)| {
                (
                    output,
                    (JsonQuery::new(config.query.to_vec()), &config.cast_to),
                )
            })
            .collect::<HashMap<_, _>>();

        while let Some(message) = rx.recv().await {
            let bytes = message.payload.to_string().into_bytes();

            let raw_json: Value = match serde_json::from_slice(&bytes) {
                Ok(val) => val,
                Err(_) => {
                    continue;
                }
            };

            let mut output_map = Map::new();
            let mut skip_message = false;

            for (&output, (query, cast_to)) in &pipelines {
                if let Some(&node) = query.pick(&raw_json).first() {
                    let mut final_node = node.clone();

                    if let Some(target_type) = cast_to {
                        if let Some(casted_value) = cast_value(final_node, target_type) {
                            final_node = casted_value;
                        } else {
                            skip_message = true;
                            break;
                        }
                    }

                    output_map.insert(output.clone(), final_node);
                } else {
                    skip_message = true;
                    break;
                }
            }

            if skip_message {
                continue;
            }

            if let Some(constants) = &self.constants {
                for (key, value) in constants {
                    output_map.insert(key.clone(), value.clone());
                }
            }

            let output_payload = Value::Object(output_map);

            for stream in &tx.streams {
                if let Err(error) = stream
                    .send(Message {
                        payload: output_payload.clone(),
                    })
                    .await
                {
                    return Err(Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to forward dynamic json downstream: {error}"),
                    ));
                }
            }
        }

        Ok(())
    }
);
