use std::sync::Arc;

use actix_web::web::{Bytes, Data, Json};
use actix_web::{HttpRequest, HttpResponse, Result};

use crate::api::AppState;
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
