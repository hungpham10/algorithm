use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ohcl_mapping_product_in_store_to_symbol")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub symbol: Option<i32>,
    pub store: i32,
    pub product_name: String,
    pub layer: i32,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
