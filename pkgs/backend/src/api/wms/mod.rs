pub mod v1;

use actix_web::error::ErrorBadRequest;
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};

#[derive(Debug)]
pub struct WmsHeaders {
    tenant_id: i32,
    is_guess: bool,
}

impl FromRequest for WmsHeaders {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers();

        let is_guess = match headers.get("x-is-guess") {
            Some(value) => match value.to_str() {
                Ok(str_val) => match str_val.parse::<bool>() {
                    Ok(parsed) => parsed,
                    Err(_) => false,
                },
                Err(_) => false,
            },
            None => false,
        };
        let tenant_id = match headers.get("x-tenant-id") {
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
                // Trả về lỗi nếu header không tồn tại
                return ready(Err(ErrorBadRequest("Missing x-tenant-id header")));
            }
        };

        ready(Ok(WmsHeaders {
            tenant_id,
            is_guess,
        }))
    }
}
