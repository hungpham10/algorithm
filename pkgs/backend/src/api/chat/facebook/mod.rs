use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::Data;
use actix_web::Result;

use crate::api::AppState;
use reqwest::Client as HttpClient;

mod v1;
pub use v1::*;

pub async fn get_facebook_username(
    appdata: &Data<Arc<AppState>>,
    sender_id: &String,
) -> Result<String> {
    let client = HttpClient::default();
    let url = format!(
        "https://graph.facebook.com/{}?fields=name&access_token={}",
        sender_id, appdata.chat.fb.page_access_token,
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
        .ok_or_else(|| ErrorInternalServerError("Failed to get username"))?;

    Ok(username)
}
