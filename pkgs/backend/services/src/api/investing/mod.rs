mod v1;

use axum::Router;
use axum_extra::TypedHeader;
use axum_macros::FromRequestParts;

use super::{AppState, XTenantId};

#[derive(FromRequestParts)]
pub struct InvestingHeaders {
    #[from_request(via(TypedHeader))]
    pub tenant_id: XTenantId,
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/v1", v1::routes())
}
