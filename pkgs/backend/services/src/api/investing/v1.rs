use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::io::ErrorKind;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use http::header;

use analysis::VolumeProfile;
use models::cache::Cache;
use models::entities::admin::ApiType;
use schemas::CandleStick;

use super::{AppState, InvestingHeaders};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ohcl/candles/{broker}/{symbol}", get(get_ohcl_from_broker))
        .route(
            "/ohcl/heatmap/{broker}/{symbol}",
            get(get_heatmap_from_broker),
        )
        .route("/ohcl/resolution", get(get_list_of_resolutions))
        .route("/ohcl/brokers", get(get_list_of_brokers))
        .route("/ohcl/brokers/{broker}/all", get(get_list_of_symbols))
        .route(
            "/ohcl/symbols/{broker}/{product}",
            get(get_list_of_symbols_by_product),
        )
        .route(
            "/ohcl/products/{broker}",
            get(get_list_of_product_by_broker),
        )
}

#[derive(Deserialize, Debug)]
struct GetOhclRequest {
    resolution: String,
    from: i64,
    to: i64,
    limit: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct HeatmapResponse {
    heatmap: Vec<Vec<f64>>,
    levels: Vec<f64>,
    ranges: Vec<(usize, usize, usize)>,
    timelines: Vec<Vec<Vec<(usize, usize)>>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct RecapResponse {
    price: Vec<f64>,
    volume: Vec<f64>,
    price_plus1: Vec<f64>,
    price_plus2: Vec<f64>,
    price_plus3: Vec<f64>,
    volume_plus1: Vec<f64>,
    volume_plus2: Vec<f64>,
    volume_plus3: Vec<f64>,
    price_minus1: Vec<f64>,
    price_minus2: Vec<f64>,
    price_minus3: Vec<f64>,
    volume_minus1: Vec<f64>,
    volume_minus2: Vec<f64>,
    volume_minus3: Vec<f64>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
struct OhclResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    heatmap: Option<HeatmapResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ohcl: Option<Vec<CandleStick>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    resolutions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    recap: Option<RecapResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    brokers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    products: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<i32>,
}

async fn get_ohcl_from_broker(
    State(app_state): State<AppState>,
    Path((broker, symbol)): Path<(String, String)>,
    Query(args): Query<GetOhclRequest>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let broker = broker.to_lowercase();
    let symbol = symbol.to_uppercase();

    match app_state
        .investing_entity
        .convert_to_broker_resolution(tenant_id.into(), &broker, &args.resolution)
        .await
    {
        Ok(resolution) => {
            match app_state
                .query_candlesticks
                .get_candlesticks(
                    &broker,
                    &symbol,
                    &resolution,
                    args.from,
                    args.to,
                    args.limit,
                )
                .await
            {
                Ok(candles) => (
                    StatusCode::OK,
                    Json(OhclResponse {
                        ohcl: Some(candles),
                        ..Default::default()
                    }),
                ),
                Err(e) => {
                    // Xử lý lỗi (ví dụ: Provider không tồn tại, lỗi mạng, v.v.)
                    let status = match e.kind() {
                        ErrorKind::NotFound => StatusCode::NOT_FOUND,
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    };

                    (
                        status,
                        Json(OhclResponse {
                            error: Some(format!("Failed to fetch OHLC: {}", e)),
                            ..Default::default()
                        }),
                    )
                }
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("Failed to convert resolution: {}", error)),
                ..Default::default()
            }),
        ),
    }
}

#[derive(Deserialize, Debug)]
pub struct HeatmapRequest {
    resolution: String,
    now: i64,
    look_back: i64,
    overlap: usize,
    number_of_levels: usize,
    interval_in_hour: i32,
}

