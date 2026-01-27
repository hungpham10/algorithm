use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data, Query};
use actix_web::{Error, HttpResponse, Result};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::cms::CmsHeaders;
use crate::api::AppState;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookEvent {
    #[serde(rename = "objectId")]
    pub object_id: i64,

    #[serde(rename = "eventId")]
    pub event_id: i64,

    #[serde(rename = "subscriptionId")]
    pub subscription_id: i64,

    #[serde(rename = "portalId")]
    pub portal_id: i64,

    #[serde(rename = "appId")]
    pub app_id: i32,

    #[serde(rename = "occurredAt")]
    pub occurred_at: i64,

    #[serde(rename = "attemptNumber")]
    pub attempt_number: i32,

    #[serde(rename = "changeSource")]
    pub change_source: String,

    #[serde(rename = "objectTypeId")]
    pub object_type_id: String,

    #[serde(flatten)]
    pub data: WebhookEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subscriptionType")] // Tag nội bộ
enum WebhookEventData {
    #[serde(rename = "object.creation")]
    Creation,

    #[serde(rename = "object.deletion")]
    Deletion,

    #[serde(rename = "object.restore")]
    Restore,

    #[serde(rename = "object.privacyDeletion")]
    PrivacyDeletion,

    #[serde(rename = "object.propertyChange")]
    PropertyChange {
        property_name: String,
        property_value: Value,
    },

    #[serde(rename = "object.merge")]
    Merge {
        #[serde(default)]
        primary_object_id: Option<i64>,
        merged_object_ids: Vec<i64>,
        #[serde(default)]
        new_object_id: Option<i64>,
        #[serde(default)]
        number_of_properties_moved: Option<i64>,
    },

    #[serde(rename = "object.associationChange")]
    AssociationChange {
        from_object_type_id: String,
        to_object_type_id: String,
        association_type_id: i64,
        association_category: String,
    },

    #[serde(other)]
    Unknown,
}

pub async fn receive_data_changing(
    appstate: Data<Arc<AppState>>,
    body: Bytes,
    headers: CmsHeaders,
) -> Result<HttpResponse> {
    let payload = serde_json::from_slice::<Vec<WebhookEvent>>(&body)
        .map_err(|error| ErrorInternalServerError(format!("Parse error: {}", e)))?;

    for event in payload {
        match &event.data {
            HubSpotEventData::Creation { object_id } => {
                println!(
                    "Object {} (type {}) được tạo",
                    object_id, event.object_type_id
                );
                // Gọi API lấy chi tiết object nếu cần
            }
            HubSpotEventData::PropertyChange {
                property_name,
                property_value,
                ..
            } => {
                println!(
                    "Property '{}' thay đổi thành: {:?}",
                    property_name, property_value
                );
            }
            HubSpotEventData::Merge {
                merged_object_ids, ..
            } => {
                println!("Object bị merge: {:?}", merged_object_ids);
            }
            HubSpotEventData::AssociationChange {
                from_object_type_id,
                to_object_type_id,
                ..
            } => {
                println!(
                    "Association từ {} tới {}",
                    from_object_type_id, to_object_type_id
                );
            }
            HubSpotEventData::Deletion { .. } => {
                println!("Object bị xóa");
            }
            HubSpotEventData::Restore { .. } => {
                println!("Object được khôi phục");
            }
            HubSpotEventData::PrivacyDeletion { .. } => {
                println!("Object bị xóa do privacy request");
            }
        }
    }

    Ok(HttpResponse::Ok().body("Received"))
}
