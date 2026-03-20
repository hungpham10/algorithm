use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use models::entities::admin::ApiType;
use serde_json::Value;

use super::{AppState, PlatformType};

pub async fn create_thread(
    app_state: &AppState,
    tenant_id: i64,
    module: PlatformType,
    payload: Value,
) -> Result<(), Error> {
    perform_chat_flow(
        app_state,
        format!("create_thread_running_in_{module}"),
        tenant_id,
        module,
        vec![],
        payload,
    )
    .await
}

pub async fn get_username(app_state: &AppState, tenant_id: i64) -> Result<String, Error> {
    Ok("".into())
}

pub async fn send_message(
    app_state: &AppState,
    tenant_id: i64,
    module: PlatformType,
    payload: Value,
) -> Result<(), Error> {
    perform_chat_flow(
        app_state,
        format!("send_message_running_in_{module}"),
        tenant_id,
        module,
        vec![],
        payload,
    )
    .await
}

pub async fn reply_message(
    app_state: &AppState,
    tenant_id: i64,
    module: PlatformType,
    payload: Value,
) -> Result<(), Error> {
    perform_chat_flow(
        app_state,
        format!("reply_message_running_in_{module}"),
        tenant_id,
        module,
        vec![],
        payload,
    )
    .await
}

async fn perform_chat_flow(
    app_state: &AppState,
    api_name: String,
    tenant_id: i64,
    module: PlatformType,
    args: Vec<String>,
    payload: Value,
) -> Result<(), Error> {
    let mut headers = HashMap::new();

    if let Ok(token) = app_state
        .admin_entity
        .get_unencrypted_token(tenant_id, &module.to_string())
        .await
    {
        headers.insert("Authorization".to_string(), format!("Bearer {}", token));
    }

    app_state
        .admin_entity
        .perform_api_by_api_name(
            tenant_id,
            &api_name,
            ApiType::Create,
            args,
            headers,
            Some(payload),
        )
        .await
        .map_err(|error| {
            Error::new(
                ErrorKind::Other,
                format!("Failed perfoming api `{api_name}`: {error}"),
            )
        })?;
    Ok(())
}
