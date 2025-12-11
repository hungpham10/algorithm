mod articlemap;
mod sitemap;
mod tenant;

pub use articlemap::Entity as Articlemap;
pub use sitemap::Entity as Sitemap;
pub use tenant::Entity as Tenant;

use std::sync::Arc;

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub struct Seo {
    db: Vec<Arc<DatabaseConnection>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Article {
    pub title: String,
    pub loc: String,
    pub name: String,
    pub language: String,
    pub keywords: Option<String>,
    pub published_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Site {
    pub loc: String,
    pub freq: String,
    pub priority: f64,
    pub lastmod: DateTime<Utc>,
}

impl Seo {
    pub fn new(db: Vec<Arc<DatabaseConnection>>) -> Self {
        Self { db }
    }

    fn dbt(&self, tenant_id: i32) -> &DatabaseConnection {
        self.db[(tenant_id as usize) % self.db.len()].as_ref()
    }

    pub async fn get_tenant_id(&self, host: &String) -> Result<i32, DbErr> {
        match Tenant::find()
            .filter(tenant::Column::Host.eq(host))
            .select_only()
            .column(tenant::Column::Id)
            .into_tuple::<i32>()
            .one(self.dbt(0))
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
            .all(self.dbt(tenant_id))
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

    pub async fn list_articles(&self, tenant_id: i32) -> Result<Vec<Article>, DbErr> {
        Ok(Articlemap::find()
            .filter(articlemap::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(articlemap::Column::Loc)
            .column(articlemap::Column::Name)
            .column(articlemap::Column::Title)
            .column(articlemap::Column::Language)
            .column(articlemap::Column::Keywords)
            .column(articlemap::Column::CreatedAt)
            .into_tuple::<(
                String,
                String,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
            )>()
            .all(self.dbt(tenant_id))
            .await?
            .iter()
            .map(
                |(loc, name, title, language, keywords, published_at)| Article {
                    loc: loc.clone(),
                    name: name.clone(),
                    title: title.clone(),
                    language: language.clone(),
                    keywords: keywords.clone(),
                    published_at: *published_at,
                },
            )
            .collect::<Vec<_>>())
    }
}
