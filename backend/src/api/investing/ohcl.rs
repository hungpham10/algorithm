use std::sync::Arc;

use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Result};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use vnscope::actors::price::{GetOHCLCommand, UpdateOHCLToCacheCommand};
use vnscope::schemas::CandleStick;

use crate::api::AppState;

#[derive(Deserialize, Debug)]
pub struct OhclRequest {
    resolution: String,
    symbol: String,
    from: i64,
    to: i64,
    limit: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OhclResponse {
    error: Option<String>,
    ohcl: Option<Vec<CandleStick>>,
}

async fn update_ohcl_cache_and_return(
    appstate: &Data<Arc<AppState>>,
    args: &Query<OhclRequest>,
    candles: &Vec<CandleStick>,
) -> Result<HttpResponse> {
    match appstate
        .price
        .send(UpdateOHCLToCacheCommand {
            resolution: args.resolution.clone(),
            stock: args.symbol.clone(),
            candles: candles.clone(),
        })
        .await
    {
        Err(error) => {
            error!("Fail to update OHCL to cache: {}", error);

            Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                error: Some(format!("Failed to update OHCL to cache: {}", error)),
            }))
        }
        Ok(Ok(_)) => {
            debug!("Update caching to optimize performance successfully");
            let mut result = Vec::new();

            for candle in candles {
                if candle.t >= args.from as i32 && candle.t <= args.to as i32 {
                    result.push(candle.clone());
                }
                if args.limit > 0 && result.len() > args.limit {
                    break;
                }
            }

            Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: Some(result),
                error: None,
            }))
        }
        Ok(Err(error)) => {
            error!("Fail to update OHCL to cache: {}", error);

            Ok(HttpResponse::ServiceUnavailable().json(OhclResponse {
                ohcl: None,
                error: Some(error.message),
            }))
        }
    }
}

pub async fn ohcl(
    appstate: Data<Arc<AppState>>,
    path: Path<(String,)>,
    args: Query<OhclRequest>,
) -> Result<HttpResponse> {
    let broker = path.into_inner().0; // Extract broker from path tuple

    match appstate
        .price
        .send(GetOHCLCommand {
            resolution: args.resolution.clone(),
            stock: args.symbol.clone(),
            from: args.from,
            to: args.to,
            broker: broker,
            limit: args.limit,
        })
        .await
    {
        Ok(Ok((candles, is_from_source))) => {
            debug!("Successful return OHCL to client");

            if is_from_source {
                update_ohcl_cache_and_return(&appstate, &args, &candles).await
            } else {
                let mut result = Vec::new();

                for candle in candles {
                    if candle.t >= args.from as i32 && candle.t <= args.to as i32 {
                        result.push(candle.clone());
                    }
                    if args.limit > 0 && result.len() > args.limit {
                        break;
                    }
                }

                Ok(HttpResponse::Ok().json(OhclResponse {
                    ohcl: Some(result),
                    error: None,
                }))
            }
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
