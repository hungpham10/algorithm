use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data, Query};
use actix_web::{HttpRequest, HttpResponse, Result};

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::api::chat::slack::{create_thread, send_message_to_existing_thread};

use super::get_facebook_username;
use crate::api::chat::ChatHeaders;
use crate::api::AppState;
use crate::entities::chat::Thread;

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
    let token = appstate.chat.fb.webhook_access_token.clone();
    let mode = "subscribe".to_string();

    if query.mode == Some(mode) && query.token == Some(token) {
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
pub struct WebhookRequest {
    object: String,
    entry: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct Entry {
    messaging: Vec<Messaging>,
    //time: u64,
    //id: String,
}

#[derive(Deserialize)]
pub struct Messaging {
    //recipient: Option<Recipient>,
    sender: Option<Sender>,
    message: Option<Message>,
}

//#[derive(Deserialize)]
//pub struct Recipient {
//    id: String,
//}

#[derive(Deserialize)]
pub struct Sender {
    id: String,
}

#[derive(Deserialize)]
pub struct Message {
    //metadata: Option<String>,
    //mid: Option<String>,
    text: Option<String>,
    is_echo: Option<bool>,
}

pub async fn receive_message(
    appstate: Data<Arc<AppState>>,
    request: HttpRequest,
    body: Bytes,
    headers: ChatHeaders,
) -> Result<HttpResponse> {
    let secret = appstate.chat.fb.incomming_secret.clone();

    if let Some(entity) = appstate.chat_entity() {
        if let Some(signature) = request.headers().get("x-hub-signature-256") {
            let signature = signature.to_str().unwrap_or("");

            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");

            mac.update(&body);

            let actual = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
            if actual != signature {
                return Ok(HttpResponse::Forbidden().body("Invalid signature"));
            }
        }

        let payload = serde_json::from_slice::<WebhookRequest>(&body)?;
        if payload.object == "page" {
            for entry in &payload.entry {
                for messaging in &entry.messaging {
                    if let (Some(sender), Some(message)) = (&messaging.sender, &messaging.message) {
                        if let Some(is_echo) = &message.is_echo {
                            if *is_echo {
                                continue;
                            }
                        }

                        if let Some(text) = &message.text {
                            let username = get_facebook_username(&appstate, &sender.id).await?;

                            if let Ok(thread_id) = entity
                                .get_thread_by_sender_id(headers.tenant_id, &sender.id)
                                .await
                            {
                                send_message_to_existing_thread(&appstate, &thread_id, &text)
                                    .await?;
                            } else {
                                let thread_id = create_thread(&appstate, &username, &text).await?;

                                entity
                                    .start_new_thread(
                                        headers.tenant_id,
                                        Thread {
                                            thread_id: thread_id.clone(),
                                            source_id: sender.id.clone(),
                                            source_type: 0,
                                        },
                                    )
                                    .await
                                    .map_err(|error| {
                                        ErrorInternalServerError(format!(
                                            "Fail to start new thread: {}",
                                            error
                                        ))
                                    })?;
                            }

                            // @TODO: store message to parquet in S3
                        }
                    }
                }
            }

            Ok(HttpResponse::Ok().body("EVENT_RECEIVED"))
        } else {
            Ok(HttpResponse::NotFound().body("Invalid payload"))
        }
    } else {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    }
}
