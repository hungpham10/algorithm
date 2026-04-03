pub mod v1;

use axum::Router;
use axum_extra::TypedHeader;
use axum_macros::FromRequestParts;
use utoipa::OpenApi;

use super::{AppState, XTenantId};

#[derive(FromRequestParts)]
pub struct InvestingHeaders {
    #[from_request(via(TypedHeader))]
    pub tenant_id: XTenantId,
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/v1", api = v1::InvestingV1Api)
    )
)]
pub struct InvestingApi;

pub fn routes() -> Router<AppState> {
    Router::new().nest("/v1", v1::routes())
}
