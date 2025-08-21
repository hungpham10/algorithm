use std::sync::Arc;

use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Result};

use log::{debug, error};
use serde::{Deserialize, Serialize};

use vnscope::actors::price::{GetOHCLCommand, UpdateOHCLToCacheCommand};
use vnscope::actors::{
    list_crypto, list_futures, list_of_hose, list_of_midcap, list_of_penny, list_of_vn100,
    list_of_vn30,
};
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
    products: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<i32>,
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
                products: None,
                resolutions: None,
                next: None,
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
                products: None,
                next: None,
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
                products: None,
                next: None,
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

    if let Some(entity) = appstate.ohcl_entity() {
        match entity
            .convert_to_broker_resolution(&broker, &args.resolution)
            .await
        {
            Ok(resolution) => match appstate
                .price
                .send(GetOHCLCommand {
                    resolution: resolution,
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
                            next: None,
                            products: None,
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
                        products: None,
                        next: None,
                        error: Some(error.message),
                    }))
                }
                Err(error) => {
                    error!("Fail to query OHCL: {}", error);

                    Ok(HttpResponse::InternalServerError().json(OhclResponse {
                        ohcl: None,
                        brokers: None,
                        symbols: None,
                        products: None,
                        resolutions: None,
                        next: None,
                        error: Some(format!("Failed to query OHCL: {}", error)),
                    }))
                }
            },
            Err(error) => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Fail to query database: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(OhclResponse {
            ohcl: None,
            brokers: None,
            symbols: None,
            products: None,
            resolutions: None,
            next: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn get_list_of_resolutions(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    if let Some(entity) = appstate.ohcl_entity() {
        match entity.list_resolutions().await {
            Ok(resolutions) => Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: Some(resolutions),
                next: None,
                error: None,
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Fail to query database: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(OhclResponse {
            ohcl: None,
            brokers: None,
            symbols: None,
            products: None,
            resolutions: None,
            next: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListBrokersRequest {
    after: Option<i32>,
    limit: Option<u64>,
}

pub async fn get_list_of_brokers(
    appstate: Data<Arc<AppState>>,
    args: Query<ListBrokersRequest>,
) -> Result<HttpResponse> {
    let limit = args.limit.unwrap_or_else(|| 10);
    let after = args.after.unwrap_or_else(|| 0);

    if let Some(entity) = appstate.ohcl_entity() {
        if let Ok((brokers, next)) = entity.list_brokers(after, limit).await {
            return Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: None,
                brokers: Some(brokers.clone()),
                symbols: None,
                products: None,
                resolutions: None,
                next: if next > 0 && brokers.len() == limit as usize {
                    Some(next)
                } else {
                    None
                },
                error: None,
            }));
        }
    }

    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        products: None,
        resolutions: None,
        next: None,
        error: Some(format!("Not implemented")),
    }))
}

pub async fn get_list_of_symbols(
    appstate: Data<Arc<AppState>>,
    path: Path<(String,)>,
) -> Result<HttpResponse> {
    let (broker,) = path.into_inner();
    if let Some(entity) = appstate.ohcl_entity() {
        match entity.is_broker_enabled(&broker).await {
            Ok(ok) => {
                if ok {
                    return match broker.as_str() {
                        "stock" => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                            ohcl: None,
                            brokers: None,
                            symbols: Some(list_of_hose().await),
                            resolutions: None,
                            next: None,
                            products: None,
                            error: Some(format!("Not implemented")),
                        })),
                        "crypto" => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                            ohcl: None,
                            brokers: None,
                            symbols: Some(list_crypto().await),
                            resolutions: None,
                            products: None,
                            next: None,
                            error: Some(format!("Not implemented")),
                        })),
                        &_ => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                            ohcl: None,
                            brokers: None,
                            symbols: None,
                            resolutions: None,
                            products: None,
                            next: None,
                            error: Some(format!("Broker {} is not exist", broker)),
                        })),
                    };
                }
            }
            Err(error) => {
                error!("Fail to perform in db: {}", error);
            }
        }
    }

    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        products: None,
        resolutions: None,
        next: None,
        error: Some(format!("Broker {} has been blocked", broker)),
    }))
}

pub async fn get_list_of_product_by_broker(
    appstate: Data<Arc<AppState>>,
    path: Path<(String,)>,
) -> Result<HttpResponse> {
    let (broker,) = path.into_inner();

    if let Some(entity) = appstate.ohcl_entity() {
        match entity.list_products(&broker).await {
            Ok(products) => Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                products: Some(products),
                resolutions: None,
                next: None,
                error: None,
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                ohcl: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                next: None,
                products: None,
                error: Some(format!("Failed to get list of products: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(OhclResponse {
            ohcl: None,
            brokers: None,
            symbols: None,
            resolutions: None,
            next: None,
            products: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn get_list_of_symbols_by_product(
    appstate: Data<Arc<AppState>>,
    path: Path<(String, String)>,
) -> Result<HttpResponse> {
    let (broker, product) = path.into_inner();

    if let Some(entity) = appstate.ohcl_entity() {
        match entity.is_product_enabled(&product, &broker).await {
            Ok(ok) => {
                if ok {
                    // @TODO: replace with another solution to show brokers from out tables

                    return match broker.as_str() {
                        "stock" => match product.as_str() {
                            "cw" => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: None,
                                resolutions: None,
                                next: None,
                                products: None,
                                error: Some(format!("Not implemented")),
                            })),
                            "vn30" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_of_vn30().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "vn100" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_of_vn100().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "midcap" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_of_midcap().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "penny" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_of_penny().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "future" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_futures().await),
                                resolutions: None,
                                products: None,
                                next: None,
                                error: None,
                            })),
                            &_ => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Product {} is not exist", product)),
                            })),
                        },
                        "crypto" => match product.as_str() {
                            "spot" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: Some(list_crypto().await),
                                products: None,
                                resolutions: None,
                                next: None,
                                error: None,
                            })),
                            "future" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Not implemented")),
                            })),
                            &_ => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                                ohcl: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Product {} is not exist", product)),
                            })),
                        },
                        &_ => Ok(HttpResponse::InternalServerError().json(OhclResponse {
                            ohcl: None,
                            brokers: None,
                            symbols: None,
                            resolutions: None,
                            products: None,
                            next: None,
                            error: Some(format!("Broker {} is not exist", broker)),
                        })),
                    };
                }
            }
            Err(error) => {
                error!("Fail to perform in db: {}", error);
            }
        }
    }

    Ok(HttpResponse::InternalServerError().json(OhclResponse {
        ohcl: None,
        brokers: None,
        symbols: None,
        resolutions: None,
        next: None,
        products: None,
        error: Some(format!(
            "Product {} of {} has been blocked",
            product, broker
        )),
    }))
}
