use std::collections::HashMap;
use std::convert::Into;

use axum::Extension;
use axum::Router;
use axum::extract::Json as JsonRequest;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResponse};
use axum::routing::{get, post};

use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Result, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};

use chrono::Utc;
use models::entities::investing::{Filter, Price, Product, Store, Symbol};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{
    IntoParams, Modify, OpenApi, ToSchema,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use super::v1::get_latest_price;
use super::{AppState, InvestingHeaders};

#[derive(OpenApi)]
#[openapi(
    paths(
        list_paginated_products,
        list_price_of_store,
        list_paginated_stores,
        create_stores,
        create_products,
        ingest_price_data,
        get_price_data_by_name,
        get_price_data_by_product_id,
        get_symbol_id_by_product_in_store,
        list_paginated_symbols,
        render_data_using_graphql,
    ),
    components(schemas(
        OhclResponse,
        Price,
        GraphQLRequestDTO,
        GraphQLResponseDTO,
    )),
    modifiers(&SecurityAddon)
)]
pub struct InvestingV2Api;

#[derive(ToSchema, Deserialize)]
#[allow(dead_code)]
struct GraphQLRequestDTO {
    #[schema(example = "query { symbols { id name } }")]
    query: String,

    #[schema(value_type = Option<Object>, example = json!({"limit": 10}))]
    variables: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    operation_name: Option<String>,
}

#[derive(ToSchema, Serialize)]
pub struct GraphQLResponseDTO {
    #[schema(value_type = Option<Object>, example = json!({"symbols": [{"id": 1, "name": "BTC"}]}))]
    pub data: Option<serde_json::Value>,

    #[schema(value_type = Option<Vec<GraphQLErrorDTO>>)]
    pub errors: Option<Vec<GraphQLErrorDTO>>,
}

#[derive(ToSchema, Serialize)]
pub struct GraphQLErrorDTO {
    pub message: String,
    pub locations: Option<Vec<SourceLocationDTO>>,
    pub path: Option<Vec<String>>,
}

#[derive(ToSchema, Serialize)]
pub struct SourceLocationDTO {
    pub line: usize,
    pub column: usize,
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, IntoParams, ToSchema)]
pub struct QueryPagingInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, ToSchema)]
pub struct ListStore {
    data: Vec<Store>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, ToSchema)]
pub struct ListSymbols {
    data: Vec<Symbol>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i32>,
}
#[derive(Deserialize, Serialize, Clone, Debug, Default, ToSchema)]
struct OhclResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stores: Option<ListStore>,

    #[serde(skip_serializing_if = "Option::is_none")]
    store: Option<Store>,

    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<Price>,

    #[serde(skip_serializing_if = "Option::is_none")]
    prices: Option<HashMap<String, Price>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<ListSymbols>,
}

pub fn routes() -> Router<AppState> {
    // @TODO: cần api để cập nhật product với store cụ thể không?

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();
    Router::new()
        .route(
            "/stores/{store}/products/{product}/price",
            get(get_price_data_by_name).post(ingest_price_data),
        )
        .route(
            "/stores/{store}/products/{product}/symbol",
            get(get_symbol_id_by_product_in_store),
        )
        .route("/stores/{store}/price", get(list_price_of_store))
        .route("/stores", get(list_paginated_stores).post(create_stores))
        .route(
            "/stores/{store}/products",
            get(list_paginated_products).post(create_products),
        )
        .route("/symbols", get(list_paginated_symbols))
        .route("/prices/{product_id}", get(get_price_data_by_product_id))
        .route("/astra-render", post(render_data_using_graphql))
        .layer(Extension(schema))
}

