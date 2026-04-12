use serde::{Deserialize, Serialize};

use axum::Router;
use axum::extract::Json as JsonRequest;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResponse};
use axum::routing::get;

use models::entities::investing::{Product, Store};
use utoipa::{
    IntoParams, Modify, OpenApi, ToSchema,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use super::{AppState, InvestingHeaders};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_symbol_id_by_product_in_store,
        list_paginated_stores,
        create_stores,
        list_paginated_products,
        create_products,
        ingest_price_data,
    ),
    components(schemas(OhclResponse, IngestPriceRequest,)),
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

#[derive(Deserialize, Serialize, Clone, Debug, Default, ToSchema)]
struct OhclResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stores: Option<ListStore>,

    store: Option<Store>,
}

pub fn routes() -> Router<AppState> {
    // @TODO: cần api để cập nhật product với store cụ thể không?

    Router::new()
        .route(
            "/stores/symbols/{store}/{product}",
            get(get_symbol_id_by_product_in_store).post(ingest_price_data),
        )
        .route("/stores", get(list_paginated_stores).post(create_stores))
        .route(
            "/stores/{store}",
            get(list_paginated_products).post(create_products),
        )
}

#[derive(Deserialize, Debug, ToSchema, IntoParams)]
pub struct IngestPriceRequest {
    pub buy: f32,
    pub sell: f32,
}

#[utoipa::path(
    post,
    path = "/stores/symbols/{store}/{product}",
    request_body = IngestPriceRequest,
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
    JsonRequest(payload): JsonRequest<IngestPriceRequest>,
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
        .update_price(product_id, payload.buy, payload.sell)
        .await
        .map_err(|error| {
            (
                StatusCode::NOT_FOUND,
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
    path = "/stores/symbols/{store}/{product}",
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
                StatusCode::NOT_FOUND,
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

    if limit > 100 {
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
    path = "/stores/{store}",
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
    path = "/stores/{store}",
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
