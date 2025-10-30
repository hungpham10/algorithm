use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::Data;
use actix_web::Result;

use reqwest::Client as HttpClient;
use serde_json::json;

use crate::api::AppState;

mod v1;
pub use v1::*;

pub async fn get_username(appdata: &Data<Arc<AppState>>, sender_id: &String) -> Result<String> {
    let client = HttpClient::default();
    let url = format!(
        "https://graph.facebook.com/{}?fields=name&access_token={}",
        sender_id, appdata.chat.fb.outgoing_secret,
    );
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?;

    let username = response["name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ErrorInternalServerError(format!("Failed to get username: {}", response)))?;

    Ok(username)
}

pub async fn send_message(
    appstate: &Data<Arc<AppState>>,
    send_id: &String,
    text: &String,
) -> Result<()> {
    let client = HttpClient::default();
    let url = format!(
        "https://graph.facebook.com/v24.0/me/messages?access_token={}",
        appstate.chat.fb.outgoing_secret,
    );
    let body = json!({
        "recipient": {
            "id": send_id
        },
        "message": {
            "text": text
        },
        "messaging_type": "RESPONSE"
    });
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|error| ErrorInternalServerError(error.to_string()))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(ErrorInternalServerError(format!(
            "Failed to send message: {}",
            error_text
        )));
    }
    Ok(())
}
