use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpResponse, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::entities::wms::{Lot, Stock};

use super::WmsHeaders;

#[derive(Serialize, Deserialize, Clone)]
pub struct QueryPagingInput {
    include_details: Option<bool>,
    after: Option<i32>,
    limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ListStocksResponse {
    data: Vec<Stock>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ListLotsResponse {
    data: Vec<Lot>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WmsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    stocks: Option<ListStocksResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stock: Option<Stock>,

    #[serde(skip_serializing_if = "Option::is_none")]
    lots: Option<ListLotsResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    lot: Option<Lot>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

//-------
#[derive(Serialize, Deserialize, Clone)]
pub struct ShelfInput {
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShelfResponse {
    id: i32,
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StockShelfInput {
    stock_id: i32,
    lot_id: i32,
    shelf_id: i32,
    quantity: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaleInput {
    item_id: i32,
    lot_id: Option<i32>,
    quantity: i32,
    price_per_unit: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaleResponse {
    item_id: i32,
    lot_id: Option<i32>,
    name: String,
    quantity_sold: i32,
    total_price: f32,
    remaining_quantity: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncInput {
    stocks: Option<Vec<Stock>>,
    lots: Option<Vec<Lot>>,
    sales: Option<Vec<SaleInput>>,
    stock_shelves: Option<Vec<StockShelfInput>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NearExpiryResponse {
    stock_id: i32,
    stock_name: String,
    lot_id: i32,
    lot_number: String,
    quantity: i32,
    expiry_date: DateTime<Utc>,
    shelves: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HighTurnoverResponse {
    id: i32,
    name: String,
    quantity: i32,
    unit: String,
    barcode: Option<String>,
    shelves: Vec<String>,
    avg_weekly_sales: f32,
    turnover_risk: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    status: String,
    message: String,
}

// Route Handlers
pub async fn list_stocks(
    appstate: Data<AppState>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let include_details = query.include_details.unwrap_or(false);
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Ok(HttpResponse::InternalServerError().body("not implemented"))
        } else {
            match entity
                .list_paginated_stocks(headers.tenant_id, include_details, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        stocks: Some(ListStocksResponse { data, next_after }),
                        stock: None,
                        lots: None,
                        lot: None,
                        error: None,
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    stocks: None,
                    stock: None,
                    lots: None,
                    lot: None,
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn create_stock(
    appstate: Data<AppState>,
    stock: Json<Stock>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let stock = Stock {
            id: None,
            quantity: None,
            shelves: None,
            lots: None,
            name: stock.name.clone(),
            unit: stock.unit.clone(),
        };

        match entity.create_stock(headers.tenant_id, &stock).await {
            Ok(id) => Ok(HttpResponse::Ok().json(WmsResponse {
                stocks: None,
                stock: Some(Stock {
                    id: Some(id),
                    quantity: None,
                    shelves: None,
                    lots: None,
                    name: stock.name.clone(),
                    unit: stock.unit.clone(),
                }),
                lots: None,
                lot: None,
                error: None,
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                stocks: None,
                stock: None,
                lots: None,
                lot: None,
                error: Some(format!("Failed to create stock: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn get_stock(
    appstate: Data<AppState>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (stock_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_stock(headers.tenant_id, stock_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                stocks: None,
                stock: Some(data),
                lots: None,
                lot: None,
                error: None,
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                stocks: None,
                stock: None,
                lots: None,
                lot: None,
                error: Some(format!("Failed to get list of stocks: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn list_lots(
    appstate: Data<AppState>,
    query: Query<QueryPagingInput>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (stock_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Ok(HttpResponse::InternalServerError().json(WmsResponse {
                stocks: None,
                stock: None,
                lots: None,
                lot: None,
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
            }))
        } else {
            match entity
                .list_paginated_lots_of_stock(headers.tenant_id, stock_id, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        stocks: None,
                        stock: None,
                        lots: Some(ListLotsResponse { data, next_after }),
                        lot: None,
                        error: None,
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    stocks: None,
                    stock: None,
                    lots: None,
                    lot: None,
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn create_lot(
    appstate: Data<AppState>,
    path: Path<i32>,
    lot: Json<Lot>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_lot(
    appstate: Data<AppState>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (lot_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_lot(headers.tenant_id, lot_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                stocks: None,
                stock: None,
                lots: None,
                lot: Some(data),
                error: None,
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                stocks: None,
                stock: None,
                lots: None,
                lot: None,
                error: Some(format!("Failed to get list of stocks: {}", error)),
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            error: Some(format!("Not implemented")),
        }))
    }
}

pub async fn get_shelves(appstate: Data<AppState>, headers: WmsHeaders) -> Result<HttpResponse> {
    if headers.is_guess {
    } else {
    }

    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_stock_in_shelve(appstate: Data<AppState>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn create_shelf(
    appstate: Data<AppState>,
    shelf: Json<ShelfInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn assign_stock_shelf(
    appstate: Data<AppState>,
    detail: Json<StockShelfInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn process_sale(appstate: Data<AppState>, sale: Json<SaleInput>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_stock_by_barcode(
    path: Path<String>,
    state: Data<AppState>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn sync_data(appstate: Data<AppState>, sync: Json<SyncInput>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_near_expiry(
    appstate: Data<AppState>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_outdated(
    appstate: Data<AppState>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_high_turnover(
    appstate: Data<AppState>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}
