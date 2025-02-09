use actix_web::{web, HttpResponse};
use actix::Addr;
use std::sync::Arc;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::schemas::Order;
use crate::actors::dnse::{DnseActor, GetOHCLCommand};

#[derive(Serialize)]
struct ListResponse {
}

pub async fn list() -> actix_web::Result<HttpResponse> { 
    Ok(HttpResponse::Ok().json(ListResponse {}))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    orders: Vec<Order>,
}

#[derive(Serialize)]
struct CreateResponse {
    orders: Vec<Order>,
}

#[derive(Debug)]
enum OrderState {
    Open = 0,
    Keep = 1,
    Close = 2,
}

impl TryFrom<i32> for OrderState {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OrderState::Open),
            1 => Ok(OrderState::Keep),
            2 => Ok(OrderState::Close),
            _ => Err("Invalid status value")
        }
    }
}

pub async fn inquiry(
    dnse: web::Data<Arc<Addr<DnseActor>>>,
    request: web::Json<CreateRequest>,
) -> actix_web::Result<HttpResponse> {
    let mut orders = Vec::new();

    for order in &request.orders {    
        let candles_1d = dnse.send(GetOHCLCommand{
            resolution: String::from("1D"),
            stock: order.stock.clone(),
            from: Utc::now().timestamp() - 14*24*60*60,
            to: Utc::now().timestamp(),
        })
        .await
        .unwrap()
        .unwrap_or(Vec::new());

        let latest_price = candles_1d.last()
            .unwrap()
            .c;
        let latest_timestamp = candles_1d.last()
            .unwrap()
            .t;

        // @NOTE: state machine for ordering stock 
        match OrderState::try_from(order.state) {
            Ok(OrderState::Open) => {
                if latest_price < order.price_order {
                    orders.push(Order {
                        id: order.id.clone(),
                        stock: order.stock.clone(),
                        state: OrderState::Keep as i32,
                        order_open_date: Some(latest_timestamp),
                        order_close_date: None,
                        earn: Some(0.0),
                        price_order: latest_price,
                        take_profit: order.take_profit,
                        stop_loss: order.stop_loss,
                        number_of_stocks: order.number_of_stocks,
                    });
                } else {
                    orders.push(Order {
                        id: order.id.clone(),
                        stock: order.stock.clone(),
                        state: OrderState::Open as i32,
                        order_open_date: None,
                        order_close_date: None,
                        earn: Some(0.0),
                        price_order: order.price_order,
                        take_profit: order.take_profit,
                        stop_loss: order.stop_loss,
                        number_of_stocks: order.number_of_stocks,
                    });
                }
            },
            Ok(OrderState::Keep) => {
                if latest_price >= order.take_profit {
                    orders.push(Order {
                        id: order.id.clone(),
                        stock: order.stock.clone(),
                        state: OrderState::Close as i32,
                        order_open_date: order.order_open_date,
                        order_close_date: Some(latest_timestamp),
                        earn: Some(
                            (latest_price - order.price_order) * (order.number_of_stocks as f64)
                        ),
                        price_order: order.price_order,
                        take_profit: order.take_profit,
                        stop_loss: order.stop_loss,
                        number_of_stocks: order.number_of_stocks,
                    });
                } else if latest_price <= order.stop_loss {
                    orders.push(Order {
                        id: order.id.clone(),
                        stock: order.stock.clone(),
                        state: OrderState::Close as i32,
                        order_open_date: order.order_open_date,
                        order_close_date: Some(latest_timestamp),
                        earn: Some(
                            (latest_price - order.price_order) * (order.number_of_stocks as f64)
                        ),
                        price_order: order.price_order,
                        take_profit: order.take_profit,
                        stop_loss: order.stop_loss,
                        number_of_stocks: order.number_of_stocks,
                    });
                } else {
                    orders.push(Order {
                        id: order.id.clone(),
                        stock: order.stock.clone(),
                        state: OrderState::Keep as i32,
                        order_open_date: order.order_open_date,
                        order_close_date: None,
                        earn: Some(
                            (latest_price - order.price_order) * (order.number_of_stocks as f64)
                        ),
                        price_order: order.price_order,
                        take_profit: order.take_profit,
                        stop_loss: order.stop_loss,
                        number_of_stocks: order.number_of_stocks,
                    });
                }
            },
            Ok(OrderState::Close) => {
                orders.push(Order {
                    id: order.id.clone(),
                    stock: order.stock.clone(),
                    state: order.state,
                    order_open_date: order.order_open_date,
                    order_close_date: order.order_close_date,
                    earn: order.earn,
                    price_order: order.price_order,
                    take_profit: order.take_profit,
                    stop_loss: order.stop_loss,
                    number_of_stocks: order.number_of_stocks,
                });
            },
            Err(_) => {
                return Err(actix_web::error::ErrorBadRequest("Invalid order state"));
            }
        }
    }

    Ok(HttpResponse::Ok().json(CreateResponse {
        orders,
    })) 
}

#[derive(Serialize)]
struct DetailResponse {
}

pub async fn detail() -> actix_web::Result<HttpResponse> {    
    Ok(HttpResponse::Ok().json(DetailResponse {}))
}

#[derive(Serialize)]
struct CloseResponse {
}

pub async fn close() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(CloseResponse {}))
    
}
