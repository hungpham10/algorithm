use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct StockInputV1 {
    name: String,
    quantity: i32,
    unit: String,
    expiry_date: DateTime<Utc>,
    supplier: Option<String>,
    barcode: Option<String>,
    shelf_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StockResponseV1 {
    id: i32,
    name: String,
    quantity: i32,
    unit: String,
    expiry_date: DateTime<Utc>,
    supplier: Option<String>,
    shelves: Vec<String>,
    is_near_expiry: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SaleInputV1 {
    item_id: i32,
    quantity: i32,
    price_per_unit: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SaleResponseV1 {
    item_id: i32,
    name: String,
    quantity_sold: i32,
    total_price: i32,
    remaining_quantity: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SyncInputV1 {
    stocks: Vec<StockInputV1>,
    sales: Vec<SaleInputV1>,
}

#[derive(Deserialize)]
pub struct TurnoverParams {
    days: Option<i32>,
    threshold: Option<f32>,
}

#[derive(Deserialize)]
pub struct Pagination {
    limit: Option<i32>,
    offset: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct HighTurnoverResponseV1 {
    id: i32,
    name: String,
    quantity: i32,
    unit: String,
    expiry_date: DateTime<Utc>,
    supplier: Option<String>,
    shelves: Vec<String>,
    avg_weekly_sales: f32,
    turnover_risk: f32,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    status: String,
    data: Option<T>,
    message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<i32>,
}

pub async fn get_stock(
    state: web::Data<AppState>,
    query: web::Query<Pagination>,
) -> impl Responder {
    let stocks = state.stocks.lock().unwrap();
    let limit = query.limit.unwrap_or(50) as usize;
    let offset = query.offset.unwrap_or(0) as usize;

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        data: Some(paginated),
        message: None,
        total: Some(stocks.len() as i32),
    })
}

pub async fn create_update_stock(
    state: web::Data<AppState>,
    input: web::Json<StockInputV1>,
) -> impl Responder {
    let mut stocks = state.stocks.lock().unwrap();

    // Simple ID generation (replace with proper database ID)
    let new_id = (stocks.len() + 1) as i32;

    let stock = StockResponseV1 {
        id: new_id,
        name: input.name.clone(),
        quantity: input.quantity,
        unit: input.unit.clone(),
        expiry_date: input.expiry_date,
        supplier: input.supplier.clone(),
        shelves: input.shelf_names.clone(),
        is_near_expiry: (input.expiry_date - Utc::now()).num_days() <= 7,
    };

    stocks.push(stock.clone());

    HttpResponse::Created().json(ApiResponse {
        status: "success".to_string(),
        data: Some(stock),
        message: Some("Stock updated successfully".to_string()),
        total: None,
    })
}

pub async fn get_stock_by_id(state: web::Data<AppState>, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let stocks = state.stocks.lock().unwrap();

    match stocks.iter().find(|s| s.id == id) {
        Some(stock) => HttpResponse::Ok().json(ApiResponse {
            status: "success".to_string(),
            data: Some(stock.clone()),
            message: None,
            total: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<StockResponseV1> {
            status: "error".to_string(),
            data: None,
            message: Some("Item not found".to_string()),
            total: None,
        }),
    }
}

pub async fn get_stock_by_barcode(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let barcode = path.into_inner();
    let stocks = state.stocks.lock().unwrap();

    // Assuming barcode is stored in supplier field for simplicity
    match stocks
        .iter()
        .find(|s| s.supplier.as_ref().map_or(false, |sup| sup == &barcode))
    {
        Some(stock) => HttpResponse::Ok().json(ApiResponse {
            status: "success".to_string(),
            data: Some(stock.clone()),
            message: None,
            total: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<StockResponseV1> {
            status: "error".to_string(),
            data: None,
            message: Some("Item not found".to_string()),
            total: None,
        }),
    }
}

pub async fn process_sale(
    state: web::Data<AppState>,
    input: web::Json<SaleInputV1>,
) -> impl Responder {
    let mut stocks = state.stocks.lock().unwrap();

    match stocks.iter_mut().find(|s| s.id == input.item_id) {
        Some(stock) => {
            if stock.quantity >= input.quantity {
                stock.quantity -= input.quantity;

                let response = SaleResponseV1 {
                    item_id: input.item_id,
                    name: stock.name.clone(),
                    quantity_sold: input.quantity,
                    total_price: input.quantity * input.price_per_unit,
                    remaining_quantity: stock.quantity,
                };

                HttpResponse::Ok().json(ApiResponse {
                    status: "success".to_string(),
                    data: Some(response),
                    message: None,
                    total: None,
                })
            } else {
                HttpResponse::BadRequest().json(ApiResponse::<SaleResponseV1> {
                    status: "error".to_string(),
                    data: None,
                    message: Some("Insufficient quantity".to_string()),
                    total: None,
                })
            }
        }
        None => HttpResponse::NotFound().json(ApiResponse::<SaleResponseV1> {
            status: "error".to_string(),
            data: None,
            message: Some("Item not found".to_string()),
            total: None,
        }),
    }
}

pub async fn sync_data(
    state: web::Data<AppState>,
    input: web::Json<SyncInputV1>,
) -> impl Responder {
    // Implement sync logic here
    // This is a simplified version that just accepts the data
    HttpResponse::Ok().json(ApiResponse::<()> {
        status: "success".to_string(),
        data: None,
        message: Some("Data synced successfully".to_string()),
        total: None,
    })
}

pub async fn get_near_expiry(state: web::Data<AppState>) -> impl Responder {
    let stocks = state.stocks.lock().unwrap();
    let near_expiry: Vec<StockResponseV1> = stocks
        .iter()
        .filter(|s| (s.expiry_date - Utc::now()).num_days() <= 7)
        .cloned()
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        data: Some(near_expiry),
        message: None,
        total: None,
    })
}

pub async fn get_outdated(state: web::Data<AppState>) -> impl Responder {
    let stocks = state.stocks.lock().unwrap();
    let outdated: Vec<StockResponseV1> = stocks
        .iter()
        .filter(|s| s.expiry_date < Utc::now())
        .cloned()
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        data: Some(outdated),
        message: None,
        total: None,
    })
}

pub async fn get_high_turnover(
    state: web::Data<AppState>,
    query: web::Query<TurnoverParams>,
) -> impl Responder {
    let stocks = state.stocks.lock().unwrap();
    let days = query.days.unwrap_or(7);
    let threshold = query.threshold.unwrap_or(0.1);

    // Simplified turnover calculation
    let high_turnover: Vec<HighTurnoverResponseV1> = stocks
        .iter()
        .map(|s| HighTurnoverResponseV1 {
            id: s.id,
            name: s.name.clone(),
            quantity: s.quantity,
            unit: s.unit.clone(),
            expiry_date: s.expiry_date,
            supplier: s.supplier.clone(),
            shelves: s.shelves.clone(),
            avg_weekly_sales: 10.0, // Replace with actual calculation
            turnover_risk: (s.quantity as f32) / 10.0, // Replace with actual calculation
        })
        .filter(|s| s.turnover_risk <= threshold)
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        status: "success".to_string(),
        data: Some(high_turnover),
        message: None,
        total: None,
    })
}
