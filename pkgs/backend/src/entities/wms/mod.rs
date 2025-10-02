mod items;
mod lots;
mod sale_events;
mod sales;
mod shelves;
mod stock_entries;
mod stock_shelves;
mod stocks;

use items::Entity as Items;
use lots::Entity as Lots;
use sale_events::Entity as SaleEvents;
use sales::Entity as Sales;
use shelves::Entity as Shelves;
use stock_entries::Entity as StockEntries;
use stock_shelves::Entity as StockShelves;
use stocks::Entity as Stocks;

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use sea_orm::entity::prelude::Expr;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, RuntimeErr, Set, TransactionTrait,
};

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

pub struct Wms {
    db: Vec<Arc<DatabaseConnection>>,
}

impl Wms {
    pub fn new(db: Vec<Arc<DatabaseConnection>>) -> Self {
        Self { db }
    }

    fn dbt(&self, tenant_id: i32) -> &DatabaseConnection {
        self.db[(tenant_id as usize) % self.db.len()].as_ref()
    }

    pub async fn create_stocks(&self, tenant_id: i32, stocks: &[Stock]) -> Result<Vec<i32>, DbErr> {
        if stocks.is_empty() {
            return Ok(vec![]);
        }

        stocks::Entity::insert_many(
            stocks
                .iter()
                .map(|stock| stocks::ActiveModel {
                    tenant_id: Set(tenant_id),
                    name: Set(stock.name.clone()),
                    unit: Set(stock.unit.clone()),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        )
        .exec(self.dbt(tenant_id))
        .await?;

        Ok(stocks::Entity::find()
            .select_only()
            .column(stocks::Column::Id)
            .filter(stocks::Column::TenantId.eq(tenant_id))
            .filter(
                stocks::Column::Name.is_in(
                    stocks
                        .iter()
                        .map(|stock| stock.name.clone())
                        .collect::<Vec<_>>(),
                ),
            )
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>())
    }

    pub async fn create_shelves(
        &self,
        tenant_id: i32,
        shelves: &[Shelf],
    ) -> Result<Vec<i32>, DbErr> {
        if shelves.is_empty() {
            return Ok(vec![]);
        }

        // Insert batch
        shelves::Entity::insert_many(
            shelves
                .iter()
                .map(|shelf| shelves::ActiveModel {
                    tenant_id: Set(tenant_id),
                    name: Set(shelf.name.clone().unwrap_or_default()),
                    description: Set(shelf.description.clone()),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        )
        .exec(self.dbt(tenant_id))
        .await?;

        Ok(shelves::Entity::find()
            .select_only()
            .column(shelves::Column::Id)
            .filter(shelves::Column::TenantId.eq(tenant_id))
            .filter(
                shelves::Column::Name.is_in(
                    shelves
                        .iter()
                        .filter_map(|s| s.name.clone())
                        .collect::<Vec<_>>(),
                ),
            )
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>())
    }

    pub async fn create_lots(&self, tenant_id: i32, lots: &[Lot]) -> Result<Vec<i32>, DbErr> {
        let mut models = Vec::new();

        if lots.is_empty() {
            return Ok(vec![]);
        }

        for l in lots {
            models.push(lots::ActiveModel {
                tenant_id: Set(tenant_id),
                lot_number: Set(l.lot_number.clone()),
                quantity: Set(l.quantity),
                supplier: Set(l.supplier.clone()),
                entry_date: Set(l.entry_date.unwrap_or_else(chrono::Utc::now)),
                cost_price: Set(l.cost_price),
                status: Set(Some(
                    LotStatus::try_from(
                        l.status
                            .clone()
                            .unwrap_or(LotStatus::Unavailable.to_string()),
                    )
                    .map_err(|error| {
                        DbErr::Custom(format!(
                            "Fail with status of lot {}: {}",
                            l.lot_number.clone(),
                            error
                        ))
                    })? as i32,
                )),
                ..Default::default()
            })
        }

        lots::Entity::insert_many(models)
            .exec(self.dbt(tenant_id))
            .await?;

        Ok(lots::Entity::find()
            .select_only()
            .column(lots::Column::Id)
            .filter(lots::Column::TenantId.eq(tenant_id))
            .filter(
                lots::Column::LotNumber.is_in(
                    lots.iter()
                        .map(|l| l.lot_number.clone())
                        .collect::<Vec<_>>(),
                ),
            )
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>())
    }

    pub async fn plan_import_new_items(
        &self,
        tenant_id: i32,
        items: &[Item],
    ) -> Result<Vec<i32>, DbErr> {
        let mut created_item_ids = Vec::new();

        let txn = self.dbt(tenant_id).begin().await?;
        let stock_ids = items
            .iter()
            .filter_map(|item| item.stock_id)
            .collect::<HashSet<_>>();
        let lot_ids = items
            .iter()
            .filter_map(|item| item.lot_id)
            .collect::<HashSet<_>>();

        let valid_stocks = stocks::Entity::find()
            .filter(stocks::Column::Id.is_in(stock_ids.clone()))
            .filter(stocks::Column::TenantId.eq(tenant_id))
            .all(&txn)
            .await?
            .into_iter()
            .map(|stock| stock.id)
            .collect::<HashSet<i32>>();

        if valid_stocks.len() != stock_ids.len() {
            let invalid_ids: Vec<i32> = stock_ids.difference(&valid_stocks).copied().collect();
            return Err(DbErr::Custom(format!(
                "Invalid stock IDs: {:?}",
                invalid_ids
            )));
        }

        let valid_lots = lots::Entity::find()
            .filter(lots::Column::Id.is_in(lot_ids.clone()))
            .filter(lots::Column::TenantId.eq(tenant_id))
            .all(&txn)
            .await?
            .into_iter()
            .map(|lot| lot.id)
            .collect::<HashSet<i32>>();

        if valid_lots.len() != lot_ids.len() {
            return Err(DbErr::Custom(format!(
                "Invalid lot IDs: {:?}",
                lot_ids.difference(&valid_lots).copied().collect::<Vec<_>>(),
            )));
        }

        let mut count_stock_entries = HashMap::new();
        for item in items {
            let lot_id = item.lot_id.unwrap_or(0);
            let stock_id = item.stock_id.unwrap_or(0);

            if valid_lots.contains(&lot_id) && valid_stocks.contains(&stock_id) {
                count_stock_entries
                    .entry(lot_id)
                    .or_insert_with(HashMap::new)
                    .entry(stock_id)
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }

        stock_entries::Entity::insert_many(count_stock_entries.iter().flat_map(
            |(&lot_id, stock_map)| {
                stock_map
                    .iter()
                    .map(move |(&stock_id, &quantity)| stock_entries::ActiveModel {
                        tenant_id: Set(tenant_id),
                        lot_id: Set(lot_id),
                        stock_id: Set(stock_id),
                        quantity: Set(quantity as i32),
                        status: Set(0),
                        ..Default::default()
                    })
            },
        ))
        .exec(&txn)
        .await?;

        let inserted_items =
            items::Entity::insert_many(items.iter().map(|item| items::ActiveModel {
                tenant_id: Set(tenant_id),
                stock_id: Set(item.stock_id.unwrap_or(0)),
                lot_id: Set(item.lot_id.unwrap_or(0)),
                expired_at: Set(item.expired_at),
                cost_price: Set(item.cost_price),
                barcode: Set(item.barcode.clone()),
                ..Default::default()
            }))
            .exec(&txn)
            .await?;

        created_item_ids.extend(inserted_items.last_insert_id as i32..);
        txn.commit().await?;

        Ok(created_item_ids)
    }

    pub async fn import_real_items(
        &self,
        tenant_id: i32,
        lot_id: i32,
        items: &[Item],
    ) -> Result<Vec<Item>, DbErr> {
        let mut ret = Vec::new();
        let txn = self.dbt(tenant_id).begin().await?;
        let valid_items = items::Entity::find()
            .filter(items::Column::TenantId.eq(tenant_id))
            .filter(items::Column::LotId.eq(lot_id))
            .filter(
                items::Column::Id.is_in(
                    items
                        .iter()
                        .filter_map(|item| item.id)
                        .collect::<HashSet<_>>(),
                ),
            )
            .all(&txn)
            .await?
            .into_iter()
            .map(|item| item.id)
            .collect::<HashSet<_>>();

        for item in items {
            let item_id = item
                .id
                .ok_or_else(|| DbErr::Custom(format!("Item ID is missing")))?;

            if valid_items.contains(&item_id) {
                let mut update_query =
                    items::Entity::update_many().filter(items::Column::Id.eq(item_id));

                if let Some(barcode) = &item.barcode {
                    update_query =
                        update_query.col_expr(items::Column::Barcode, Expr::value(barcode.clone()));
                }

                if let Some(expired_at) = item.expired_at {
                    update_query =
                        update_query.col_expr(items::Column::ExpiredAt, Expr::value(expired_at));
                }

                update_query
                    .col_expr(items::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
                    .exec(&txn)
                    .await?;

                items::Entity::find_by_id(item_id)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| {
                        DbErr::Custom(format!("Item with id {} not found after update", item_id))
                    })?;

                ret.push(Item {
                    id: item.id,
                    expired_at: item.expired_at.clone(),
                    shelf: item.shelf.clone(),
                    lot_number: item.lot_number.clone(),
                    lot_id: item.lot_id,
                    stock_id: item.stock_id,
                    barcode: item.barcode.clone(),
                    cost_price: item.cost_price,
                    status: item.status.clone(),
                });
            }
        }

        txn.commit().await?;
        Ok(ret)
    }

    pub async fn assign_items_to_shelf(
        &self,
        tenant_id: i32,
        shelf_id: i32,
        items: &[Item],
    ) -> Result<(), DbErr> {
        if items.is_empty() {
            return Ok(());
        }

        let update_result = items::Entity::update_many()
            .col_expr(items::Column::ShelfId, Expr::value(Some(shelf_id)))
            .filter(items::Column::Id.is_in(items.iter().map(|item| item.id).collect::<Vec<_>>()))
            .filter(items::Column::TenantId.eq(tenant_id))
            .exec(self.dbt(tenant_id))
            .await?;

        if update_result.rows_affected != items.len() as u64 {
            Err(DbErr::Custom("Not all items were updated".to_string()))
        } else {
            Ok(())
        }
    }

    pub async fn get_stock(&self, tenant_id: i32, stock_id: i32) -> Result<Stock, DbErr> {
        let result = Stocks::find()
            .filter(stocks::Column::TenantId.eq(tenant_id))
            .filter(stocks::Column::Id.eq(stock_id))
            .one(self.dbt(tenant_id))
            .await?;

        if let Some(result) = result {
            Ok(Stock {
                id: Some(result.id),
                quantity: None,
                name: result.name.clone(),
                unit: result.unit.clone(),
                cost_price: None,
                lots: None,
                shelves: None,
            })
        } else {
            Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Stock with id {}, not exist",
                stock_id
            ))))
        }
    }

    pub async fn list_paginated_stocks(
        &self,
        tenant_id: i32,
        include_details: bool,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Stock>, DbErr> {
        if include_details {
            // @NOTE: khi scale lớn cần tính lại vị trí các item trong hệ thống
            //        tương ứng với nơi mà chúng được đặt trên kệ ở shelves

            let rows = StockShelves::find()
                .select_only()
                .column_as(stocks::Column::Id, "id")
                .column_as(stocks::Column::Name, "name")
                .column_as(stocks::Column::Unit, "unit")
                .column_as(shelves::Column::Name, "shelf_name")
                .column_as(lots::Column::Id, "lot_id")
                .column_as(lots::Column::LotNumber, "lot_number")
                .column_as(lots::Column::CostPrice, "cost_price")
                .column_as(lots::Column::Status, "lot_status")
                .column_as(lots::Column::Supplier, "lot_supplier")
                .column_as(lots::Column::Quantity, "lot_quantity")
                .join_rev(
                    JoinType::InnerJoin,
                    stocks::Entity::belongs_to(StockShelves)
                        .from(stocks::Column::Id)
                        .to(stock_shelves::Column::StockId)
                        .into(),
                )
                .join_rev(
                    JoinType::InnerJoin,
                    stock_entries::Entity::belongs_to(Stocks)
                        .from(stock_entries::Column::StockId)
                        .to(stocks::Column::Id)
                        .into(),
                )
                .join_rev(
                    JoinType::InnerJoin,
                    shelves::Entity::belongs_to(StockShelves)
                        .from(shelves::Column::Id)
                        .to(stock_shelves::Column::ShelfId)
                        .into(),
                )
                .join_rev(
                    JoinType::InnerJoin,
                    lots::Entity::belongs_to(StockEntries)
                        .from(lots::Column::Id)
                        .to(stock_entries::Column::LotId)
                        .into(),
                )
                .filter(stocks::Column::TenantId.eq(tenant_id))
                .filter(stocks::Column::Id.gt(after))
                .limit(limit)
                .order_by_asc(stocks::Column::Id)
                .into_tuple::<(
                    i32,
                    String,
                    String,
                    String,
                    i32,
                    String,
                    f64,
                    i32,
                    Option<String>,
                    i32,
                )>()
                .all(self.dbt(tenant_id))
                .await?;

            let mut stock_map = HashMap::new();

            for (
                id,
                name,
                unit,
                shelf_name,
                lot_id,
                lot_number,
                cost_price,
                lot_status,
                lot_supplier,
                lot_quantity,
            ) in rows
            {
                let entry = stock_map.entry(id).or_insert_with(|| {
                    (name.clone(), unit.clone(), Vec::new(), Vec::new(), 0, 0.0)
                });

                if !entry.2.contains(&shelf_name) {
                    entry.2.push(shelf_name);
                }

                let lot = Lot {
                    id: Some(lot_id),
                    entry_date: None,
                    expired_date: None,
                    cost_price: Some(cost_price),
                    status: Some(
                        LotStatus::try_from(lot_status)
                            .map_err(|error| {
                                DbErr::Query(RuntimeErr::Internal(format!(
                                    "Lot with id {} face issue: {}",
                                    lot_id, error,
                                )))
                            })?
                            .to_string(),
                    ),
                    supplier: lot_supplier.clone(),
                    lot_number,
                    quantity: lot_quantity,
                };
                entry.3.push(lot);

                entry.4 += lot_quantity as i64;
                entry.5 += (lot_quantity as f64) * cost_price;
            }

            let mut list_ret_stocks = Vec::new();
            for (id, (name, unit, shelves, lots, total_qty, total_cost)) in stock_map {
                let avg_cost = if total_qty > 0 {
                    Some(total_cost / total_qty as f64)
                } else {
                    None
                };

                let stock = Stock {
                    id: Some(id),
                    shelves: Some(shelves),
                    lots: Some(lots),
                    quantity: Some(total_qty as i32),
                    cost_price: avg_cost,
                    name,
                    unit,
                };
                list_ret_stocks.push(stock);
            }

            list_ret_stocks.sort_by_key(|s| s.id.unwrap_or(0));
            Ok(list_ret_stocks)
        } else {
            Ok(Stocks::find()
                .select_only()
                .column(stocks::Column::Id)
                .column(stocks::Column::Name)
                .column(stocks::Column::Unit)
                .column_as(
                    Expr::col((stock_entries::Entity, stock_entries::Column::Quantity)).sum(),
                    "quantity",
                )
                .join_rev(
                    JoinType::InnerJoin,
                    stock_entries::Entity::belongs_to(Stocks)
                        .from(stock_entries::Column::StockId)
                        .to(stocks::Column::Id)
                        .into(),
                )
                .filter(stocks::Column::TenantId.eq(tenant_id))
                .filter(stocks::Column::Id.gt(after))
                .limit(limit)
                .order_by_asc(stocks::Column::Id)
                .into_tuple::<(i32, String, String, i32)>()
                .all(self.dbt(tenant_id))
                .await?
                .into_iter()
                .map(|(id, name, unit, quantity)| Stock {
                    id: Some(id),
                    quantity: Some(quantity),
                    name: name.clone(),
                    unit: unit.clone(),
                    cost_price: None,
                    lots: None,
                    shelves: None,
                })
                .collect::<Vec<_>>())
        }
    }

    pub async fn get_lot(&self, tenant_id: i32, lot_id: i32) -> Result<Lot, DbErr> {
        let result = Lots::find()
            .filter(lots::Column::TenantId.eq(tenant_id))
            .filter(lots::Column::Id.eq(lot_id))
            .one(self.dbt(tenant_id))
            .await?;

        if let Some(result) = result {
            let status = LotStatus::try_from(result.status.unwrap_or(0)).map_err(|error| {
                DbErr::Query(RuntimeErr::Internal(format!(
                    "Lot with id {} face issue: {}",
                    lot_id, error,
                )))
            })?;

            Ok(Lot {
                id: Some(result.id),
                entry_date: Some(result.entry_date),
                expired_date: None,
                lot_number: result.lot_number.to_string(),
                quantity: result.quantity,
                cost_price: result.cost_price,
                supplier: result.supplier.clone(),
                status: Some(status.to_string()),
            })
        } else {
            Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Lot with id {}, not exist",
                lot_id
            ))))
        }
    }

    pub async fn list_paginated_lots_of_stock(
        &self,
        tenant_id: i32,
        stock_id: i32,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Lot>, DbErr> {
        Ok(Lots::find()
            .select_only()
            .column(lots::Column::Id)
            .column(lots::Column::LotNumber)
            .column(lots::Column::Supplier)
            .column(lots::Column::EntryDate)
            .column(lots::Column::CostPrice)
            .column(lots::Column::Status)
            .column_as(
                Expr::col((items::Entity, items::Column::Id)).count(),
                "quantity",
            )
            .join_rev(
                JoinType::InnerJoin,
                items::Entity::belongs_to(Lots)
                    .from(items::Column::LotId)
                    .to(lots::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::InnerJoin,
                stocks::Entity::belongs_to(Items)
                    .from(stocks::Column::Id)
                    .to(items::Column::StockId)
                    .into(),
            )
            .filter(
                Condition::all()
                    .add(lots::Column::TenantId.eq(tenant_id))
                    .add(items::Column::StockId.eq(stock_id)),
            )
            .filter(lots::Column::Id.gt(after))
            .limit(limit)
            .group_by(lots::Column::Id)
            .group_by(lots::Column::TenantId)
            .group_by(lots::Column::LotNumber)
            .group_by(lots::Column::Supplier)
            .group_by(lots::Column::EntryDate)
            .group_by(lots::Column::CostPrice)
            .group_by(lots::Column::Status)
            .order_by_asc(lots::Column::Id)
            .into_tuple::<(
                i32,
                String,
                Option<String>,
                DateTime<Utc>,
                f64,
                Option<String>,
                i32,
            )>()
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(
                |(id, lot_number, supplier, entry_date, cost_price, status, quantity)| Lot {
                    id: Some(id),
                    entry_date: Some(entry_date),
                    expired_date: None,
                    cost_price: Some(cost_price),
                    status: status,
                    supplier: supplier,
                    lot_number: lot_number,
                    quantity: quantity,
                },
            )
            .collect::<Vec<_>>())
    }

    pub async fn list_paginated_stocks_of_shelf(
        &self,
        tenant_id: i32,
        shelf_id: i32,
        is_publish: bool,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Stock>, DbErr> {
        if is_publish {
            Ok(Stocks::find()
                .filter(
                    Condition::all()
                        .add(stock_shelves::Column::TenantId.eq(tenant_id))
                        .add(stock_shelves::Column::ShelfId.eq(shelf_id))
                        .add(stocks::Column::Id.gt(after))
                        .add(shelves::Column::Publish.eq(Some(1))),
                )
                .limit(limit)
                .join_rev(
                    JoinType::InnerJoin,
                    stock_shelves::Entity::belongs_to(Stocks)
                        .from(stock_shelves::Column::StockId)
                        .to(stocks::Column::Id)
                        .into(),
                )
                .select_only()
                .column(stocks::Column::Id)
                .column(stocks::Column::Name)
                .column(stocks::Column::Unit)
                .expr_as(
                    Expr::col((items::Entity, items::Column::Id)).count(),
                    "quantity",
                )
                .group_by(stocks::Column::Id)
                .group_by(stocks::Column::Name)
                .group_by(stocks::Column::Unit)
                .order_by_asc(stocks::Column::Id)
                .into_tuple::<(i32, String, String, i64)>()
                .all(self.dbt(tenant_id))
                .await?
                .into_iter()
                .map(|(id, name, unit, quantity)| Stock {
                    id: Some(id),
                    lots: None,
                    shelves: None,
                    quantity: Some(quantity as i32),
                    cost_price: None,
                    name,
                    unit,
                })
                .collect())
        } else {
            Ok(Stocks::find()
                .filter(
                    Condition::all()
                        .add(stock_shelves::Column::TenantId.eq(tenant_id))
                        .add(stock_shelves::Column::ShelfId.eq(shelf_id))
                        .add(stocks::Column::Id.gt(after)),
                )
                .limit(limit)
                .join_rev(
                    JoinType::InnerJoin,
                    stock_shelves::Entity::belongs_to(Stocks)
                        .from(stock_shelves::Column::StockId)
                        .to(stocks::Column::Id)
                        .into(),
                )
                .select_only()
                .column(stocks::Column::Id)
                .column(stocks::Column::Name)
                .column(stocks::Column::Unit)
                .expr_as(
                    Expr::col((items::Entity, items::Column::Id)).count(),
                    "quantity",
                )
                .group_by(stocks::Column::Id)
                .group_by(stocks::Column::Name)
                .group_by(stocks::Column::Unit)
                .order_by_asc(stocks::Column::Id)
                .into_tuple::<(i32, String, String, i64)>()
                .all(self.dbt(tenant_id))
                .await?
                .into_iter()
                .map(|(id, name, unit, quantity)| Stock {
                    id: Some(id),
                    lots: None,
                    shelves: None,
                    quantity: Some(quantity as i32),
                    cost_price: None,
                    name,
                    unit,
                })
                .collect())
        }
    }

    pub async fn list_paginated_shelves(
        &self,
        tenant_id: i32,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Shelf>, DbErr> {
        Ok(Shelves::find()
            .filter(shelves::Column::TenantId.eq(tenant_id))
            .filter(shelves::Column::Id.gt(after))
            .order_by_asc(shelves::Column::Id)
            .limit(limit)
            .all(self.dbt(tenant_id))
            .await?
            .iter()
            .map(|it| Shelf {
                id: Some(it.id),
                name: Some(it.name.clone()),
                description: it.description.clone(),
            })
            .collect::<Vec<_>>())
    }

    pub async fn get_item_by_barcode(
        &self,
        tenant_id: i32,
        barcode: &String,
    ) -> Result<Item, DbErr> {
        let ret = Items::find()
            .filter(items::Column::TenantId.eq(tenant_id))
            .filter(items::Column::Barcode.eq(barcode))
            .join_rev(
                JoinType::InnerJoin,
                lots::Entity::belongs_to(Items)
                    .from(lots::Column::Id)
                    .to(items::Column::LotId)
                    .into(),
            )
            .join_rev(
                JoinType::InnerJoin,
                shelves::Entity::belongs_to(Items)
                    .from(shelves::Column::Id)
                    .to(items::Column::ShelfId)
                    .into(),
            )
            .select_only()
            .column(items::Column::Id)
            .column(items::Column::LotId)
            .column(items::Column::OrderId)
            .column(items::Column::StockId)
            .column(items::Column::ExpiredAt)
            .column(items::Column::CostPrice)
            .column(lots::Column::LotNumber)
            .column(shelves::Column::Name)
            .into_tuple::<(
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<DateTime<Utc>>,
                f64,
                Option<String>,
                Option<String>,
            )>()
            .one(self.dbt(tenant_id))
            .await?;

        if let Some((id, lot_id, order_id, stock_id, expired_at, cost_price, lot_number, shelf)) =
            ret
        {
            Ok(Item {
                lot_id,
                stock_id,
                expired_at,
                shelf,
                lot_number,
                cost_price,

                id: Some(id),
                barcode: Some(barcode.clone()),
                status: (if order_id.is_none() {
                    "in-stock"
                } else {
                    "sold-out"
                })
                .to_string(),
            })
        } else {
            Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found barcode {}",
                barcode
            ))))
        }
    }

    pub async fn sale_at_storefront(&self, tenant_id: i32, sale: &Sale) -> Result<Sale, DbErr> {
        match &sale.barcodes {
            Some(barcodes) => {
                let txn = self.dbt(tenant_id).begin().await?;

                Sales::insert(sales::ActiveModel {
                    tenant_id: Set(tenant_id),
                    order_id: Set(sale.order_id),
                    cost_price: Set(sale.cost_prices.iter().sum()),
                    ..Default::default()
                })
                .exec(&txn)
                .await?;

                let result = Items::update_many()
                    .col_expr(items::Column::Status, Expr::value(2))
                    .col_expr(items::Column::OrderId, Expr::value(sale.order_id))
                    .filter(items::Column::Barcode.is_in(barcodes.clone()))
                    .filter(items::Column::Status.eq(1))
                    .exec(&txn)
                    .await?;
                if result.rows_affected < barcodes.len() as u64 {
                    txn.rollback().await?;
                    Err(DbErr::Query(RuntimeErr::Internal(format!(
                        "Some items are not existed, please check"
                    ))))
                } else {
                    txn.commit().await?;
                    Ok(sale.clone())
                }
            }
            None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Missing field `barcodes`"
            )))),
        }
    }

    pub async fn sale_at_website(&self, tenant_id: i32, sale: &Sale) -> Result<Sale, DbErr> {
        match &sale.stock_ids {
            Some(stock_ids) => {
                let txn = self.dbt(tenant_id).begin().await?;

                Sales::insert(sales::ActiveModel {
                    tenant_id: Set(tenant_id),
                    order_id: Set(sale.order_id),
                    cost_price: Set(sale.cost_prices.iter().sum()),
                    ..Default::default()
                })
                .exec(&txn)
                .await?;

                let sale_id = Sales::find()
                    .filter(sales::Column::OrderId.eq(sale.order_id))
                    .filter(sales::Column::TenantId.eq(tenant_id))
                    .one(&txn)
                    .await?
                    .ok_or(DbErr::RecordNotFound("Sale not found".to_string()))?
                    .id;

                SaleEvents::insert_many(
                    stock_ids
                        .iter()
                        .map(|stock_id| sale_events::ActiveModel {
                            tenant_id: Set(tenant_id),
                            sale_id: Set(sale_id),
                            stock_id: Set(*stock_id),
                            status: Set(0), // 0 = pending
                            ..Default::default()
                        })
                        .collect::<Vec<_>>(),
                )
                .exec(&txn)
                .await?;
                Ok(sale.clone())
            }
            None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Missing field `stocks`"
            )))),
        }
    }
}
