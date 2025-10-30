use std::sync::Arc;

use log::info;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data, Query};
use actix_web::{Error, HttpResponse, Result};

use serde::{Deserialize, Serialize};

use crate::api::chat::facebook::send_message;
use crate::api::chat::ChatHeaders;
use crate::api::AppState;

#[derive(Deserialize, Debug)]
pub struct SlackMessage {
    channel: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,

    #[serde(rename = "bot_id")]
    bot: Option<String>,

    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    thread_ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackEventWrapper {
    token: String,
    challenge: Option<String>,
    event: Option<SlackEvent>,
}

pub async fn receive_message(
    appstate: Data<Arc<AppState>>,
    body: Bytes,
    headers: ChatHeaders,
) -> Result<HttpResponse> {
    let payload = serde_json::from_slice::<SlackEventWrapper>(&body)?;

    if let Some(challenge) = &payload.challenge {
        return Ok(HttpResponse::Ok().body(challenge.clone()));
    }

    if let Some(entity) = appstate.chat_entity() {
        if let Some(event) = &payload.event {
            if event.event_type == "message" {
                if event.bot.is_none() {
                    if let (Some(user), Some(text), Some(thread_ts)) =
                        (&event.user, &event.text, &event.thread_ts)
                    {
                        match entity
                            .get_sender_id_by_thread(headers.tenant_id, thread_ts)
                            .await
                        {
                            Ok(Some(sender_id)) => {
                                send_message(&appstate, &sender_id, &text)
                                    .await
                                    .map_err(|error| do_send_message_falure(error))?
                                // @TODO: store message to parquet in S3
                            }
                            Ok(None) => {
                                // @TODO: maybe this is command from CSKH
                            }
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
        }

        Ok(HttpResponse::Ok().finish())
    } else {
        Err(ErrorInternalServerError("Not implemented"))
    }
}

fn do_send_message_falure(error: Error) -> Error {
    error
}
