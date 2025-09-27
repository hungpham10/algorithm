use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Result};

use serde::{Deserialize, Serialize};

use crate::api::chat::facebook::send_message;
use crate::api::chat::ChatHeaders;
use crate::api::AppState;

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
    thread_ts: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SlackEventWrapper {
    token: String,
    challenge: Option<String>,
    event: Option<SlackEvent>,
}

pub async fn receive_message(
    appstate: Data<Arc<AppState>>,
    payload: Json<SlackEventWrapper>,
    headers: ChatHeaders,
) -> Result<HttpResponse> {
    if let Some(challenge) = &payload.challenge {
        return Ok(HttpResponse::Ok().body(challenge.clone()));
    }

    if let Some(entity) = appstate.chat_entity() {
        if let Some(event) = &payload.event {
            if event.event_type == "message" {
                if let (Some(user), Some(text), Some(thread_ts)) =
                    (&event.user, &event.text, &event.thread_ts)
                {
                    match entity
                        .get_sender_id_by_thread(headers.tenant_id, thread_ts)
                        .await
                    {
                        Ok(Some(sender_id)) => {
                            send_message(&appstate, &sender_id, &text).await?;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            return Err(ErrorInternalServerError(format!(
                                "Fail to get sender.id: {}",
                                error
                            )));
                        }
                    }
                }
            }
        }

        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::InternalServerError().body(format!("Not implemented")))
    }
}
