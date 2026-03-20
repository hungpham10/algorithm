use axum::Router;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use http::StatusCode;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::{AppState, ChatHeaders, PlatformType};

pub fn routes() -> Router<AppState> {
    Router::new().route("/webhook", get(verify_webhook))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,

    #[serde(rename = "hub.verify_token")]
    token: Option<String>,

    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

async fn verify_webhook(
    State(app_state): State<AppState>,
    Query(request): Query<VerifyRequest>,
) -> impl IntoResponse {
    let token = app_state.chat_secrets.fb.webhook_access_token.clone();

    if request.mode == Some("subscribe".to_string()) && request.token == Some(token) {
        match request.challenge {
            Some(challenge) => (StatusCode::OK, challenge),
            None => (StatusCode::UNAUTHORIZED, "Missing challenge".into()),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            format!(
                "Cannot recognize request type {}",
                request.mode.unwrap_or("unknown".into())
            ),
        )
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct WebhookRequest {
    object: String,

    #[serde(rename = "entry")]
    entries: Vec<Entry>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Entry {
    #[serde(rename = "messaging")]
    messages: Vec<Messaging>,

    time: u64,
    id: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Messaging {
    recipient: Option<Recipient>,
    sender: Option<Sender>,
    message: Option<Message>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Recipient {
    id: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Sender {
    id: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Message {
    metadata: Option<String>,
    mid: Option<String>,
    text: Option<String>,
    is_echo: Option<bool>,
}

async fn receive_message(
    State(app_state): State<AppState>,
    ChatHeaders {
        tenant_id,
        signature,
    }: ChatHeaders,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, impl IntoResponse)> {
    let tenant_id = tenant_id.into();
    let secret = app_state
        .admin_entity
        .get_unencrypted_token(tenant_id, &app_state.chat_secrets.fb.incomming_secret)
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed querying secret: {error}"),
            )
        })?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to calculate hmac<sha256>: {error}"),
        )
    })?;
    mac.update(&body);

    if format!("sha256={}", hex::encode(mac.finalize().into_bytes())) != signature.to_string() {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Compare between {signature} and our secret failed"),
        ));
    }

    let payload = serde_json::from_slice::<WebhookRequest>(&body).map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse body: {error}"),
        )
    })?;
    let request = serde_json::to_value(payload.clone()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render json request: {error}"),
        )
    })?;

    if payload.object == "page" {
        for entry in &payload.entries {
            for item in &entry.messages {
                if let (Some(sender), Some(message)) = (&item.sender, &item.message) {
                    if let Some(is_echo) = message.is_echo {
                        if is_echo {
                            continue;
                        }
                    }

                    if let Some(_text) = &message.text {
                        // @TODO: suy nghĩ thêm về cơ chế để xử lý theo luồng để có thể đẩy
                        //        xử lý một cách cân bằng giữa

                        //let username = get_username(&app_state, tenant_id)
                        //    .await
                        //    .unwrap_or(sender.id.clone());

                        match app_state
                            .chat_entity
                            .get_thread_by_sender_id(tenant_id.into(), &sender.id)
                            .await
                        {
                            Ok(Some(thread_id)) => {
                                //send_message(
                                //    &app_state,
                                //    tenant_id,
                                //    PlatformType::Facebook,
                                //    request.clone(),
                                //)
                                //.await
                                //.map_err(|error| (
                                //    StatusCode::INTERNAL_SERVER_ERROR,
                                //    format!("Failed to send message to `{}`: {error}", sender.id),
                                //))?;
                            }
                            Ok(None) => {
                                //create_thread(
                                //    &app_state,
                                //    tenant_id,
                                //    PlatformType::Facebook,
                                //    request.clone(),
                                //)
                                //.await
                                //.map_err(|error| (
                                //    StatusCode::INTERNAL_SERVER_ERROR,
                                //    format!("Failed to create new thread of `{}`: {error}", sender.id),
                                //))?;
                            }
                            Err(error) => {
                                return Err((
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Failed to find thread of `{}`", sender.id),
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("Invalid payload with object_type is `{}`", payload.object),
        ))
    }
}
