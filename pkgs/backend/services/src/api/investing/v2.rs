use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use axum::Router;
use axum::extract::Json as JsonRequest;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResponse};
use axum::routing::get;

use models::entities::investing::{Price, Product, Store, Symbol};
use utoipa::{
    IntoParams, Modify, OpenApi, ToSchema,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use super::{AppState, InvestingHeaders};

#[derive(OpenApi)]
#[openapi(
    paths(
        list_paginated_products,
        list_price_data,
        list_paginated_stores,
        list_paginated_symbols,
        create_stores,
        create_products,
        ingest_price_data,
        get_price_data,
        get_symbol_id_by_product_in_store,
    ),
    components(schemas(OhclResponse, Price,)),
    modifiers(&SecurityAddon)
)]
pub struct InvestingV2Api;

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

    Router::new()
        .route(
            "/stores/{store}/products/{product}/price",
            get(get_price_data).post(ingest_price_data),
        )
        .route(
            "/stores/{store}/products/{product}/symbol",
            get(get_symbol_id_by_product_in_store),
        )
        .route("/stores/{store}/price", get(list_price_data))
        .route("/stores", get(list_paginated_stores).post(create_stores))
        .route(
            "/stores/{store}/products",
            get(list_paginated_products).post(create_products),
        )
        .route("/symbols", get(list_paginated_symbols))
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
        .list_paginated_symbols(tenant_id, after, limit, detail)
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
async fn list_price_data(
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
                .list_price(tenant_id, store_id)
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

#[utoipa::path(
    get,
    path = "/stores/{store}/products/{product}/price",
    params(
        ("store" = String, Path, description = "Store name"),
        ("product" = String, Path, description = "Product name"),
    ),
    responses((status = 200, body = OhclResponse)),
)]
async fn get_price_data(
    State(app_state): State<AppState>,
    Path((store, product)): Path<(String, String)>,
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

    Ok(JsonResponse(OhclResponse {
        price: Some(
            app_state
                .investing_entity
                .get_price(tenant_id, product_id)
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
        (status = 500, description = "Internal Server Error", body = OhclResponse)
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