#[utoipa::path(
    get,
    path = "/symbols",
    params(QueryPagingInput),
    responses((status = 200, body = OhclResponse)),
)]
async fn list_paginated_symbols(
    State(app_state): State<AppState>,
    Query(QueryPagingInput {
        after,
        limit,
        detail,
    }): Query<QueryPagingInput>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<OhclResponse>)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);
    let detail = detail.unwrap_or(false);
    let tenant_id = tenant_id.into();
    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }),
        )
    })?;

    if (limit > 10 && detail) || limit > 100 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("`limit` mustn't be larger than 100".to_string()),
                ..Default::default()
            }),
        ));
    }

    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to list symbols: {error}",)),
                    ..Default::default()
                }),
            )
        })?;

    match app_state
        .investing_entity
        .list_paginated_symbols(tenant_id, &broker, after, limit, detail, None)
        .await
    {
        Ok(data) => {
            let next_after = if data.len() as u64 == limit
                && let Some(item) = data.last()
            {
                item.id
            } else {
                None
            };

            Ok((
                StatusCode::OK,
                JsonResponse(OhclResponse {
                    symbols: Some(ListSymbols { data, next_after }),
                    ..Default::default()
                }),
            ))
        }
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some(format!("Failed listing stores: {error}")),
                ..Default::default()
            }),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/stores/{store}/price",
    params(
        ("store" = String, Path, description = "Store name"),
        ("product" = String, Path, description = "Product name"),
    ),
    responses((status = 200, body = OhclResponse)),
)]
async fn list_price_of_store(
    State(app_state): State<AppState>,
    Path(store): Path<String>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<OhclResponse>)> {
    // @TODO: get broker_id by tenant_id
    let tenant_id = tenant_id.into();
    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }),
        )
    })?;

    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get price of {store}: {error}",)),
                    ..Default::default()
                }),
            )
        })?;

    let store_id = app_state
        .investing_entity
        .get_store_detail(tenant_id, &store, 0, 0, false)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get price of {store}: {error}",)),
                    ..Default::default()
                }),
            )
        })?
        .id
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get price of {store}: return id is null",)),
                    ..Default::default()
                }),
            )
        })?;

    Ok(JsonResponse(OhclResponse {
        prices: Some(
            app_state
                .investing_entity
                .list_current_price_of_store(tenant_id, store_id)
                .await
                .map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        JsonResponse(OhclResponse {
                            error: Some(format!(
                                "Ingest pricing data failed (user {}): {error}",
                                user_id.0.clone().unwrap_or("guest".to_string())
                            )),
                            ..Default::default()
                        }),
                    )
                })?,
        ),
        ..Default::default()
    }))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct PriceQuery {
    degree: Option<f32>,
}