async fn get_heatmap_from_broker(
    State(app_state): State<AppState>,
    Path((broker, symbol)): Path<(String, String)>,
    Query(args): Query<HeatmapRequest>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let to = args.now;
    let from = match args.resolution.as_str() {
        "1H" => to - 60 * 60 * args.look_back,
        "1D" => to - 24 * 60 * 60 * args.look_back,
        "1W" => to - 7 * 24 * 60 * 60 * args.look_back,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OhclResponse {
                    error: Some(format!("Not support resolution `{}`", args.resolution)),
                    ..Default::default()
                }),
            );
        }
    };

    let resolution = match args.resolution.as_str() {
        "1H" => "1m",
        "1D" => "1H",
        "1W" => "1D",
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OhclResponse {
                    error: Some(format!("Not support resolution `{}`", args.resolution)),
                    ..Default::default()
                }),
            );
        }
    }
    .to_string();

    match app_state
        .investing_entity
        .convert_to_broker_resolution(tenant_id.into(), &broker, &resolution)
        .await
    {
        Ok(resolution) => {
            match app_state
                .query_candlesticks
                .get_candlesticks(&broker, &symbol, &resolution, from, to, 0)
                .await
            {
                Ok(candles) => {
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
                        Ok(vp) => {
                            let cols = vp.heatmap().len();
                            let rows = args.number_of_levels;
                            let mut heatmap: Vec<Vec<f64>> = vec![vec![0.0; cols]; rows];

                            for (j, profile) in vp.heatmap().iter().enumerate() {
                                for (row, &value) in
                                    heatmap.iter_mut().zip(profile.iter()).take(rows)
                                {
                                    row[j] = value;
                                }
                            }

                            (
                                StatusCode::OK,
                                Json(OhclResponse {
                                    heatmap: Some(HeatmapResponse {
                                        heatmap,
                                        levels: vp.levels().clone(),
                                        ranges: vp.ranges().clone(),
                                        timelines: vp.timelines().clone(),
                                    }),
                                    ..Default::default()
                                }),
                            )
                        }
                        Err(error) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(OhclResponse {
                                error: Some(format!(
                                    "Failed to calculate volume profile: {}",
                                    error
                                )),
                                ..Default::default()
                            }),
                        ),
                    }
                }
                Err(error) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(OhclResponse {
                        error: Some(format!("Failed to fetch OHLC: {}", error)),
                        ..Default::default()
                    }),
                ),
            }
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("Failed to convert resolution: {}", error)),
                ..Default::default()
            }),
        ),
    }
}

async fn get_list_of_product_by_broker(
    State(app_state): State<AppState>,
    Path(broker): Path<String>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let broker = broker.to_lowercase();

    match app_state
        .investing_entity
        .list_products(tenant_id.into(), &broker)
        .await
    {
        Ok(products) => (
            StatusCode::OK,
            Json(OhclResponse {
                products: Some(products),
                ..Default::default()
            }),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("Failed to get list of products: {}", error)),
                ..Default::default()
            }),
        ),
    }
}

async fn get_list_of_resolutions(
    State(app_state): State<AppState>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    match app_state
        .investing_entity
        .list_resolutions(tenant_id.into())
        .await
    {
        Ok(resolutions) => (
            StatusCode::OK,
            Json(OhclResponse {
                resolutions: Some(resolutions),
                ..Default::default()
            }),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("Fail to query database: {}", error)),
                ..Default::default()
            }),
        ),
    }
}

#[derive(Deserialize, Clone, Debug)]
struct ListBrokersRequest {
    after: Option<i32>,
    limit: Option<u64>,
}

async fn get_list_of_brokers(
    State(app_state): State<AppState>,
    Query(args): Query<ListBrokersRequest>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let limit = args.limit.unwrap_or(100);
    let after = args.after.unwrap_or(0);

    match app_state
        .investing_entity
        .list_brokers(tenant_id.into(), after, limit)
        .await
    {
        Ok((brokers, next)) => (
            StatusCode::OK,
            Json(OhclResponse {
                brokers: Some(brokers.clone()),
                next: if next > 0 && brokers.len() == limit as usize {
                    Some(next)
                } else {
                    None
                },
                ..Default::default()
            }),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("Fail to query database: {}", error)),
                ..Default::default()
            }),
        ),
    }
}

