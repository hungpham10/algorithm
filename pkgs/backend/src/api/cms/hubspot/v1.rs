use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Bytes, Data, Query};
use actix_web::{Error, HttpResponse, Result};

use serde::{Deserialize, Serialize};

use crate::api::cms::CmsHeaders;
use crate::api::AppState;

pub fn receive_data_changing(
    appstate: Data<Arc<AppState>>,
    body: Bytes,
    headers: CmsHeaders,
) -> Result<HttpResponse> {
    Err(ErrorInternalServerError("Not implemented"))
}
