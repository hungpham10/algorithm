use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data, Json};
use actix_web::{HttpRequest, HttpResponse, Result};

use crate::api::AppState;
use log::error;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SlackMessage {
    channel: String,
    text: String,
}

#[derive(Serialize, Deserialize)]
pub struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SlackEventWrapper {
    token: String,
    challenge: Option<String>,
    event: Option<SlackEvent>,
}

pub async fn receive_message(
    appstate: Data<Arc<AppState>>,
    request: HttpRequest,
    payload: Json<SlackEventWrapper>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::NotFound().body("Invalid payload"))
}

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

pub async fn create_new_thread(
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
