use super::items::Entity as Items;
use super::lots::Entity as Lots;
use super::sale_events::Entity as SaleEvents;
use super::sales::Entity as Sales;
use super::shelves::Entity as Shelves;
use super::stock_shelves::Entity as StockShelves;
use super::stocks::Entity as Stocks;
use super::{
    items, lots, sale_events, sales, shelves, stock_shelves, stocks, Item, Lot, Sale, Shelf, Stock,
};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use sea_orm::entity::prelude::Expr;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, RuntimeErr, Set, TransactionTrait,
};

pub struct Wms {
    db: Vec<Arc<DatabaseConnection>>,
}

impl Wms {
    pub fn new(db: Vec<Arc<DatabaseConnection>>) -> Self {
        Self { db }
    }

    fn get_db_id(&self, tenant_id: i32) -> usize {
        ((tenant_id as usize) % self.db.len())
    }

    async fn create_stocks(&self, tenant_id: i32, stocks: &[Stock]) -> Result<Vec<i32>, DbErr> {
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
                    quantity: Set(0),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        )
        .all(&*self.db[self.get_db_id(tenant_id)])
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
            .all(&*self.db[self.get_db_id(tenant_id)])
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
        .exec(&*self.db[self.get_db_id(tenant_id)])
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
            .all(&*self.db[self.get_db_id(tenant_id)])
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<_>>())
    }

    pub async fn create_lots(&self, tenant_id: i32, lots: &[Lot]) -> Result<Vec<i32>, DbErr> {
        if lots.is_empty() {
            return Ok(vec![]);
        }

        let models: Vec<lots::ActiveModel> = lots
            .iter()
            .map(|l| lots::ActiveModel {
                tenant_id: Set(tenant_id),
                lot_number: Set(l.lot_number.clone()),
                quantity: Set(l.quantity),
                supplier: Set(l.supplier.clone()),
                entry_date: Set(l.entry_date.unwrap_or_else(chrono::Utc::now)),
                cost_price: Set(l.cost_price),
                status: Set(l.status.clone()),
                ..Default::default()
            })
            .collect();

        lots::Entity::insert_many(models)
            .exec(&*self.db[self.get_db_id(tenant_id)])
            .await?;

        let lot_numbers: Vec<String> = lots.iter().map(|l| l.lot_number.clone()).collect();

        let inserted: Vec<i32> = lots::Entity::find()
            .select_only()
            .column(lots::Column::Id)
            .filter(lots::Column::TenantId.eq(tenant_id))
            .filter(lots::Column::LotNumber.is_in(lot_numbers))
            .all(&*self.db[self.get_db_id(tenant_id)])
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect();

        Ok(inserted)
    }

    pub async fn plan_import_new_items(
        &self,
        tenant_id: i32,
        items: &[Item],
    ) -> Result<Vec<i32>, DbErr> {
        // @NOTE: sharding by stock_id
    }

    pub async fn import_real_items(
        &self,
        tenant_id: i32,
        lot_id: i32,
        items: &[Item],
    ) -> Result<Vec<Item>, DbErr> {
    }

    pub async fn assign_items_to_shelf(
        &self,
        tenant_id: i32,
        shelf_id: i32,
        items: &[Item],
    ) -> Result<(), DbErr> {
    }

    pub async fn get_stock(&self, tenant_id: i32, stock_id: i32) -> Result<Stock, DbErr> {
        let result = Stocks::find()
            .filter(stocks::Column::TenantId.eq(tenant_id))
            .filter(stocks::Column::Id.eq(stock_id))
            .one(&*self.db[self.get_db_id(tenant_id)])
            .await?;

        if let Some(result) = result {
            Ok(Stock {
                id: Some(result.id),
                quantity: Some(result.quantity),
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
        let items = Stocks::find()
            .filter(stocks::Column::TenantId.eq(tenant_id))
            .filter(stocks::Column::Id.gt(after))
            .order_by_asc(stocks::Column::Id)
            .limit(limit)
            .all(&*self.db[self.get_db_id(tenant_id)])
            .await?
            .iter()
            .map(|it| Stock {
                id: Some(it.id),
                quantity: Some(it.quantity),
                name: it.name.clone(),
                unit: it.unit.clone(),
                cost_price: None,
                lots: None,
                shelves: None,
            })
            .collect::<Vec<_>>();

        if !include_details {
            return Ok(items);
        }

        // @NOTE: this flow only work to pull data for filling
        // cache only, the cache server will be used to provide
        // data in realtime
    }

    pub async fn get_lot(&self, tenant_id: i32, lot_id: i32) -> Result<Lot, DbErr> {
        let result = Lots::find()
            .filter(lots::Column::TenantId.eq(tenant_id))
            .filter(lots::Column::Id.eq(lot_id))
            .one(&*self.db[self.get_db_id(tenant_id)])
            .await?;

        if let Some(result) = result {
            Ok(Lot {
                id: Some(result.id),
                entry_date: Some(result.entry_date),
                lot_number: result.lot_number.to_string(),
                quantity: result.quantity,
                cost_price: result.cost_price,
                supplier: result.supplier.clone(),
                status: result.status.clone(),
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
    }

    pub async fn list_paginated_stocks_of_shelf(
        &self,
        tenant_id: i32,
        shelf_id: i32,
        is_publish: bool,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Stock>, DbErr> {
    }

    pub async fn list_paginated_shelves(
        &self,
        tenant_id: i32,
        after: i32,
        limit: u64,
    ) -> Result<Vec<Shelf>, DbErr> {
    }

    pub async fn get_item_by_barcode(
        &self,
        tenant_id: i32,
        barcode: &String,
    ) -> Result<Item, DbErr> {
    }

    pub async fn sale_at_storefront(&self, tenant_id: i32, sale: &Sale) -> Result<Sale, DbErr> {}

    pub async fn sale_at_website(&self, tenant_id: i32, sale: &Sale) -> Result<Sale, DbErr> {
        match &sale.stock_ids {
            Some(stock_ids) => {
                let txn = self.db[self.get_db_id(tenant_id)].begin().await?;

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