#[utoipa::path(
    get,
    path = "/prices/{product_ids}",
    params(
        ("product_ids" = String, Path, description = "List of Product Id separated by comma"),
        PriceQuery,
    ),
    responses((status = 200, body = [OhclResponse])),
)]
async fn get_price_data_by_product_id(
    State(app_state): State<AppState>,
    Path(product_ids_str): Path<String>,
    Query(PriceQuery { degree }): Query<PriceQuery>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<Vec<OhclResponse>>)> {
    let tenant_id = tenant_id.into();
    let product_ids = product_ids_str
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| s.parse::<i32>().ok())
        .collect::<Vec<_>>();

    if product_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            JsonResponse(vec![OhclResponse {
                error: Some("Invalid product IDs".into()),
                ..Default::default()
            }]),
        ));
    }

    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(vec![OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }]),
        )
    })?;

    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|e| {
            (
                StatusCode::FORBIDDEN,
                JsonResponse(vec![OhclResponse {
                    error: Some(e.to_string()),
                    ..Default::default()
                }]),
            )
        })?;

    let mut price_map = app_state
        .investing_entity
        .get_price(tenant_id, &product_ids, Some(24 * 60 * 60))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(vec![OhclResponse {
                    error: Some(format!("Database error: {}", e)),
                    ..Default::default()
                }]),
            )
        })?;

    Ok(JsonResponse(
        product_ids
            .into_iter()
            .map(|id| {
                if let Some(price) = price_map.remove(&id) {
                    OhclResponse {
                        price: if let Some(degree) = degree {
                            Some(Price {
                                buy: price.buy / degree,
                                sell: price.sell / degree,
                                diff: if let Some((diff_buy, diff_sell)) = price.diff {
                                    Some((diff_buy / degree, diff_sell / degree))
                                } else {
                                    None
                                },
                                ..Default::default()
                            })
                        } else {
                            Some(price)
                        },
                        ..Default::default()
                    }
                } else {
                    OhclResponse {
                        error: Some("Product price not found".to_string()),
                        ..Default::default()
                    }
                }
            })
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get,
    path = "/stores/{store}/products/{product}/price",
    params(
        ("store" = String, Path, description = "Store name"),
        ("product" = String, Path, description = "Product name"),
    ),
    responses((status = 200, body = OhclResponse)),
)]
async fn get_price_data_by_name(
    State(app_state): State<AppState>,
    Path((store, product)): Path<(String, String)>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<OhclResponse>)> {
    let tenant_id = tenant_id.into();
    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }),
        )
    })?;

    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to validate broker: {error}")),
                    ..Default::default()
                }),
            )
        })?;

    let product_id = app_state
        .investing_entity
        .get_product_id_from_website(tenant_id, &store, &product)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get product id for {product}: {error}")),
                    ..Default::default()
                }),
            )
        })?;

    // Lấy HashMap các giá từ backend
    let prices_map = app_state
        .investing_entity
        .get_price(tenant_id, &[product_id], Some(24 * 60 * 60)) // Truyền slice chứa 1 ID
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!(
                        "Ingest pricing data failed (user {}): {error}",
                        user_id.0.clone().unwrap_or("guest".to_string())
                    )),
                    ..Default::default()
                }),
            )
        })?;

    Ok(JsonResponse(OhclResponse {
        price: prices_map.get(&product_id).cloned(),
        ..Default::default()
    }))
}

#[utoipa::path(
    post,
    path = "/stores/{store}/products/{product}/price",
    request_body = Price,
    params(
        ("store" = String, Path, description = "Store name"),
        ("product" = String, Path, description = "Product name"),
    ),
    responses((status = 200, body = OhclResponse)),
    security(
        ("bearer_auth" = [])
    )
)]
async fn ingest_price_data(
    State(app_state): State<AppState>,
    Path((store, product)): Path<(String, String)>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
    JsonRequest(payload): JsonRequest<Price>,
) -> Result<impl IntoResponse, (StatusCode, JsonResponse<OhclResponse>)> {
    // @TODO: get broker_id by tenant_id
    let tenant_id = tenant_id.into();
    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }),
        )
    })?;

    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!(
                        "Failed to ingest data to {product} of {store}: {error}",
                    )),
                    ..Default::default()
                }),
            )
        })?;

    let product_id = app_state
        .investing_entity
        .get_product_id_from_website(tenant_id, &store, &product)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!(
                        "Failed to ingest data to {product} of {store}: {error}",
                    )),
                    ..Default::default()
                }),
            )
        })?;

    app_state
        .investing_entity
        .update_price(product_id, payload)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!(
                        "Ingest pricing data failed (user {}): {error}",
                        user_id.0.clone().unwrap_or("guest".to_string())
                    )),
                    ..Default::default()
                }),
            )
        })?;

    Ok(JsonResponse(OhclResponse {
        ..Default::default()
    }))
}

