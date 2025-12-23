mod articlemap;
mod filemap;
mod sitemap;
mod tenant;
mod tokenmap;

pub use articlemap::Entity as Articlemap;
pub use filemap::Entity as Filemap;
pub use sitemap::Entity as Sitemap;
pub use tenant::Entity as Tenant;
pub use tokenmap::Entity as Tokenmap;

use std::env;
use std::sync::Arc;

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr, Set,
};

use chrono::{DateTime, Utc};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

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

    pub async fn get_full_path(&self, tenant_id: i32, path: &String) -> Result<String, DbErr> {
        match Filemap::find()
            .filter(filemap::Column::TenantId.eq(tenant_id))
            .filter(filemap::Column::Src.eq(path))
            .select_only()
            .column(filemap::Column::Dest)
            .into_tuple::<String>()
            .one(self.dbt(tenant_id))
            .await?
        {
            Some(dest) => Ok(dest),
            None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found path {}",
                path,
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

    pub async fn get_unencrypted_token(
        &self,
        tenant_id: i32,
        service_name: &String,
    ) -> Result<String, DbErr> {
        let encrypted_bytes = Tokenmap::find()
            .filter(tokenmap::Column::TenantId.eq(tenant_id))
            .filter(tokenmap::Column::Service.eq(service_name))
            .select_only()
            .column(tokenmap::Column::Token)
            .into_tuple::<Vec<u8>>()
            .one(self.dbt(tenant_id))
            .await?
            .ok_or_else(|| {
                DbErr::Query(RuntimeErr::Internal(format!(
                    "Not found service {}, tenant {}",
                    service_name, tenant_id,
                )))
            })?;

        let master_key_str = env::var("MASTER_KEY").map_err(|_| {
            DbErr::Query(RuntimeErr::Internal(format!(
                "Not found MASTER_KEY while decoding service {}",
                service_name,
            )))
        })?;

        let key = Key::<Aes256Gcm>::from_slice(master_key_str.as_bytes());
        let cipher = Aes256Gcm::new(key);

        if encrypted_bytes.len() < 12 {
            return Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Data too short when decoding service {}, tenant {}",
                service_name, tenant_id,
            ))));
        }

        let (nonce_part, ciphertext_part) = encrypted_bytes.split_at(12);
        let nonce = Nonce::from_slice(nonce_part);

        let decrypted_bytes = cipher.decrypt(nonce, ciphertext_part).map_err(|error| {
            DbErr::Query(RuntimeErr::Internal(format!(
                "Decode service {}, tenant {} failed: {}",
                service_name, tenant_id, error,
            )))
        })?;

        String::from_utf8(decrypted_bytes).map_err(|error| {
            DbErr::Query(RuntimeErr::Internal(format!(
                "Validate token of service {}, tenant {} failed: {}",
                service_name, tenant_id, error,
            )))
        })
    }

    pub async fn put_unencrypted_token(
        &self,
        tenant_id: i32,
        service_name: &String,
        token_plain: &String,
    ) -> Result<(), DbErr> {
        let master_key_str = env::var("MASTER_KEY").map_err(|_| {
            DbErr::Query(RuntimeErr::Internal(format!(
                "Not found MASTER_KEY while encrypting service {}",
                service_name,
            )))
        })?;

        let key = Key::<Aes256Gcm>::from_slice(master_key_str.as_bytes());
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, token_plain.as_bytes())
            .map_err(|error| {
                DbErr::Query(RuntimeErr::Internal(format!(
                    "Encrypt service {}, tenant {} failed: {}",
                    service_name, tenant_id, error,
                )))
            })?;

        let mut final_blob = nonce_bytes.to_vec();
        final_blob.extend_from_slice(&ciphertext);

        tokenmap::Entity::insert(tokenmap::ActiveModel {
            tenant_id: Set(tenant_id),
            service: Set(service_name.to_owned()),
            token: Set(final_blob),
            ..Default::default()
        })
        .on_conflict(
            sea_query::OnConflict::columns([tokenmap::Column::TenantId, tokenmap::Column::Service])
                .update_column(tokenmap::Column::Token)
                .update_column(tokenmap::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(self.dbt(tenant_id))
        .await?;

        Ok(())
    }

    pub async fn list_supported_services(&self, tenant_id: i32) -> Result<Vec<String>, DbErr> {
        Ok(Tokenmap::find()
            .filter(tokenmap::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(tokenmap::Column::Service)
            .into_tuple::<String>()
            .all(self.dbt(tenant_id))
            .await?)
    }
}
