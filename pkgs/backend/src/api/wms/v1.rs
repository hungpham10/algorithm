use std::sync::Arc;

use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpResponse, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::entities::wms::{Item, Lot, Sale, Shelf, Stock};

use super::WmsHeaders;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct QueryPagingInput {
    #[serde(default)]
    include_details: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
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
pub struct ListShelvesResponse {
    data: Vec<Shelf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ListItemsResponse {
    data: Vec<Item>,

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
    shelves: Option<ListShelvesResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    shelf: Option<Shelf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<ListItemsResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    item: Option<Item>,

    #[serde(skip_serializing_if = "Option::is_none")]
    sale: Option<Sale>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Default for WmsResponse {
    fn default() -> Self {
        Self {
            stocks: None,
            stock: None,
            lots: None,
            lot: None,
            shelves: None,
            shelf: None,
            items: None,
            item: None,
            sale: None,
            error: None,
        }
    }
}
//-------
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
    appstate: Data<Arc<AppState>>,
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
                        ..Default::default()
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_stocks(
    appstate: Data<Arc<AppState>>,
    stocks: Json<Vec<Stock>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let stocks = stocks.into_inner();

        match entity.create_stocks(headers.tenant_id, &stocks).await {
            Ok(ids) => Ok(HttpResponse::Ok().json(WmsResponse {
                stocks: Some(ListStocksResponse {
                    data: ids
                        .iter()
                        .enumerate()
                        .map(|(i, &id)| Stock {
                            id: Some(id),
                            shelves: None,
                            lots: None,
                            quantity: None,
                            cost_price: stocks[i].cost_price,
                            name: stocks[i].name.clone(),
                            unit: stocks[i].unit.clone(),
                        })
                        .collect::<Vec<_>>(),
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create stock: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_stock(
    appstate: Data<Arc<AppState>>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (stock_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_stock(headers.tenant_id, stock_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                stock: Some(data),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to get list of stocks: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_lots(
    appstate: Data<Arc<AppState>>,
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
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
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
                        lots: Some(ListLotsResponse { data, next_after }),
                        ..Default::default()
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_lots(
    appstate: Data<Arc<AppState>>,
    lots: Json<Vec<Lot>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let lots = lots.into_inner();

        match entity.create_lots(headers.tenant_id, &lots).await {
            Ok(ids) => Ok(HttpResponse::Ok().json(WmsResponse {
                lots: Some(ListLotsResponse {
                    data: ids
                        .iter()
                        .enumerate()
                        .map(|(i, &id)| Lot {
                            id: Some(id),
                            entry_date: lots[i].entry_date.clone(),
                            cost_price: lots[i].cost_price.clone(),
                            status: lots[i].status.clone(),
                            supplier: lots[i].supplier.clone(),
                            lot_number: lots[i].lot_number.clone(),
                            quantity: lots[i].quantity,
                        })
                        .collect::<Vec<_>>(),
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create stock: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_lot(
    appstate: Data<Arc<AppState>>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (lot_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_lot(headers.tenant_id, lot_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                lot: Some(data),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to get list of stocks: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_shelves(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
        } else {
            match entity
                .list_paginated_shelves(headers.tenant_id, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        shelves: Some(ListShelvesResponse {
                            data: data,
                            next_after,
                        }),
                        ..Default::default()
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    error: Some(format!("Failed to get list of shelves: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_stocks_in_shelf(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    path: Path<(i32,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (shelf_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Ok(HttpResponse::InternalServerError().body("not implemented"))
        } else {
            match entity
                .list_paginated_stocks_of_shelf(
                    headers.tenant_id,
                    shelf_id,
                    headers.is_guess,
                    after,
                    limit,
                )
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
                        ..Default::default()
                    }))
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_shelves(
    appstate: Data<Arc<AppState>>,
    shelves: Json<Vec<Shelf>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let shelves = shelves.into_inner();

        match entity.create_shelves(headers.tenant_id, &shelves).await {
            Ok(ids) => Ok(HttpResponse::Ok().json(WmsResponse {
                shelves: Some(ListShelvesResponse {
                    data: ids
                        .iter()
                        .enumerate()
                        .map(|(i, &id)| Shelf {
                            id: Some(id),
                            name: shelves[i].name.clone(),
                            description: shelves[i].description.clone(),
                        })
                        .collect::<Vec<_>>(),
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create shelves: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn plan_item_for_new_lot(
    appstate: Data<Arc<AppState>>,
    plan: Json<Vec<Stock>>,
    path: Path<i32>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let lot_id = path.into_inner();

    if plan.len() > 100 {
        return Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!(
                "Maximum stock per plan does not exceed 100, currently is {}",
                plan.len(),
            )),
            ..Default::default()
        }));
    }

    if let Some(entity) = appstate.wms_entity() {
        let items = plan
            .iter()
            .filter_map(|it| {
                if let Some(quantity) = it.quantity {
                    Some(
                        (0..quantity)
                            .map(|_| Item {
                                id: None,
                                shelf: None,
                                expired_at: None,
                                lot_number: None,
                                stock_id: it.id,
                                lot_id: Some(lot_id),
                                cost_price: it.cost_price.unwrap_or(0.0),
                                status: "plan".to_string(),
                                barcode: None,
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        match entity
            .plan_import_new_items(headers.tenant_id, &items)
            .await
        {
            Ok(ids) => Ok(HttpResponse::Ok().json(WmsResponse {
                items: Some(ListItemsResponse {
                    data: ids
                        .iter()
                        .enumerate()
                        .map(|(i, &id)| Item {
                            id: Some(id),
                            shelf: items[i].shelf.clone(),
                            expired_at: items[i].expired_at,
                            lot_number: items[i].lot_number.clone(),
                            stock_id: items[i].stock_id,
                            lot_id: items[i].lot_id,
                            cost_price: items[i].cost_price,
                            status: items[i].status.clone(),
                            barcode: items[i].barcode.clone(),
                        })
                        .collect::<Vec<_>>(),
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create shelves: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn import_item_to_warehouse(
    appstate: Data<Arc<AppState>>,
    items: Json<Vec<Item>>,
    path: Path<i32>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let lot_id = path.into_inner();

    if items.len() > 100 {
        return Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!(
                "Maximum item per batch does not exceed 100, currently is {}",
                items.len(),
            )),
            ..Default::default()
        }));
    }

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .import_real_items(headers.tenant_id, lot_id, &items)
            .await
        {
            Ok(items) => Ok(HttpResponse::Ok().json(WmsResponse {
                items: Some(ListItemsResponse {
                    data: items,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create shelves: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn assign_item_to_shelf(
    appstate: Data<Arc<AppState>>,
    items: Json<Vec<Item>>,
    path: Path<i32>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let shelf_id = path.into_inner();

    if items.len() > 100 {
        return Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!(
                "Maximum item per batch does not exceed 100, currently is {}",
                items.len(),
            )),
            ..Default::default()
        }));
    }

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .assign_items_to_shelf(headers.tenant_id, shelf_id, &items)
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(WmsResponse {
                items: Some(ListItemsResponse {
                    data: items.into_inner(),
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to create shelves: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_item_by_barcode(
    path: Path<String>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let barcode = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .get_item_by_barcode(headers.tenant_id, &barcode)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                item: Some(data),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Failed to get list of stocks: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn process_offline_sale(
    appstate: Data<Arc<AppState>>,
    sale: Json<Sale>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if sale.barcodes.is_none() {
        return Ok(HttpResponse::BadRequest().json(WmsResponse {
            error: Some(format!("Missing field `barcode`")),
            ..Default::default()
        }));
    }

    if let Some(entity) = appstate.wms_entity() {
        match entity.sale_at_storefront(headers.tenant_id, &sale).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                sale: Some(data),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Fail to sale: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn process_online_sale(
    appstate: Data<Arc<AppState>>,
    sale: Json<Sale>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if sale.stock_ids.is_none() {
        return Ok(HttpResponse::BadRequest().json(WmsResponse {
            error: Some(format!("Missing field `stock_ids`")),
            ..Default::default()
        }));
    }

    if let Some(entity) = appstate.wms_entity() {
        match entity.sale_at_website(headers.tenant_id, &sale).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                sale: Some(data),
                ..Default::default()
            })),
            Err(error) => Ok(HttpResponse::InternalServerError().json(WmsResponse {
                error: Some(format!("Fail to sale: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn sync_data(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_near_expiry(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_outdated(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_high_turnover(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}
