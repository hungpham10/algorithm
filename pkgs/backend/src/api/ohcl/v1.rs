use std::collections::HashMap;
use std::fmt::{Display, Error as FmtError, Formatter, Result as FmtResult};
use std::sync::Arc;

use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Result};

use lazy_static::lazy_static;
use log::{debug, error};
use serde::{Deserialize, Serialize};

use vnscope::actors::price::{GetOHCLCommand, UpdateOHCLToCacheCommand};
use vnscope::actors::{
    list_crypto, list_futures, list_of_hose, list_of_industry, list_of_midcap, list_of_penny,
    list_of_vn100, list_of_vn30,
};
use vnscope::algorithm::VolumeProfile;
use vnscope::schemas::CandleStick;

use crate::api::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct HeatmapResponse {
    heatmap: Vec<Vec<f64>>,
    levels: Vec<f64>,
    ranges: Vec<(usize, usize, usize)>,
}

lazy_static! {
    static ref INDUSTRY_CODES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("petroleum", "0500");
        m.insert("chemical", "1300");
        m.insert("basic resources", "1700");
        m.insert("construction & building materials", "2300");
        m.insert("industrial goods & services", "2700");
        m.insert("cars & car parts", "3300");
        m.insert("food & beverage", "3500");
        m.insert("personal & household goods", "3700");
        m.insert("medical", "4500");
        m.insert("retail", "5300");
        m.insert("communication", "5500");
        m.insert("travel & entertainment", "5700");
        m.insert("telecomunication", "6500");
        m.insert("electricity, water & petrol", "7500");
        m.insert("banking", "8300");
        m.insert("insurance", "8500");
        m.insert("real estate", "8600");
        m.insert("finance service", "8700");
        m.insert("information technology", "9500");
        m
    };
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OhclResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap: Option<HeatmapResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ohcl: Option<Vec<CandleStick>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub brokers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<i32>,
}

