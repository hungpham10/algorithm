use std::fmt::{Display, Error as FmtError, Formatter, Result as FmtResult};
use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpResponse, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::entities::wms::{
    Item, ItemStatus, Lot, Node, Order, PathWay, PickingNodeScope, PickingRouteStatus, Plan, Route,
    Sale, Shelf, Stock, Zone,
};

use super::WmsHeaders;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct QueryPagingInput {
    #[serde(default)]
    include_details: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListStocksResponse {
    data: Vec<Stock>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListLotsResponse {
    data: Vec<Lot>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListShelvesResponse {
    data: Vec<Shelf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListItemsResponse {
    data: Vec<Item>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListZonesResponse {
    data: Vec<Zone>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListNodesResponse {
    data: Vec<Node>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListPathsResponse {
    data: Vec<PathWay>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ListPlansResponse {
    data: Vec<Plan>,

    #[serde(skip_serializing_if = "Option::is_none")]
    next_after: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    order: Option<Order>,

    #[serde(skip_serializing_if = "Option::is_none")]
    zones: Option<ListZonesResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    zone: Option<Zone>,

    #[serde(skip_serializing_if = "Option::is_none")]
    nodes: Option<ListNodesResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<Node>,

    #[serde(skip_serializing_if = "Option::is_none")]
    paths: Option<ListPathsResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathWay>,

    #[serde(skip_serializing_if = "Option::is_none")]
    plans: Option<ListPlansResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<Plan>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Display for WmsResponse {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let json = serde_json::to_string(self).map_err(|_| FmtError)?;
        f.write_str(&json)
    }
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
            order: None,
            zones: None,
            zone: None,
            nodes: None,
            node: None,
            paths: None,
            path: None,
            plans: None,
            plan: None,
            error: None,
        }
    }
}
//-------
#[derive(Serialize, Deserialize, Clone)]
pub struct NearExpiryResponse {
    stock_id: i64,
    stock_name: String,
    lot_id: i64,
    lot_number: String,
    quantity: i32,
    expiry_date: DateTime<Utc>,
    shelves: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HighTurnoverResponse {
    id: i64,
    name: String,
    quantity: i32,
    unit: String,
    barcode: Option<String>,
    shelves: Vec<String>,
    avg_weekly_sales: f32,
    turnover_risk: f32,
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
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
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
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                stocks: Some(ListStocksResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to create stock: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_stock(
    appstate: Data<Arc<AppState>>,
    path: Path<(i64,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (stock_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_stock(headers.tenant_id, stock_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                stock: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to get stocks: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_lots(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    path: Path<(i64,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (stock_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
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
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Failed to get list of lots: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                lots: Some(ListLotsResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to create stock: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_lot(
    appstate: Data<Arc<AppState>>,
    path: Path<(i64,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (lot_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_lot(headers.tenant_id, lot_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                lot: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to get lot: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Err(ErrorInternalServerError(WmsResponse {
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
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Failed to get list of shelves: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_stocks_in_shelf(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    path: Path<(i64,)>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (shelf_id,) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Failed to list stocks: limit must not larger than 100"
                )),
                ..Default::default()
            }))
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
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!(
                        "Failed to get list of stocks in shelf {}: {}",
                        shelf_id, error
                    )),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                shelves: Some(ListShelvesResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to create shelves: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn plan_item_for_new_lot(
    appstate: Data<Arc<AppState>>,
    plan: Json<Vec<Stock>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let lot_id = path.into_inner();

    if plan.len() > 100 {
        return Err(ErrorInternalServerError(WmsResponse {
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
                                stock_id: it.id,
                                lot_id: Some(lot_id),
                                cost_price: it.cost_price,
                                status: Some(ItemStatus::Plan),
                                ..Default::default()
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
            Ok(items) => Ok(HttpResponse::Ok().json(WmsResponse {
                items: Some(ListItemsResponse {
                    data: items,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to plan new items: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn import_item_to_warehouse(
    appstate: Data<Arc<AppState>>,
    items: Json<Vec<Item>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let lot_id = path.into_inner();

    if items.len() > 100 {
        return Err(ErrorInternalServerError(WmsResponse {
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
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Failed to import items: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn assign_item_to_shelf(
    appstate: Data<Arc<AppState>>,
    items: Json<Vec<Item>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let shelf_id = path.into_inner();

    if items.len() > 100 {
        return Err(ErrorInternalServerError(WmsResponse {
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
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Failed to assign items to shelf {}: {}",
                    shelf_id, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_item_by_barcode_about_order(
    path: Path<String>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let barcode = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .get_item_by_barcode_in_picking(headers.tenant_id, &barcode)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                item: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Failed to get item by barcode {}: {}",
                    barcode, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_item_by_barcode_in_inventory(
    path: Path<String>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let barcode = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .get_item_by_barcode_in_inventory(headers.tenant_id, &barcode)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                item: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Failed to get item by barcode {}: {}",
                    barcode, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateItemStatusRequest {}

pub async fn update_healthy_status_of_item(
    path: Path<(i64, String)>,
    report: Json<UpdateItemStatusRequest>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (shelf_id, barcode) = path.into_inner();

    Err(ErrorInternalServerError(WmsResponse {
        error: Some(format!("Not implemented")),
        ..Default::default()
    }))
}

pub async fn get_order_detail(
    path: Path<i64>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let order_id = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_order_detail(headers.tenant_id, order_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                order: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to query order {}: {}", order_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_zones(
    appstate: Data<Arc<AppState>>,
    zones: Json<Vec<Zone>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let zones = zones.into_inner();

        match entity.create_zones(headers.tenant_id, &zones).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                zones: Some(ListZonesResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to create new zone: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_zones(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
        } else {
            match entity
                .list_paginated_zones(headers.tenant_id, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        zones: Some(ListZonesResponse { data, next_after }),
                        ..Default::default()
                    }))
                }
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Fail to list zones: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_zone(
    path: Path<i64>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let zone_id = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity.get_zone(headers.tenant_id, zone_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                zone: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to get zone {}: {}", zone_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_nodes(
    path: Path<i64>,
    appstate: Data<Arc<AppState>>,
    nodes: Json<Vec<Node>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let nodes = nodes.into_inner();
        let zone_id = path.into_inner();

        match entity
            .create_nodes(headers.tenant_id, Some(zone_id), &nodes)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                nodes: Some(ListNodesResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to create new zone: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_nodes(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let zone_id = path.into_inner();
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
        } else {
            match entity
                .list_paginated_nodes(headers.tenant_id, zone_id, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().node_id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        nodes: Some(ListNodesResponse { data, next_after }),
                        ..Default::default()
                    }))
                }
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Fail to list zones: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_node_by_id(
    path: Path<(i64, i64)>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (zone_id, node_id) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .get_node_by_id(headers.tenant_id, zone_id, node_id)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                node: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to get zone {}: {}", zone_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_paths_by_node(
    appstate: Data<Arc<AppState>>,
    path: Path<(i64, i64)>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (zone_id, node_id) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
        } else {
            match entity
                .list_paginated_paths_by_node(headers.tenant_id, zone_id, node_id, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().path_id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        paths: Some(ListPathsResponse { data, next_after }),
                        ..Default::default()
                    }))
                }
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!(
                        "Fail to list path zone {}, node {}: {}",
                        zone_id, node_id, error
                    )),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn get_path_by_id(
    path: Path<(i64, i64, i64)>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (zone_id, node_id, path_id) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .get_path_by_id(headers.tenant_id, zone_id, node_id, path_id)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                path: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Fail to list path zone {}, node {}: {}",
                    zone_id, node_id, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn create_paths_from_node(
    path: Path<(i64, i64)>,
    appstate: Data<Arc<AppState>>,
    configs: Json<Vec<PathWay>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (zone_id, node_id) = path.into_inner();
    if let Some(entity) = appstate.wms_entity() {
        let configs = configs
            .into_inner()
            .iter()
            .map(|config| PathWay {
                is_one_way: config.is_one_way,
                path_id: None,
                zone_id: Some(zone_id),
                from_node: Some(node_id),
                to_node: config.to_node,
                name: config.name.clone(),
                status: config.status.clone(),
                sharp: config.sharp.clone(),
            })
            .collect::<Vec<_>>();

        match entity.create_paths(headers.tenant_id, &configs).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                paths: Some(ListPathsResponse {
                    data,
                    next_after: None,
                }),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Fail to list path zone {}, node {}: {}",
                    zone_id, node_id, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn put_shelf_to_node(
    path: Path<(i64, i64, i64)>,
    appstate: Data<Arc<AppState>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let (shelf_id, zone_id, node_id) = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .put_shelf_to_node(headers.tenant_id, zone_id, node_id, shelf_id, true)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                shelf: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Fail to list path zone {}, node {}: {}",
                    zone_id, node_id, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct PickingScope {
    zones: Vec<i64>,
    nodes: Vec<PickingNodeScope>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SetupPickingWaveRequest {
    orders: Vec<Order>,
    scope: Option<PickingScope>,
}

pub async fn get_detail_picking_wave(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let wave_id = path.into_inner();

        match entity.get_picking_wave(headers.tenant_id, wave_id).await {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                plan: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Failed to get detail of plan {}: {}",
                    wave_id, error
                )),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn list_picking_wave(
    appstate: Data<Arc<AppState>>,
    query: Query<QueryPagingInput>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let include_details = query.include_details.unwrap_or(false);
        let after = query.after.unwrap_or(0);
        let limit = query.limit.unwrap_or(10);

        if limit > 100 {
            Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!(
                    "Maximum item per page does not exceed 100, currently is {}",
                    limit
                )),
                ..Default::default()
            }))
        } else {
            match entity
                .list_picking_wave(headers.tenant_id, include_details, after, limit)
                .await
            {
                Ok(data) => {
                    let next_after = if data.len() == limit as usize {
                        data.last().unwrap().id
                    } else {
                        None
                    };

                    Ok(HttpResponse::Ok().json(WmsResponse {
                        plans: Some(ListPlansResponse { data, next_after }),
                        ..Default::default()
                    }))
                }
                Err(error) => Err(ErrorInternalServerError(WmsResponse {
                    error: Some(format!("Failed to get list of stocks: {}", error)),
                    ..Default::default()
                })),
            }
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn setup_picking_wave(
    appstate: Data<Arc<AppState>>,
    config: Json<SetupPickingWaveRequest>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    if let Some(entity) = appstate.wms_entity() {
        let zones = if let Some(scope) = config.scope.clone() {
            scope.zones
        } else {
            vec![]
        };
        let nodes = if let Some(scope) = config.scope.clone() {
            scope.nodes
        } else {
            vec![]
        };

        match entity
            .create_picking_plan(headers.tenant_id, &config.orders, &zones, &nodes)
            .await
        {
            Ok(data) => Ok(HttpResponse::Ok().json(WmsResponse {
                plan: Some(data),
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail setup wave orders: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn update_one_existed_route(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    config: Json<Route>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    Err(ErrorInternalServerError(WmsResponse {
        error: Some(format!("Not implemented")),
        ..Default::default()
    }))
}

pub async fn update_existed_routes_in_batch(
    appstate: Data<Arc<AppState>>,
    config: Json<Vec<Route>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    Err(ErrorInternalServerError(WmsResponse {
        error: Some(format!("Not implemented")),
        ..Default::default()
    }))
}

pub async fn create_new_routes(
    appstate: Data<Arc<AppState>>,
    config: Json<Vec<Route>>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    Err(ErrorInternalServerError(WmsResponse {
        error: Some(format!("Not implemented")),
        ..Default::default()
    }))
}

pub async fn start_one_route(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let route_id = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .notify_when_route_status_changed(
                headers.tenant_id,
                0,
                route_id,
                PickingRouteStatus::Running,
            )
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(WmsResponse {
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to finish route {}: {}", route_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn report_one_route_failed(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let route_id = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .notify_when_route_status_changed(
                headers.tenant_id,
                0,
                route_id,
                PickingRouteStatus::Failed,
            )
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(WmsResponse {
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to finish route {}: {}", route_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
            error: Some(format!("Not implemented")),
            ..Default::default()
        }))
    }
}

pub async fn finish_one_route(
    appstate: Data<Arc<AppState>>,
    path: Path<i64>,
    headers: WmsHeaders,
) -> Result<HttpResponse> {
    let route_id = path.into_inner();

    if let Some(entity) = appstate.wms_entity() {
        match entity
            .notify_when_route_status_changed(
                headers.tenant_id,
                0,
                route_id,
                PickingRouteStatus::Done,
            )
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(WmsResponse {
                ..Default::default()
            })),
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to finish route {}: {}", route_id, error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to sale: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
            Err(error) => Err(ErrorInternalServerError(WmsResponse {
                error: Some(format!("Fail to sale: {}", error)),
                ..Default::default()
            })),
        }
    } else {
        Err(ErrorInternalServerError(WmsResponse {
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
