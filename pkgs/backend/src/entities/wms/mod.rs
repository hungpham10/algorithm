mod items;
mod lots;
mod sale_events;
mod sales;
mod shelves;
mod stock_shelves;
mod stocks;

#[cfg(not(feature = "sharding"))]
mod single;

#[cfg(not(feature = "sharding"))]
pub use single::*;

#[cfg(feature = "sharding")]
mod sharding;

#[cfg(feature = "sharding")]
pub use sharding::*;

use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Result as FmtResult};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Stock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelves: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lots: Option<Vec<Lot>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_price: Option<f64>,

    pub name: String,
    pub unit: String,
}

#[repr(i32)]
enum LotStatus {
    Unavailable,
    Planing,
    Transporting,
    Available,
    Outdated,
    Returned,
}

impl TryFrom<i32> for LotStatus {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LotStatus::Unavailable),
            1 => Ok(LotStatus::Planing),
            2 => Ok(LotStatus::Transporting),
            3 => Ok(LotStatus::Available),
            4 => Ok(LotStatus::Outdated),
            5 => Ok(LotStatus::Returned),
            _ => Err(format!("Invalid state({}) for lot", value)),
        }
    }
}

impl TryFrom<String> for LotStatus {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Unavailable" => Ok(LotStatus::Unavailable),
            "On planing" => Ok(LotStatus::Planing),
            "On transporting" => Ok(LotStatus::Transporting),
            "Available" => Ok(LotStatus::Available),
            "Outdated" => Ok(LotStatus::Outdated),
            "Being returned" => Ok(LotStatus::Returned),
            _ => Err(format!("Invalid state({}) for lot", value)),
        }
    }
}

impl Display for LotStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            LotStatus::Unavailable => write!(f, "Unavailable"),
            LotStatus::Planing => write!(f, "On planing"),
            LotStatus::Transporting => write!(f, "On transporting"),
            LotStatus::Available => write!(f, "Available"),
            LotStatus::Outdated => write!(f, "Outdated"),
            LotStatus::Returned => write!(f, "Being returned"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Lot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_date: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_date: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_price: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier: Option<String>,

    pub lot_number: String,
    pub quantity: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Shelf {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[repr(i32)]
enum ItemStatus {
    Unavailable,
    Available,
    Damaged,
    Outdated,
    Saled,
    Returned,
}

impl TryFrom<i32> for ItemStatus {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ItemStatus::Unavailable),
            1 => Ok(ItemStatus::Available),
            2 => Ok(ItemStatus::Damaged),
            3 => Ok(ItemStatus::Outdated),
            4 => Ok(ItemStatus::Saled),
            5 => Ok(ItemStatus::Returned),
            _ => Err(format!("Invalid state({}) for item", value)),
        }
    }
}

impl Display for ItemStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ItemStatus::Unavailable => write!(f, "Unavailable"),
            ItemStatus::Available => write!(f, "Available"),
            ItemStatus::Damaged => write!(f, "Being damaged"),
            ItemStatus::Saled => write!(f, "Being saled"),
            ItemStatus::Returned => write!(f, "Being returned"),
            ItemStatus::Outdated => write!(f, "Outdated"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Item {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelf: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_number: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,

    pub cost_price: f64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sale {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_ids: Option<Vec<i32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcodes: Option<Vec<String>>,

    pub order_id: i32,
    pub cost_prices: Vec<f64>,
}
