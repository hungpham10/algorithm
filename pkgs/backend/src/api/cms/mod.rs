pub mod hubspot;
pub mod posts;

use actix_web::error::ErrorBadRequest;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};

#[derive(Debug)]
pub struct CmsHeaders {
    tenant_id: i32,
    hubspot_sign: Option<String>,
}

impl FromRequest for CmsHeaders {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers();

        let tenant_id = match headers.get("X-Tenant-Id") {
            Some(value) => match value.to_str() {
                Ok(str_val) => match str_val.parse::<i32>() {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        return ready(Err(ErrorBadRequest(
                            "Invalid x-tenant-id: must be a valid integer",
                        )));
                    }
                },
                Err(_) => {
                    return ready(Err(ErrorBadRequest(
                        "Invalid x-tenant-id: must be a valid string",
                    )));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing x-tenant-id header")));
            }
        };

        let hubspot_sign = match headers.get("X-HubSpot-Signature-v3") {
            Some(value) => match value.to_str() {
                Ok(str_val) => match str_val.parse::<i32>() {
                    Ok(parsed) => Some(parsed),
                    Err(_) => None,
                },
                Err(_) => None,
            },
            None => None,
        };

        ready(Ok(CmsHeaders {
            tenant_id,
            hubspot_sign,
        }))
    }
}
