pub mod facebook;
pub mod slack;

use actix_web::error::ErrorBadRequest;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};

pub struct Slack {
    pub token: String,
    pub channel: String,
}

pub struct Facebook {
    pub webhook_access_token: String,
    pub page_access_token: String,
    pub incomming_secret: String,
    pub outgoing_secret: String,
}

pub struct Chat {
    pub slack: Slack,
    pub fb: Facebook,
}

#[derive(Debug)]
pub struct ChatHeaders {
    tenant_id: i32,
}

impl FromRequest for ChatHeaders {
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

        ready(Ok(ChatHeaders { tenant_id }))
    }
}
