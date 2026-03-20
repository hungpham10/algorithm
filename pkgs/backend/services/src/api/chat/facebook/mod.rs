use std::fmt::{Display, Formatter, Result as FmtResult};

use axum::Router;
use axum_extra::TypedHeader;
use axum_macros::FromRequestParts;
use headers::Header;
use http::{HeaderName, HeaderValue};

use super::PlatformType;
use crate::api::{AppState, XTenantId};

#[derive(Debug)]
pub struct XHubSignature256(pub String);

impl From<XHubSignature256> for String {
    fn from(signature: XHubSignature256) -> Self {
        signature.0
    }
}

impl Display for XHubSignature256 {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl Header for XHubSignature256 {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-hub-signature-256");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> std::result::Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(headers::Error::invalid)?
            .to_str()
            .map_err(|_| headers::Error::invalid())?;

        Ok(XHubSignature256(value.to_owned()))
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Ok(value) = HeaderValue::from_str(&self.0) {
            values.extend(std::iter::once(value));
        }
    }
}

mod v1;

#[derive(FromRequestParts)]
struct ChatHeaders {
    #[from_request(via(TypedHeader))]
    pub tenant_id: XTenantId,

    #[from_request(via(TypedHeader))]
    pub signature: XHubSignature256,
}

pub fn v1() -> Router<AppState> {
    v1::routes()
}