async fn get_list_of_symbols(
    State(app_state): State<AppState>,
    Path(broker): Path<String>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let tenant_id: i64 = tenant_id.into();
    let cache = Cache::new(app_state.connector.clone(), tenant_id);

    let api_mapping = match app_state.investing_apis.get("get_list_of_symbols") {
        Some(name) => name,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OhclResponse {
                    error: Some("API mapping configuration missing".to_string()),
                    ..Default::default()
                }),
            )
                .into_response();
        }
    };

    // Key này đại diện cho nguyên một cục OhclResponse đã được serialize
    let key = format!("res:{api_mapping}:{tenant_id}:{broker}");

    // --- [FAST PATH] ---
    if let Ok(cached) = cache.get(&key).await {
        return fast_cache_response(cached).into_response();
    }

    // --- [CACHE MISS PATH] ---
    match app_state
        .investing_entity
        .is_broker_enabled(tenant_id, &broker)
        .await
    {
        Ok(true) => {
            let mut headers = HashMap::new();
            if let Ok(token) = app_state
                .admin_entity
                .get_unencrypted_token(tenant_id, &broker)
                .await
            {
                headers.insert("Authorization".to_string(), format!("Bearer {}", token));
            }

            let api_name = format!("{api_mapping}:{broker}");
            match app_state
                .admin_entity
                .perform_api_by_api_name(tenant_id, &api_name, ApiType::Read, vec![], headers, None)
                .await
            {
                Ok(data) => {
                    let symbols: Vec<String> = data
                        .into_iter()
                        .filter_map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| Some(v.to_string()))
                        })
                        .collect();

                    let res_obj = OhclResponse {
                        symbols: Some(symbols),
                        ..Default::default()
                    };

                    // Serialize nguyên struct OhclResponse để lần sau "trả thẳng"
                    if let Ok(serialized) = serde_json::to_string(&res_obj) {
                        let _ = cache.set(&key, &serialized, 3600).await;
                        return fast_cache_response(serialized).into_response();
                    }

                    Json(res_obj).into_response()
                }
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    Json(OhclResponse {
                        error: Some(format!("Broker query failed: {e}")),
                        ..Default::default()
                    }),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::FORBIDDEN,
            Json(OhclResponse {
                error: Some(format!("Broker {broker} is blocked")),
                ..Default::default()
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("DB error: {e}")),
                ..Default::default()
            }),
        )
            .into_response(),
    }
}

async fn get_list_of_symbols_by_product(
    State(app_state): State<AppState>,
    Path((broker, product)): Path<(String, String)>,
    InvestingHeaders { tenant_id }: InvestingHeaders,
) -> impl IntoResponse {
    let tenant_id: i64 = tenant_id.into();
    let cache = Cache::new(app_state.connector.clone(), tenant_id);
    let func = format!("get_list_of_symbols_by_{product}_in_{broker}");

    // Key bao gồm cả product để tránh đè dữ liệu
    let key = format!("res:{func}:{tenant_id}:{broker}");

    // --- [FAST PATH] ---
    if let Ok(cached) = cache.get(&key).await {
        return fast_cache_response(cached).into_response();
    }

    // --- [CACHE MISS PATH] ---
    match app_state
        .investing_entity
        .is_product_enabled(tenant_id, &product, &broker)
        .await
    {
        Ok(true) => {
            let api_name = match app_state.investing_apis.get(&func) {
                Some(name) => name,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(OhclResponse {
                            error: Some(format!("API {func} not found")),
                            ..Default::default()
                        }),
                    )
                        .into_response();
                }
            };

            let mut headers = HashMap::new();
            if let Ok(token) = app_state
                .admin_entity
                .get_unencrypted_token(tenant_id, &broker)
                .await
            {
                headers.insert("Authorization".to_string(), format!("Bearer {}", token));
            }

            match app_state
                .admin_entity
                .perform_api_by_api_name(tenant_id, api_name, ApiType::Read, vec![], headers, None)
                .await
            {
                Ok(data) => {
                    let symbols: Vec<String> = data
                        .into_iter()
                        .filter_map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| Some(v.to_string()))
                        })
                        .collect();

                    let res_obj = OhclResponse {
                        symbols: Some(symbols),
                        ..Default::default()
                    };

                    if let Ok(serialized) = serde_json::to_string(&res_obj) {
                        let _ = cache.set(&key, &serialized, 3600).await;
                        return fast_cache_response(serialized).into_response();
                    }

                    Json(res_obj).into_response()
                }
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    Json(OhclResponse {
                        error: Some(format!("Query failed: {e}")),
                        ..Default::default()
                    }),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::FORBIDDEN,
            Json(OhclResponse {
                error: Some("Product or Broker blocked".into()),
                ..Default::default()
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OhclResponse {
                error: Some(format!("DB error: {e}")),
                ..Default::default()
            }),
        )
            .into_response(),
    }
}

fn fast_cache_response(cached_json: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cached_json))
        .unwrap()
}
