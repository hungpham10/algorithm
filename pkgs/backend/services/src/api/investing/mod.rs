mod v1;
mod v2;

use axum::Router;
use axum_extra::TypedHeader;
use axum_macros::FromRequestParts;
use headers::Header;
use http::{HeaderName, HeaderValue};
use utoipa::OpenApi;

use super::{AppState, XTenantId};

#[derive(Debug, Clone)]
pub struct XUserId(pub Option<String>);

impl Header for XUserId {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-user-id");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> std::result::Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        // Lấy giá trị đầu tiên từ iterator
        let value = values.next();

        match value {
            Some(v) => {
                let value_str = v.to_str().map_err(|_| headers::Error::invalid())?;
                if value_str.is_empty() {
                    Ok(XUserId(None))
                } else {
                    Ok(XUserId(Some(value_str.to_string())))
                }
            }
            None => Ok(XUserId(None)),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Some(ref id) = self.0
            && let Ok(value) = HeaderValue::from_str(id)
        {
            values.extend(std::iter::once(value));
        }
    }
}

#[derive(FromRequestParts)]
pub struct InvestingHeaders {
    #[from_request(via(TypedHeader))]
    pub tenant_id: XTenantId,

    #[from_request(via(TypedHeader))]
    pub user_id: XUserId,
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/v1", api = v1::InvestingV1Api),
        (path = "/v2", api = v2::InvestingV2Api)
    )
)]
pub struct InvestingApi;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/v1", v1::routes())
        .nest("/v2", v2::routes())
}
