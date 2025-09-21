use std::sync::Arc;

use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Result};

use log::{debug, error};
use serde::Deserialize;

use crate::api::AppState;

#[derive(Deserialize)]
pub struct VerifyRequest {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,

    #[serde(rename = "hub.verify_token")]
    token: Option<String>,

    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

pub async fn verify_webhook(
    appstate: Data<Arc<AppState>>,
    query: Query<VerifyRequest>,
) -> Result<HttpResponse> {
    let fb_token = appstate.chat.fb_token.clone();

    if query.token == Some(fb_token) {
        let challenge = query.challenge.clone();

        match challenge {
            Some(challenge) => Ok(HttpResponse::Ok().body(challenge)),
            None => Ok(HttpResponse::Forbidden().body("Missing challenge")),
        }
    } else {
        Ok(HttpResponse::Forbidden().body("Verification failed"))
    }
}

#[derive(Deserialize)]
struct WebhookRequest {
    object: String,
    entry: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    messaging: Vec<Messaging>,
    time: u64,
    id: String,
}

#[derive(Deserialize)]
struct Messaging {
    sender: Sender,
    recipient: Recipient,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Recipient {
    id: String,
}

#[derive(Deserialize)]
struct Sender {
    id: String,
}

#[derive(Deserialize)]
struct Message {
    text: String,
}

pub async fn receive_message(query: Query<serde_json::Value>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}
