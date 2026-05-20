use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::Error;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc};

pub enum Event {
    Minor((usize, Error)),
    Major((usize, Error)),
    Panic((usize, Error)),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum ComponentType {
    Unknown,
    Input,
    Output,
    Source,
    Sink,
    Transform,
}

impl From<i32> for ComponentType {
    fn from(value: i32) -> Self {
        match value {
            1 => ComponentType::Source,
            2 => ComponentType::Sink,
            3 => ComponentType::Transform,
            4 => ComponentType::Input,
            5 => ComponentType::Output,
            _ => ComponentType::Unknown,
        }
    }
}

impl From<String> for ComponentType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Source" => ComponentType::Source,
            "Sink" => ComponentType::Sink,
            "Transform" => ComponentType::Transform,
            "Input" => ComponentType::Input,
            "Output" => ComponentType::Output,
            _ => ComponentType::Unknown,
        }
    }
}

impl Display for ComponentType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ComponentType::Unknown => write!(f, "Unknown"),
            ComponentType::Source => write!(f, "Source"),
            ComponentType::Sink => write!(f, "Sink"),
            ComponentType::Input => write!(f, "Input"),
            ComponentType::Output => write!(f, "Output"),
            ComponentType::Transform => write!(f, "Transform"),
        }
    }
}

impl<'de> Deserialize<'de> for ComponentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ComponentType::from(s))
    }
}

impl serde::Serialize for ComponentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Message {
    pub payload: Value,
}

pub struct Outbound {
    pub streams: Vec<mpsc::Sender<Message>>,
    pub broadcast: Option<broadcast::Sender<Message>>,
    pub event: mpsc::Sender<Event>,
}

pub trait Identify {
    fn id(&self) -> String;
    fn get_inputs(&self) -> Option<&Vec<String>>;
    fn clone_arc(&self) -> Arc<dyn Component>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn component_type(&self) -> ComponentType;
    fn compare(&self, other: &dyn Component) -> bool;
}

#[typetag::serde(tag = "type")]
#[async_trait]
pub trait Component: Identify + Send + Sync + Debug {
    async fn run(
        &self,
        id: usize,
        rx: &mut mpsc::Receiver<Message>,
        tx: Outbound,
    ) -> Result<(), Error>;
}
