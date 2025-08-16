use std::sync::Arc;

use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Result};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use vnscope::actors::price::{GetOHCLCommand, UpdateOHCLToCacheCommand};
use vnscope::schemas::CandleStick;

use crate::api::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OhclResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ohcl: Option<Vec<CandleStick>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    resolutions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    brokers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<Vec<String>>,
}

async fn update_ohcl_cache_and_return(
    appstate: &Data<Arc<AppState>>,
    symbol: &String,
    args: &Query<OhclRequest>,
    candles: &Vec<CandleStick>,
) -> Result<HttpResponse> {
    match appstate
        .price
        .send(UpdateOHCLToCacheCommand {
            resolution: args.resolution.clone(),
            stock: symbol.clone(),
            candles: candles.clone(),
        })
        .await
    {
        Err(error) => {
            error!("Fail to update OHCL to cache: {}", error);

            Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                resolutions: None,
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
                brokers: None,
                symbols: None,
                resolutions: None,
                error: None,
            }))
        }
        Ok(Err(error)) => {
            error!("Fail to update OHCL to cache: {}", error);

            Ok(HttpResponse::ServiceUnavailable().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                error: Some(error.message),
            }))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct OhclRequest {
    resolution: String,
    from: i64,
    to: i64,
    limit: usize,
}

pub async fn get_ohcl_from_broker(
    appstate: Data<Arc<AppState>>,
    path: Path<(String, String)>,
    args: Query<OhclRequest>,
) -> Result<HttpResponse> {
    let (broker, symbol) = path.into_inner();

    match appstate
        .price
        .send(GetOHCLCommand {
            resolution: args.resolution.clone(),
            stock: symbol.clone(),
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
                update_ohcl_cache_and_return(&appstate, &symbol, &args, &candles).await
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
                    brokers: None,
                    symbols: None,
                    resolutions: None,
                    error: None,
                }))
            }
        }
        Ok(Err(error)) => {
            error!("Fail to query OHCL: {}", error);

            Ok(HttpResponse::ServiceUnavailable().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                error: Some(error.message),
            }))
        }
        Err(error) => {
            error!("Fail to query OHCL: {}", error);

            Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                error: Some(format!("Failed to query OHCL: {}", error)),
            }))
        }
    }
}

pub async fn get_list_of_resolutions(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        resolutions: None,
        error: Some(format!("Not implemented")),
    }))
}

pub async fn get_list_of_brokers(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        resolutions: None,
        error: Some(format!("Not implemented")),
    }))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SymbolResquest {
    group: Option<String>,
}

pub async fn get_list_of_symbols(
    appstate: Data<Arc<AppState>>,
    path: Path<(String,)>,
    args: Query<SymbolResquest>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        resolutions: None,
        error: Some(format!("Not implemented")),
    }))
}