impl Display for OhclResponse {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let json = serde_json::to_string(self).map_err(|_| FmtError)?;
        f.write_str(&json)
    }
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

            Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
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
            let mut next = None;

            for candle in candles {
                if candle.t >= args.from as i32 && candle.t <= args.to as i32 {
                    result.push(candle.clone());
                }
                if args.limit > 0 && result.len() > args.limit {
                    next = Some(candle.t);
                    break;
                }
            }

            Ok(HttpResponse::Ok().json(OhclResponse {
                ohcl: Some(result),
                heatmap: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                products: None,
                error: None,
                next,
            }))
        }
        Ok(Err(error)) => Err(ErrorServiceUnavailable(OhclResponse {
            ohcl: None,
            heatmap: None,
            brokers: None,
            symbols: None,
            resolutions: None,
            products: None,
            next: None,
            error: Some(error.message),
        })),
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

    if args.from >= args.to {
        return Err(ErrorBadRequest(OhclResponse {
            ohcl: None,
            heatmap: None,
            brokers: None,
            symbols: None,
            products: None,
            resolutions: None,
            next: None,
            error: Some(format!("From({}) shouldn't >= To({})", args.from, args.to)),
        }));
    }

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
                    limit: 0, // @NOTE: call to fetch full data from actor
                })
                .await
            {
                Ok(Ok((candles, is_from_source))) => {
                    debug!("Successful return OHCL to client");

                    if is_from_source {
                        update_ohcl_cache_and_return(&appstate, &symbol, &args, &candles).await
                    } else {
                        let mut result = Vec::new();
                        let mut next = None;

                        for candle in candles {
                            if candle.t >= args.from as i32 && candle.t <= args.to as i32 {
                                result.push(candle.clone());
                            }
                            if args.limit > 0 && result.len() > args.limit {
                                next = Some(candle.t);
                                break;
                            }
                        }

                        Ok(HttpResponse::Ok().json(OhclResponse {
                            ohcl: Some(result),
                            heatmap: None,
                            brokers: None,
                            symbols: None,
                            resolutions: None,
                            products: None,
                            error: None,
                            next,
                        }))
                    }
                }
                Ok(Err(error)) => {
                    error!("Fail to query OHCL: {}", error);

                    Err(ErrorServiceUnavailable(OhclResponse {
                        ohcl: None,
                        heatmap: None,
                        brokers: None,
                        symbols: None,
                        resolutions: None,
                        products: None,
                        next: None,
                        error: Some(error.message),
                    }))
                }
                Err(error) => Err(ErrorInternalServerError(OhclResponse {
                    ohcl: None,
                    heatmap: None,
                    brokers: None,
                    symbols: None,
                    products: None,
                    resolutions: None,
                    next: None,
                    error: Some(format!("Failed to query OHCL: {}", error)),
                })),
            },
            Err(error) => Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Fail to query database: {}", error)),
            })),
        }
    } else {
        Err(ErrorInternalServerError(OhclResponse {
            ohcl: None,
            heatmap: None,
            brokers: None,
            symbols: None,
            products: None,
            resolutions: None,
            next: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

#[derive(Deserialize, Debug)]
pub struct HeatmapRequest {
    resolution: String,
    now: i64,
    lookback: i64,
    overlap: usize,
    number_of_levels: usize,
    interval_in_hour: i32,
}

pub async fn get_heatmap_from_broker(
    appstate: Data<Arc<AppState>>,
    path: Path<(String, String)>,
    args: Query<HeatmapRequest>,
) -> Result<HttpResponse> {
    let (broker, symbol) = path.into_inner();
    let to = args.now;
    let from = match args.resolution.as_str() {
        "1D" => to - 24 * 60 * 60 * args.lookback,
        "1W" => to - 7 * 24 * 60 * 60 * args.lookback,
        _ => {
            return Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Not support resolution `{}`", args.resolution,)),
            }));
        }
    };

    if let Some(entity) = appstate.ohcl_entity() {
        match entity
            .convert_to_broker_resolution(&broker, &args.resolution)
            .await
        {
            Ok(resolution) => match appstate
                .price
                .send(GetOHCLCommand {
                    resolution: resolution.clone(),
                    stock: symbol.clone(),
                    from: from,
                    to: to,
                    broker: broker,
                    limit: 0,
                })
                .await
            {
                Ok(Ok((candles, is_from_source))) => {
                    if is_from_source {
                        match appstate
                            .price
                            .send(UpdateOHCLToCacheCommand {
                                resolution: resolution.clone(),
                                stock: symbol.clone(),
                                candles: candles.clone(),
                            })
                            .await
                        {
                            Ok(Ok(_)) => {
                                match VolumeProfile::new_from_candles(
                                    candles
                                        .iter()
                                        .filter_map(|candle| {
                                            if candle.t >= from as i32 && candle.t <= to as i32 {
                                                Some(candle.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .as_slice(),
                                    args.number_of_levels,
                                    args.overlap,
                                    args.interval_in_hour,
                                ) {
                                    Ok(vp) => Ok(HttpResponse::Ok().json(OhclResponse {
                                        ohcl: None,
                                        heatmap: Some(HeatmapResponse {
                                            heatmap: vp.heatmap().clone(),
                                            levels: vp.levels().clone(),
                                            ranges: vp.ranges().clone(),
                                        }),
                                        brokers: None,
                                        symbols: None,
                                        resolutions: None,
                                        products: None,
                                        next: None,
                                        error: None,
                                    })),
                                    Err(error) => Err(ErrorServiceUnavailable(OhclResponse {
                                        ohcl: None,
                                        heatmap: None,
                                        brokers: None,
                                        symbols: None,
                                        resolutions: None,
                                        products: None,
                                        next: None,
                                        error: Some(format!(
                                            "Calculate VolumeProfile got error: {:?}",
                                            error
                                        )),
                                    })),
                                }
                            }
                            Ok(Err(error)) => Err(ErrorServiceUnavailable(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                resolutions: None,
                                products: None,
                                next: None,
                                error: Some(error.message),
                            })),
                            Err(error) => Err(ErrorInternalServerError(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Failed to update OHCL to cache: {}", error)),
                            })),
                        }
                    } else {
                        match VolumeProfile::new_from_candles(
                            candles
                                .iter()
                                .filter_map(|candle| {
                                    if candle.t >= from as i32 && candle.t <= to as i32 {
                                        Some(candle.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
                            args.number_of_levels,
                            args.overlap,
                            args.interval_in_hour,
                        ) {
                            Ok(vp) => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: Some(HeatmapResponse {
                                    heatmap: vp.heatmap().clone(),
                                    levels: vp.levels().clone(),
                                    ranges: vp.ranges().clone(),
                                }),
                                brokers: None,
                                symbols: None,
                                resolutions: None,
                                products: None,
                                next: None,
                                error: None,
                            })),
                            Err(error) => Err(ErrorServiceUnavailable(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                resolutions: None,
                                products: None,
                                next: None,
                                error: Some(format!(
                                    "Calculate VolumeProfile got error: {:?}",
                                    error
                                )),
                            })),
                        }
                    }
                }
                Ok(Err(error)) => Err(ErrorServiceUnavailable(OhclResponse {
                    ohcl: None,
                    heatmap: None,
                    brokers: None,
                    symbols: None,
                    resolutions: None,
                    products: None,
                    next: None,
                    error: Some(error.message),
                })),
                Err(error) => Err(ErrorInternalServerError(OhclResponse {
                    ohcl: None,
                    heatmap: None,
                    brokers: None,
                    symbols: None,
                    products: None,
                    resolutions: None,
                    next: None,
                    error: Some(format!("Failed to query OHCL: {}", error)),
                })),
            },
            Err(error) => Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Fail to query database: {}", error)),
            })),
        }
    } else {
        Err(ErrorInternalServerError(OhclResponse {
            ohcl: None,
            heatmap: None,
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
                heatmap: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: Some(resolutions),
                next: None,
                error: None,
            })),
            Err(error) => Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
                brokers: None,
                symbols: None,
                products: None,
                resolutions: None,
                next: None,
                error: Some(format!("Fail to query database: {}", error)),
            })),
        }
    } else {
        Err(ErrorInternalServerError(OhclResponse {
            ohcl: None,
            heatmap: None,
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
                heatmap: None,
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

    Err(ErrorInternalServerError(OhclResponse {
        ohcl: None,
        heatmap: None,
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
                        "stock" => Err(ErrorInternalServerError(OhclResponse {
                            ohcl: None,
                            heatmap: None,
                            brokers: None,
                            symbols: Some(list_of_hose().await),
                            resolutions: None,
                            next: None,
                            products: None,
                            error: Some(format!("Not implemented")),
                        })),
                        "crypto" => Err(ErrorInternalServerError(OhclResponse {
                            ohcl: None,
                            heatmap: None,
                            brokers: None,
                            symbols: Some(list_crypto().await),
                            resolutions: None,
                            products: None,
                            next: None,
                            error: Some(format!("Not implemented")),
                        })),
                        &_ => Err(ErrorInternalServerError(OhclResponse {
                            ohcl: None,
                            heatmap: None,
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

    Err(ErrorInternalServerError(OhclResponse {
        ohcl: None,
        heatmap: None,
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
                heatmap: None,
                brokers: None,
                symbols: None,
                products: Some(products),
                resolutions: None,
                next: None,
                error: None,
            })),
            Err(error) => Err(ErrorInternalServerError(OhclResponse {
                ohcl: None,
                heatmap: None,
                brokers: None,
                symbols: None,
                resolutions: None,
                next: None,
                products: None,
                error: Some(format!("Failed to get list of products: {}", error)),
            })),
        }
    } else {
        Err(ErrorInternalServerError(OhclResponse {
            ohcl: None,
            heatmap: None,
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
                            "cw" => Err(ErrorInternalServerError(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                resolutions: None,
                                next: None,
                                products: None,
                                error: Some(format!("Not implemented")),
                            })),
                            "vn30" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_of_vn30().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "vn100" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_of_vn100().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "midcap" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_of_midcap().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "penny" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_of_penny().await),
                                resolutions: None,
                                next: None,
                                products: None,
                                error: None,
                            })),
                            "future" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_futures().await),
                                resolutions: None,
                                products: None,
                                next: None,
                                error: None,
                            })),
                            &_ => {
                                if let Some(_) = INDUSTRY_CODES.get(product.as_str()) {
                                    let symbols = list_of_industry(&product).await;
                                    Ok(HttpResponse::Ok().json(OhclResponse {
                                        ohcl: None,
                                        heatmap: None,
                                        brokers: None,
                                        symbols: Some(symbols),
                                        resolutions: None,
                                        next: None,
                                        products: None,
                                        error: None,
                                    }))
                                } else {
                                    Err(ErrorInternalServerError(OhclResponse {
                                        ohcl: None,
                                        heatmap: None,
                                        brokers: None,
                                        symbols: None,
                                        products: None,
                                        resolutions: None,
                                        next: None,
                                        error: Some(format!("Product {} is not exist", product)),
                                    }))
                                }
                            }
                        },
                        "crypto" => match product.as_str() {
                            "spot" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: Some(list_crypto().await),
                                products: None,
                                resolutions: None,
                                next: None,
                                error: None,
                            })),
                            "future" => Ok(HttpResponse::Ok().json(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Not implemented")),
                            })),
                            &_ => Err(ErrorInternalServerError(OhclResponse {
                                ohcl: None,
                                heatmap: None,
                                brokers: None,
                                symbols: None,
                                products: None,
                                resolutions: None,
                                next: None,
                                error: Some(format!("Product {} is not exist", product)),
                            })),
                        },
                        &_ => Err(ErrorInternalServerError(OhclResponse {
                            ohcl: None,
                            heatmap: None,
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

    Err(ErrorInternalServerError(OhclResponse {
        ohcl: None,
        heatmap: None,
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
