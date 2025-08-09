use std::sync::Arc;

use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Result};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use vnscope::actors::price::GetOHCLCommand;
use vnscope::schemas::CandleStick;

use crate::api::AppState;

#[derive(Deserialize, Debug)]
pub struct OhclRequest {
    resolution: String,
    symbol: String,
    from: i64,
    to: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OhclResponse {
    error: Option<String>,
    ohcl: Option<Vec<CandleStick>>,
}

pub async fn ohcl(appstate: Data<Arc<AppState>>, args: Query<OhclRequest>) -> Result<HttpResponse> {
    match appstate
        .price
        .send(GetOHCLCommand {
            resolution: args.resolution.clone(),
            stock: args.symbol.clone(),
            from: args.from,
            to: args.to,
        })
        .await
    {
        Ok(Ok(ohcl)) => {
            debug!("Successful return OHCL to client");

            Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: Some(ohcl.clone()),
                error: None,
            }))
        }
        Ok(Err(error)) => {
            error!("Fail to query OHCL: {}", error);

            Ok(HttpResponse::ServiceUnavailable().json(OhclResponse {
                ohcl: None,
                error: Some(error.message),
            }))
        }
        Err(error) => {
            error!("Fail to query OHCL: {}", error);

            Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                error: Some(format!("Failed to query OHCL: {}", error)),
            }))
        }
    }
}
