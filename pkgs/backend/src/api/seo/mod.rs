mod v1;
pub use v1::*;

use actix_web::error::ErrorBadRequest;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};

#[derive(Debug)]
pub struct SeoHeaders {
    host: String,
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
                return ready(Err(ErrorBadRequest("Missing User-Agent header")));
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
                return ready(Err(ErrorBadRequest("Missing User-Agent header")));
            }
        };
        let device_type = match headers.get("X-Devide-Type") {
            Some(value) => match value.to_str() {
                Ok(str_val) => str_val.to_string(),
                Err(_) => {
                    return ready(Err(ErrorBadRequest(
                        "Invalid x-device-type: must be a valid string",
                    )));
                }
            },
            None => {
                return ready(Err(ErrorBadRequest("Missing User-Agent header")));
            }
        };

        ready(Ok(SeoHeaders {
            host,
            user_agent,
            user_type,
            device_type,
        }))
    }
}
