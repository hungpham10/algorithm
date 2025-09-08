mod sitemap;

pub use sitemap::Entity as Sitemap;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, JoinType, QueryFilter,
    QueryOrder, QuerySelect, RuntimeErr, Set, TransactionTrait,
};

use serde::{Deserialize, Serialize};
pub struct Seo {
    db: Arc<DatabaseConnection>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Site {
    pub loc: String,
    pub freq: String,
    pub priority: f64,
    pub lastmod: DateTime<Utc>,
}

impl Seo {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_sites(&self, host: &String) -> Result<Vec<Site>, DbErr> {
        Ok(Sitemap::find()
            .filter(sitemap::Column::Host.eq(host))
            .select_only()
            .column(sitemap::Column::Loc)
            .column(sitemap::Column::Freq)
            .column(sitemap::Column::Priority)
            .column(sitemap::Column::CreatedAt)
            .into_tuple::<(String, String, f64, DateTime<Utc>)>()
            .all(&*self.db)
            .await?
            .iter()
            .map(|(loc, freq, priority, lastmod)| Site {
                loc: loc.clone(),
                freq: freq.clone(),
                priority: *priority,
                lastmod: *lastmod,
            })
            .collect::<Vec<_>>())
    }
}