#[utoipa::path(
    get,
    path = "/stores/{store}/products/{product}/symbol",
    params(
        ("store" = String, Path, description = "Store name"),
        ("product" = String, Path, description = "Product name"),
    ),
    responses(
        (status = 200, description = "Success", body = OhclResponse),
        (status = 404, description = "Broker or Symbol not found", body = OhclResponse),
        (status = 500, description = "Internal Server Error", body = OhclResponse),
    )
)]
async fn get_symbol_id_by_product_in_store(
    State(app_state): State<AppState>,
    Path((store, product)): Path<(String, String)>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let symbol = app_state
        .investing_entity
        .store_product_name_to_symbol(tenant_id.into(), &store, &product)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Not found {product} in store {store}: {error}")),
                    ..Default::default()
                }),
            )
        })?;

    Ok((
        StatusCode::OK,
        JsonResponse(OhclResponse {
            symbol: Some(symbol),
            ..Default::default()
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/stores",
    params(QueryPagingInput),
    responses((status = 200, body = OhclResponse))
)]
async fn list_paginated_stores(
    State(app_state): State<AppState>,
    Query(QueryPagingInput {
        after,
        limit,
        detail,
    }): Query<QueryPagingInput>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);
    let detail = detail.unwrap_or(false);

    if (limit > 10 && detail) || limit > 100 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("`limit` mustn't be larger than 100".to_string()),
                ..Default::default()
            }),
        ));
    }

    match app_state
        .investing_entity
        .list_paginated_stores(tenant_id.into(), after, limit, detail)
        .await
    {
        Ok(data) => {
            let next_after = if data.len() as u64 == limit
                && let Some(item) = data.last()
            {
                item.id
            } else {
                None
            };

            Ok((
                StatusCode::OK,
                JsonResponse(OhclResponse {
                    stores: Some(ListStore { data, next_after }),
                    ..Default::default()
                }),
            ))
        }
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some(format!("Failed listing stores: {error}")),
                ..Default::default()
            }),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/stores",
    responses((status = 201, body = OhclResponse)),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_stores(
    State(app_state): State<AppState>,
    Query(stores): Query<Vec<Store>>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    match app_state
        .investing_entity
        .create_stores(tenant_id.into(), stores)
        .await
    {
        Ok(data) => Ok((
            StatusCode::CREATED,
            JsonResponse(OhclResponse {
                stores: Some(ListStore {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            }),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some(format!("Failed creating stores: {error}")),
                ..Default::default()
            }),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/stores/{store}/products",
    params(("store" = String, Path), QueryPagingInput),
    responses((status = 200, body = OhclResponse))
)]
async fn list_paginated_products(
    State(app_state): State<AppState>,
    Path(store): Path<String>,
    Query(QueryPagingInput {
        after,
        limit,
        detail,
    }): Query<QueryPagingInput>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);
    let detail = detail.unwrap_or(false);

    if limit > 100 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("`limit` mustn't be larger than 100".to_string()),
                ..Default::default()
            }),
        ));
    }

    // @TODO: setup product to specific location stores, some stores share same price with-in
    // district, or only price in specific stores or share same price among every stores with-in
    // same system
    match app_state
        .investing_entity
        .get_store_detail(tenant_id.into(), &store, after, limit, detail)
        .await
    {
        Ok(data) => Ok((
            StatusCode::OK,
            JsonResponse(OhclResponse {
                store: Some(data),
                ..Default::default()
            }),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some(format!("List products failed: {error}")),
                ..Default::default()
            }),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/stores/{store}/products",
    params(("store" = String, Path)),
    responses((status = 201, body = OhclResponse)),
    security(
        ("bearer_auth" = [])
    )
)]
async fn create_products(
    State(app_state): State<AppState>,
    Path(store): Path<String>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
    JsonRequest(products): JsonRequest<Vec<Product>>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    // @TODO: get broker_id by tenant_id
    let tenant_id = tenant_id.into();
    let broker = app_state.secret.get("BROKER", "/").await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some("BROKER not set".into()),
                ..Default::default()
            }),
        )
    })?;

    // @TODO: setup product to specific location stores, some stores share same price with-in
    // district, or only price in specific stores or share same price among every stores with-in
    // same system
    app_state
        .investing_entity
        .validate_broker_listing_limit(tenant_id, &broker, &user_id.0)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get list of products: {}", error)),
                    ..Default::default()
                }),
            )
        })?;

    let broker_id = app_state
        .investing_entity
        .get_broker_id(tenant_id, &broker)
        .await
        .map_err(|error| {
            (
                StatusCode::NOT_FOUND,
                JsonResponse(OhclResponse {
                    error: Some(format!("Failed to get list of products: {}", error)),
                    ..Default::default()
                }),
            )
        })?;

    match app_state
        .investing_entity
        .create_products(tenant_id, broker_id, &store, products)
        .await
    {
        Ok(products) => Ok((
            StatusCode::CREATED,
            JsonResponse(OhclResponse {
                store: Some(Store {
                    products: Some(products),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(OhclResponse {
                error: Some(format!("Create products failed: {error}")),
                ..Default::default()
            }),
        )),
    }
}

#[derive(Debug)]
struct RenderGraphQL {
    symbol: Symbol,
    degree: f32,
    product: Option<i32>,
    history: Vec<Price>,
    current: Option<Price>,
    yesterday: Option<Price>,
}

#[Object]
impl RenderGraphQL {
    #[instrument]
    async fn symbol(&self) -> Option<i32> {
        self.symbol.id
    }

    #[instrument]
    async fn product(&self) -> Option<i32> {
        self.product
    }

    #[graphql(name = "type")]
    #[instrument]
    async fn r_type(&self) -> Option<String> {
        self.symbol.name.clone()
    }

    #[instrument]
    async fn description(&self) -> Option<String> {
        self.symbol.description.clone()
    }

    #[instrument]
    async fn live_price(&self) -> Option<String> {
        self.product
            .map(|product_id| format!("/api/investing/v2/prices/{product_id}"))
    }

    #[instrument]
    async fn buy(&self) -> Option<String> {
        self.current
            .as_ref()
            .map(|p| format!("{:.2}", p.buy / self.degree))
    }

    #[instrument]
    async fn sell(&self) -> Option<String> {
        self.current
            .as_ref()
            .map(|p| format!("{:.2}", p.sell / self.degree))
    }

    #[instrument]
    async fn diff_buy(&self) -> Option<String> {
        let current = self.current.as_ref()?;
        let yesterday = self.yesterday.as_ref()?;

        let diff = current.buy - yesterday.buy;
        let icon = if diff > 0.0 {
            "▲"
        } else if diff < 0.0 {
            "▼"
        } else {
            "●"
        };
        Some(format!("{} {:.2}", icon, diff.abs() / self.degree))
    }

    #[instrument]
    async fn diff_sell(&self) -> Option<String> {
        let current = self.current.as_ref()?;
        let yesterday = self.yesterday.as_ref()?;

        let diff = current.sell - yesterday.sell;
        let icon = if diff > 0.0 {
            "▲"
        } else if diff < 0.0 {
            "▼"
        } else {
            "●"
        };
        Some(format!("{} {:.2}", icon, diff.abs() / self.degree))
    }

    #[instrument]
    async fn yesterday_buy(&self) -> Option<String> {
        self.yesterday
            .as_ref()
            .map(|p| format!("{:.2}", p.buy / self.degree))
    }

    #[instrument]
    async fn yesterday_sell(&self) -> Option<String> {
        self.yesterday
            .as_ref()
            .map(|p| format!("{:.2}", p.sell / self.degree))
    }

    #[instrument]
    async fn trend(&self) -> String {
        let first_price = self.history.first().map(|c| c.buy / self.degree);
        let last_price = self.history.last().map(|c| c.buy / self.degree);

        match (first_price, last_price) {
            (Some(start), Some(end)) if start != 0.0 => {
                let change = ((end - start) / start) * 100.0;

                format!("{:+1.1}%", change)
            }
            _ => "0%".to_string(),
        }
    }

    #[instrument]
    async fn trend_data(&self) -> Vec<f32> {
        self.history
            .iter()
            .map(|it| it.buy / self.degree)
            .collect::<Vec<_>>()
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn gold_market_list(
        &self,
        ctx: &Context<'_>,
        #[graphql(default)] scopes: Vec<i32>,
        #[graphql(default = 0)] after: i32,
        #[graphql(default = 10)] limit: u64,
        #[graphql(default = 7)] lookback: i64,
        #[graphql(default = 1.0)] degree: f64,
    ) -> async_graphql::Result<Vec<RenderGraphQL>> {
        let app_state = ctx.data::<AppState>()?;
        let tenant_id = ctx.data::<i64>()?;
        let lookahead = ctx.look_ahead();

        let broker = app_state
            .investing_entity
            .convert_to_real_broker(
                *tenant_id,
                app_state
                    .secret
                    .get("BROKER", "/")
                    .await
                    .map_err(|e| async_graphql::Error::new(format!("BROKER not set: {e}")))?,
            )
            .await
            .map_err(|e| async_graphql::Error::new(format!("Broker validation failed: {e}")))?;

        let needs_history =
            lookahead.field("trendData").exists() || lookahead.field("trend").exists();
        let needs_diff = lookahead.field("diffBuy").exists()
            || lookahead.field("diffSell").exists()
            || lookahead.field("yesterdayBuy").exists()
            || lookahead.field("yesterdaySell").exists();
        let needs_current = lookahead.field("buy").exists() || lookahead.field("sell").exists();

        let all_symbols = app_state
            .investing_entity
            .list_paginated_symbols(
                *tenant_id,
                &broker,
                after,
                limit,
                true,
                Some(scopes.clone()),
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut current_map = HashMap::new();
        let mut yesterday_map = HashMap::new();
        let mut history_map = HashMap::new();
        let mut product_id_map = HashMap::new();

        if needs_history || needs_diff {
            let from = Utc::now().timestamp() - (lookback * 24 * 60 * 60);
            let to = Utc::now().timestamp();

            let history_res = app_state
                .investing_entity
                .list_history_price_of_symbols(
                    *tenant_id,
                    &broker,
                    Filter {
                        from,
                        to,
                        after,
                        limit,
                        scopes: scopes.clone(),
                        interval: 24 * 60 * 60,
                    },
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

            for (s_id, products_map) in history_res {
                if let Some((p_id, prices)) = products_map.into_iter().next() {
                    product_id_map.insert(s_id, p_id);
                    if let Some(latest) = prices.last() {
                        current_map.insert(s_id, latest.clone());
                        if prices.len() >= 2 {
                            yesterday_map.insert(s_id, prices[prices.len() - 2].clone());
                        }
                    }
                    if needs_history {
                        history_map.insert(s_id, prices);
                    }
                }
            }
        } else if needs_current {
            let current_res = app_state
                .investing_entity
                .list_current_price_of_symbols(
                    *tenant_id,
                    &broker,
                    Filter {
                        after,
                        limit,
                        scopes: scopes.clone(),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

            for (s_id, products_map) in current_res {
                if let Some((p_id, price)) = products_map.into_iter().next() {
                    product_id_map.insert(s_id, p_id);
                    current_map.insert(s_id, price);
                }
            }
        }

        let mut results = Vec::new();

        for symbol in all_symbols {
            let id = symbol.id.unwrap_or(0);
            let current = match current_map.remove(&id) {
                Some(curr) => Some(curr),
                None => {
                    if needs_current {
                        get_latest_price(
                            app_state,
                            *tenant_id,
                            &broker,
                            &symbol.symbol.clone().unwrap_or_default(),
                            0,
                        )
                        .await
                    } else {
                        None
                    }
                }
            };

            results.push(RenderGraphQL {
                symbol,
                current,
                degree: degree as f32,
                product: product_id_map.remove(&id),
                yesterday: yesterday_map.remove(&id),
                history: history_map.remove(&id).unwrap_or_default(),
            });
        }

        Ok(results)
    }
}

#[utoipa::path(
    post,
    path = "/astra-render",
    request_body = GraphQLRequestDTO,
    responses(
        (status = 200, description = "Query successed", body = GraphQLResponseDTO),
        (status = 400, description = "Query failed", body = GraphQLResponseDTO),
    ),
    security(("bearer_auth" = []))
)]
async fn render_data_using_graphql(
    State(app_state): State<AppState>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
    Extension(schema): Extension<Schema<QueryRoot, EmptyMutation, EmptySubscription>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut req = req.into_inner();
    req = req.data(app_state);
    req = req.data(Into::<i64>::into(tenant_id));

    schema.execute(req).await.into()
}
