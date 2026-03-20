use axum::Router;
use axum_extra::TypedHeader;
use axum_macros::FromRequestParts;

use super::PlatformType;
use crate::api::{AppState, XTenantId};

mod v1;

#[derive(FromRequestParts)]
struct ChatHeaders {
    #[from_request(via(TypedHeader))]
    pub tenant_id: XTenantId,
}

pub fn v1() -> Router<AppState> {
    v1::routes()
}
