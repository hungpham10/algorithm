mod sitemap;
mod tenant;

pub use sitemap::Entity as Sitemap;
pub use tenant::Entity as Tenant;

use std::sync::Arc;

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr,
};

use chrono::{DateTime, Utc};
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

    pub async fn get_tenant_id(&self, host: &String) -> Result<i32, DbErr> {
        match Tenant::find()
            .filter(tenant::Column::Host.eq(host))
            .select_only()
            .column(tenant::Column::Id)
            .into_tuple::<i32>()
            .one(&*self.db)
            .await?
        {
            Some(id) => Ok(id),
            None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found host {}",
                host,
            )))),
        }
    }

    pub async fn list_sites(&self, tenant_id: i32) -> Result<Vec<Site>, DbErr> {
        Ok(Sitemap::find()
            .filter(sitemap::Column::TenantId.eq(tenant_id))
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
