use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ohcl_broker_limitation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub broker_id: i32,
    pub created_at: Option<DateTimeUtc>,
    pub updated_at: Option<DateTimeUtc>,
    pub guest_max_history_days: i32,
    pub guest_invisible: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
