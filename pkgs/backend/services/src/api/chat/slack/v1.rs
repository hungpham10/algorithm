use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use http::StatusCode;

use serde::{Deserialize, Serialize};

use super::{AppState, ChatHeaders, PlatformType};

pub fn routes() -> Router<AppState> {
    Router::new().route("/webhook", post(receive_message))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SlackMessage {
    channel: String,
    text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,

    #[serde(rename = "bot_id")]
    bot: Option<String>,

    user: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    thread_ts: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SlackEventWrapper {
    token: String,
    challenge: Option<String>,
    event: Option<SlackEvent>,
}

async fn receive_message(
    State(app_state): State<AppState>,
    ChatHeaders { tenant_id }: ChatHeaders,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let tenant_id = tenant_id.into();
    let payload = serde_json::from_slice::<SlackEventWrapper>(&body).map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed parsing payload: {error}"),
        )
    })?;
    let request = serde_json::to_value(payload.clone()).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render json request: {error}"),
        )
    })?;

    if let Some(challenge) = &payload.challenge {
        return Err((StatusCode::OK, challenge.to_string()));
    }

    if let Some(event) = &payload.event {
        if event.event_type == "message" {
            if let (Some(user), Some(text), Some(thread_ts)) =
                (&event.user, &event.text, &event.thread_ts)
            {
                match app_state
                    .chat_entity
                    .get_sender_id_by_thread(tenant_id, thread_ts)
                    .await
                {
                    Ok(Some(sender_id)) => {
                        //reply_message(
                        //        &app_state,
                        //        tenant_id.into(),
                        //        PlatformType::Slack,
                        //        request,
                        //    )
                        //    .await
                        //    .map_err(|error| (
                        //        StatusCode::INTERNAL_SERVER_ERROR,
                        //        format!("Reply message failed: {error}"),
                        //    ))?;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to get sender's id: {error}"),
                        ));
                    }
                }
            }
        }

        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Payload is empty".to_string(),
        ))
    }
}
