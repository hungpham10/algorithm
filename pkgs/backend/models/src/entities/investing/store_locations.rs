use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ohcl_store_locations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub store: i32,
    pub address_line: String,
    pub district: String,
    pub province: String,
    pub latitude: f32,
    pub longitude: f32,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
