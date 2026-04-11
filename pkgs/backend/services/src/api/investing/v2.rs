use serde::{Deserialize, Serialize};

use axum::Router;
use axum::extract::Json as JsonRequest;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResponse};
use axum::routing::get;

use models::entities::investing::{Product, Store};
use utoipa::{IntoParams, OpenApi, ToSchema};

use super::{AppState, InvestingHeaders};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_symbol_id_by_product_in_store,
        list_paginated_stores,
        create_stores,
        list_paginated_products,
        create_products,
    ),
    components(schemas(OhclResponse))
)]
pub struct InvestingV2Api;

#[derive(Serialize, Deserialize, Default, Clone, Debug, IntoParams, ToSchema)]
pub struct QueryPagingInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
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
            "/stores/{store}/{product}/symbol",
            get(get_symbol_id_by_product_in_store),
        )
        .route("/stores", get(list_paginated_stores).post(create_stores))
        .route(
            "/stores/{store}",
            get(list_paginated_products).post(create_products),
        )
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
    Query(QueryPagingInput { after, limit }): Query<QueryPagingInput>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    match app_state
        .investing_entity
        .list_paginated_stores(tenant_id.into(), after, limit)
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
    responses((status = 201, body = OhclResponse))
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
    Query(QueryPagingInput { after, limit }): Query<QueryPagingInput>,
    InvestingHeaders { tenant_id, .. }: InvestingHeaders,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let after = after.unwrap_or(0);
    let limit = limit.unwrap_or(10);

    match app_state
        .investing_entity
        .get_store_detail(tenant_id.into(), &store, after, limit, true)
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
    responses((status = 201, body = OhclResponse))
)]
async fn create_products(
    State(app_state): State<AppState>,
    Path(store): Path<String>,
    InvestingHeaders { tenant_id, user_id }: InvestingHeaders,
    JsonRequest(products): JsonRequest<Vec<Product>>,
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    // @TODO: get broker_id by tenant_id
    let tenant_id = tenant_id.into();
    let broker = "".to_string();

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
