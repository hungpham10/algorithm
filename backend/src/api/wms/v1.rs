use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpResponse, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct QueryPagingInputV1 {
    limit: usize,
    offset: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StockInputV1 {
    name: String,
    unit: String,
    barcode: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StockResponseV1 {
    id: i32,
    name: String,
    quantity: i32,
    unit: String,
    barcode: Option<String>,
    shelves: Vec<String>,
    lots: Vec<LotResponseV1>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LotInputV1 {
    lot_number: String,
    quantity: i32,
    initial_quantity: i32,
    expiry_date: Option<DateTime<Utc>>,
    supplier: Option<String>,
    cost_price: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LotResponseV1 {
    id: i32,
    lot_number: String,
    quantity: i32,
    initial_quantity: i32,
    expiry_date: Option<DateTime<Utc>>,
    supplier: Option<String>,
    status: String,
    shelves: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShelfInputV1 {
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShelfResponseV1 {
    id: i32,
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StockShelfInputV1 {
    stock_id: i32,
    lot_id: i32,
    shelf_id: i32,
    quantity: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaleInputV1 {
    item_id: i32,
    lot_id: Option<i32>,
    quantity: i32,
    price_per_unit: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaleResponseV1 {
    item_id: i32,
    lot_id: Option<i32>,
    name: String,
    quantity_sold: i32,
    total_price: f32,
    remaining_quantity: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncInputV1 {
    stocks: Option<Vec<StockInputV1>>,
    lots: Option<Vec<LotInputV1>>,
    stock_shelves: Option<Vec<StockShelfInputV1>>,
    sales: Option<Vec<SaleInputV1>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NearExpiryResponseV1 {
    stock_id: i32,
    stock_name: String,
    lot_id: i32,
    lot_number: String,
    quantity: i32,
    expiry_date: DateTime<Utc>,
    shelves: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HighTurnoverResponseV1 {
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
pub async fn get_stocks(
    appstate: Data<AppState>,
    query: Query<QueryPagingInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn create_stock(
    appstate: Data<AppState>,
    stock: Json<StockInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_stock(appstate: Data<AppState>, path: Path<i32>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_lots(appstate: Data<AppState>, path: Path<i32>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn create_lot(
    appstate: Data<AppState>,
    path: Path<i32>,
    lot: Json<LotInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_shelves(appstate: Data<AppState>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn create_shelf(
    appstate: Data<AppState>,
    shelf: Json<ShelfInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn assign_stock_shelf(
    appstate: Data<AppState>,
    detail: Json<StockShelfInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn process_sale(
    appstate: Data<AppState>,
    sale: Json<SaleInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_stock_by_barcode(
    path: Path<String>,
    state: Data<AppState>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn sync_data(appstate: Data<AppState>, sync: Json<SyncInputV1>) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_near_expiry(
    appstate: Data<AppState>,
    query: Query<QueryPagingInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_outdated(
    appstate: Data<AppState>,
    query: Query<QueryPagingInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}

pub async fn get_high_turnover(
    appstate: Data<AppState>,
    query: Query<QueryPagingInputV1>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().body("not implemented"))
}
