use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ohcl_mapping_broker_resolution")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub broker_id: Option<i32>,
    pub resolution_id: Option<i32>,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
    pub resolution: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
