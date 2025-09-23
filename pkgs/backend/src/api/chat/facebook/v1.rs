use std::sync::Arc;

use actix_web::web::{Data, Json, Query};
use actix_web::{HttpRequest, HttpResponse, Result};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

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
    let fb_token = appstate.chat.fb.token.clone();

    if query.mode == Some("subscribe".to_string()) && query.token == Some(fb_token) {
        let challenge = query.challenge.clone();

        match challenge {
            Some(challenge) => Ok(HttpResponse::Ok().body(challenge)),
            None => Ok(HttpResponse::Forbidden().body("Missing challenge")),
        }
    } else {
        Ok(HttpResponse::Forbidden().body("Verification failed"))
    }
}

#[derive(Serialize, Deserialize)]
struct WebhookRequest {
    object: String,
    entry: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    messaging: Vec<Messaging>,
    time: u64,
    id: String,
}

#[derive(Serialize, Deserialize)]
struct Messaging {
    sender: Sender,
    recipient: Recipient,
    message: Option<Message>,
}

#[derive(Serialize, Deserialize)]
struct Recipient {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct Sender {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct Message {
    text: Option<String>,
}

pub async fn receive_message(
    appstate: Data<Arc<AppState>>,
    request: HttpRequest,
    payload: Json<WebhookRequest>,
) -> Result<HttpResponse> {
    let secret = appstate.chat.fb.secret.clone();

    if let Some(signature) = request.headers().get("x-hub-signature-256") {
        let signature = signature.to_str().unwrap_or("");
        let bytes = serde_json::to_vec(&payload).unwrap();

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");

        mac.update(&bytes);

        if format!("sha256={}", hex::encode(mac.finalize().into_bytes())) != signature {
            return Ok(HttpResponse::Forbidden().body("Invalid signature"));
        }
    }

    if payload.object == "page" {
        for entry in &payload.entry {
            for messaging in &entry.messaging {
                if let Some(message) = &messaging.message {
                    if let Some(text) = &message.text {
                        println!("Received message from {}: {}", messaging.sender.id, text);
                    }
                }
            }
        }

        Ok(HttpResponse::Ok().body("EVENT_RECEIVED"))
    } else {
        Ok(HttpResponse::NotFound().body("Invalid payload"))
    }
}
