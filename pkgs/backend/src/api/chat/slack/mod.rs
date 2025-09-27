use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::Data;
use actix_web::Result;

use crate::api::AppState;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

mod v1;
pub use v1::*;

#[derive(Serialize)]
struct SlackPostMessageRequest {
    channel: String,
    text: String,
    username: Option<String>,
    thread_ts: Option<String>,
}

#[derive(Deserialize)]
struct SlackPostMessageReponse {
    ts: String,
}

pub async fn create_thread(
    appstate: &Data<Arc<AppState>>,
    sender: &String,
    message: &String,
) -> Result<String> {
    let client = HttpClient::default();
    let payload = SlackPostMessageRequest {
        channel: appstate.chat.slack.channel.clone(),
        text: format!("fb {}: {}", sender, message),
        username: Some(format!("fb {}", sender)),
        thread_ts: None,
    };
    let response = client
        .post("https://slack.com/api/chat.postMessage")
        .header(
            "Authorization",
            format!("Bearer {}", appstate.chat.slack.token),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?
        .json::<SlackPostMessageReponse>()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?;

    Ok(response.ts)
}

pub async fn send_message(
    appstate: &Data<Arc<AppState>>,
    thread_id: &String,
    sender_id: &String,
    message: &String,
) -> Result<()> {
    let client = HttpClient::default();
    let payload = SlackPostMessageRequest {
        channel: appstate.chat.slack.channel.clone(),
        text: message.clone(),
        username: Some(format!("fb:{}", sender_id)),
        thread_ts: Some(thread_id.clone()),
    };

    client
        .post("https://slack.com/api/chat.postMessage")
        .header(
            "Authorization",
            format!("Bearer {}", appstate.chat.slack.token),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?
        .json::<SlackPostMessageReponse>()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?;

    Ok(())
}
