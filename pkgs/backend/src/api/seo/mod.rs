mod v1;
pub use v1::*;

use actix_web::error::ErrorBadRequest;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};

#[derive(Debug)]
pub struct SeoHeaders {
    host: String,
    tenant_id: i32,
    user_agent: String,
    user_type: String,
    device_type: String,
}

impl FromRequest for SeoHeaders {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers();

        let user_agent = match headers.get("User-Agent") {
            Some(value) => match value.to_str() {
                Ok(str_val) => str_val.to_string(),
                Err(_) => {
                    return ready(Err(ErrorBadRequest(
                        "Invalid user-agent: must be a valid string",
                    )));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing User-Agent header")));
            }
        };
        let host = match headers.get("Host") {
            Some(value) => match value.to_str() {
                Ok(str_val) => str_val.to_string(),
                Err(_) => {
                    return ready(Err(ErrorBadRequest("Invalid host: must be a valid string")));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing Host header")));
            }
        };
        let user_type = match headers.get("X-User-Type") {
            Some(value) => match value.to_str() {
                Ok(str_val) => str_val.to_string(),
                Err(_) => {
                    return ready(Err(ErrorBadRequest(
                        "Invalid x-user-type: must be a valid string",
                    )));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing X-User-Type header")));
            }
        };
        let device_type = match headers.get("X-Device-Type") {
            Some(value) => match value.to_str() {
                Ok(str_val) => str_val.to_string(),
                Err(_) => {
                    return ready(Err(ErrorBadRequest(
                        "Invalid x-device-type: must be a valid string",
                    )));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing X-Device-Type header")));
            }
        };
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
            None => 0,
        };

        ready(Ok(SeoHeaders {
            host,
            tenant_id,
            user_agent,
            user_type,
            device_type,
        }))
    }
}
