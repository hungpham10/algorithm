mod facebook;
mod slack;

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error, ErrorKind};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use axum::Router;
use models::entities::admin::ApiType;

use super::AppState;

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Slack {
    token: String,
    channel: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Facebook {
    webhook_access_token: String,
    incomming_secret: String,
    outgoing_secret: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ChatSecret {
    slack: Slack,
    fb: Facebook,
}

#[derive(Clone, Debug, PartialEq)]
#[repr(i32)]
pub enum PlatformType {
    Unknown,
    Facebook,
    Slack,
}

impl From<i32> for PlatformType {
    fn from(value: i32) -> Self {
        match value {
            1 => PlatformType::Facebook,
            2 => PlatformType::Slack,
            _ => PlatformType::Unknown,
        }
    }
}

impl From<String> for PlatformType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "facebook" => PlatformType::Facebook,
            "slack" => PlatformType::Slack,
            _ => PlatformType::Unknown,
        }
    }
}

impl Display for PlatformType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            PlatformType::Unknown => write!(f, "unknown"),
            PlatformType::Facebook => write!(f, "facebook"),
            PlatformType::Slack => write!(f, "slack"),
        }
    }
}

impl<'de> Deserialize<'de> for PlatformType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PlatformType::from(s))
    }
}

impl serde::Serialize for PlatformType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl PlatformType {
    pub async fn create_thread(
        &self,
        app_state: &AppState,
        tenant_id: i64,
        payload: Value,
    ) -> Result<(), Error> {
        self.perform_chat_flow(
            app_state,
            format!("create_thread_running_in_{self}"),
            tenant_id,
            vec![],
            payload,
        )
        .await
    }

    pub async fn send_message(
        &self,
        app_state: &AppState,
        tenant_id: i64,
        payload: Value,
    ) -> Result<(), Error> {
        self.perform_chat_flow(
            app_state,
            format!("send_message_running_in_{self}"),
            tenant_id,
            vec![],
            payload,
        )
        .await
    }

    pub async fn reply_message(
        &self,
        app_state: &AppState,
        tenant_id: i64,
        payload: Value,
    ) -> Result<(), Error> {
        self.perform_chat_flow(
            app_state,
            format!("reply_message_running_in_{self}"),
            tenant_id,
            vec![],
            payload,
        )
        .await
    }

    async fn perform_chat_flow(
        &self,
        app_state: &AppState,
        api_name: String,
        tenant_id: i64,
        args: Vec<String>,
        payload: Value,
    ) -> Result<(), Error> {
        let mut headers = HashMap::new();

        if let Ok(token) = app_state
            .admin_entity
            .get_unencrypted_token(tenant_id, &self.to_string())
            .await
        {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }

        app_state
            .admin_entity
            .perform_api_by_api_name(
                tenant_id,
                &api_name,
                ApiType::Create,
                args,
                headers,
                Some(payload),
            )
            .await
            .map_err(|error| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed perfoming api `{api_name}`: {error}"),
                )
            })?;
        Ok(())
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/v1/facebook", facebook::v1())
        .nest("/v1/slack", slack::v1())
}
