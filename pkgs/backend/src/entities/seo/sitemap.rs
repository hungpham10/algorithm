use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "seo_sitemap")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub host: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub loc: String,
    pub freq: String,
    pub priority: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
