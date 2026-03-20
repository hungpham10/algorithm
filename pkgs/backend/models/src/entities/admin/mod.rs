mod api_map;
mod article_map;
mod file_map;
mod sitemap;
mod tenant;
mod token_map;

pub use api_map::Entity as ApiMap;
pub use article_map::Entity as ArticleMap;
pub use file_map::Entity as FileMap;
pub use sitemap::Entity as Sitemap;
pub use tenant::Entity as Tenant;
pub use token_map::Entity as TokenMap;

use std::collections::HashMap;
use std::env;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, RuntimeErr, Set,
};

use algorithm::{Operator, LruCache};
use chrono::{DateTime, Utc};
use integration::Api as ApiEngine;
use rand::{RngCore, thread_rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

use crate::resolver::Resolver;

pub struct Admin {
    // @NOTE: controller
    resolver: Arc<Resolver>,
    api: Arc<ApiEngine>,

    // @NOTE: caching
    cache_unencrypted_tokens: Arc<LruCache<(i64, String), Option<String>, 32>>,
    cache_api_info_by_name: Arc<LruCache<String, Option<Api>, 32>>,
    cache_api_info_by_id: Arc<LruCache<i64, Option<Api>, 32>>,
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
    pub last_mod: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ApiType {
    Unknown,
    Create,
    Delete,
    Update,
    Read,
}

impl From<i32> for ApiType {
    fn from(value: i32) -> Self {
        match value {
            1 => ApiType::Create,
            2 => ApiType::Delete,
            3 => ApiType::Update,
            4 => ApiType::Read,
            _ => ApiType::Unknown,
        }
    }
}

impl From<String> for ApiType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "create" => ApiType::Create,
            "read" => ApiType::Read,
            "update" => ApiType::Update,
            "delete" => ApiType::Delete,
            _ => ApiType::Unknown,
        }
    }
}

impl Display for ApiType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ApiType::Unknown => write!(f, "unknown"),
            ApiType::Create => write!(f, "create"),
            ApiType::Read => write!(f, "read"),
            ApiType::Update => write!(f, "update"),
            ApiType::Delete => write!(f, "delete"),
        }
    }
}

impl<'de> Deserialize<'de> for ApiType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ApiType::from(s))
    }
}

impl serde::Serialize for ApiType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Api {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub mode: Option<ApiType>,
    pub url: Option<String>,
    pub parser: Option<Vec<Operator>>,
}

impl Admin {
    pub fn new(resolver: &Arc<Resolver>) -> Self {
        // @TODO: có cách nào lấy dữ liêụ từ resolver về capacity của cache_unencrypted_tokens và api
        Self {
            resolver: resolver.clone(),
            api: Arc::new(ApiEngine::new(10 * 32)),
            cache_unencrypted_tokens: Arc::new(LruCache::new(10 * 32)),
            cache_api_info_by_name: Arc::new(LruCache::new(10 * 32)),
            cache_api_info_by_id: Arc::new(LruCache::new(10 * 32)),
        }
    }

    fn dbt(&self, tenant_id: i64) -> &DatabaseConnection {
        self.resolver.database(tenant_id)
    }

    async fn get_master_key(&self) -> Result<Vec<u8>, DbErr> {
        // TODO: Sau này thay thế đoạn này bằng gọi KMS SDK
        env::var("MASTER_KEY")
            .map(|s| s.into_bytes())
            .map_err(|_| DbErr::Custom("Missing MASTER_KEY".into()))
    }

    // @TODO: refresh cache

    // --------------------------------------------------------------
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

    pub async fn get_full_path(&self, tenant_id: i64, path: &String) -> Result<String, DbErr> {
        match FileMap::find()
            .filter(file_map::Column::TenantId.eq(tenant_id))
            .filter(file_map::Column::Src.eq(path))
            .select_only()
            .column(file_map::Column::Dest)
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

    // --------------------------------------------------------------
    pub async fn list_sites(&self, tenant_id: i64) -> Result<Vec<Site>, DbErr> {
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
                last_mod: *lastmod,
            })
            .collect::<Vec<_>>())
    }

    pub async fn list_articles(&self, tenant_id: i64) -> Result<Vec<Article>, DbErr> {
        Ok(ArticleMap::find()
            .filter(article_map::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(article_map::Column::Loc)
            .column(article_map::Column::Name)
            .column(article_map::Column::Title)
            .column(article_map::Column::Language)
            .column(article_map::Column::Keywords)
            .column(article_map::Column::CreatedAt)
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

    // --------------------------------------------------------------
    pub async fn get_unencrypted_token(
        &self,
        tenant_id: i64,
        service_name: &String,
    ) -> Result<String, DbErr> {
        let cache_key = (tenant_id, service_name.clone());

        match self.cache_unencrypted_tokens.get(&cache_key) {
            Some(Some(token)) => Ok(token),
            Some(None) => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found service {}, tenant {}",
                service_name, tenant_id,
            )))),
            None => {
                let cache_key_after_done = cache_key.clone();

                self.cache_unencrypted_tokens.put(cache_key, None);

                let encrypted_bytes = TokenMap::find()
                    .filter(token_map::Column::TenantId.eq(tenant_id))
                    .filter(token_map::Column::Service.eq(service_name))
                    .select_only()
                    .column(token_map::Column::Token)
                    .into_tuple::<Vec<u8>>()
                    .one(self.dbt(tenant_id))
                    .await?
                    .ok_or_else(|| {
                        DbErr::Query(RuntimeErr::Internal(format!(
                            "Not found service {}, tenant {}",
                            service_name, tenant_id,
                        )))
                    })?;

                let master_key_str = self.get_master_key().await?;
                let key = Key::<Aes256Gcm>::from_slice(&master_key_str.as_slice());
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
                let token = String::from_utf8(decrypted_bytes).map_err(|error| {
                    DbErr::Query(RuntimeErr::Internal(format!(
                        "Validate token of service {}, tenant {} failed: {}",
                        service_name, tenant_id, error,
                    )))
                })?;

                self.cache_unencrypted_tokens.put(
                    cache_key_after_done,
                    Some(token.clone())
                );
                Ok(token)
            }
        }
    }

    pub async fn put_unencrypted_token(
        &self,
        tenant_id: i64,
        service_name: &String,
        token_plain: &String,
    ) -> Result<(), DbErr> {
        let master_key_str = self.get_master_key().await?;

        let key = Key::<Aes256Gcm>::from_slice(&master_key_str.as_slice());
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

        token_map::Entity::insert(token_map::ActiveModel {
            tenant_id: Set(tenant_id),
            service: Set(service_name.to_owned()),
            token: Set(final_blob),
            ..Default::default()
        })
        .on_conflict(
            sea_query::OnConflict::columns([
                token_map::Column::TenantId,
                token_map::Column::Service,
            ])
            .update_column(token_map::Column::Token)
            .update_column(token_map::Column::UpdatedAt)
            .to_owned(),
        )
        .exec(self.dbt(tenant_id))
        .await?;

        self.cache_unencrypted_tokens.put(
            (tenant_id, service_name.clone()),
            Some(token_plain.clone()),
        );
        Ok(())
    }

    pub async fn list_supported_services(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<String>, DbErr> {
        Ok(TokenMap::find()
            .filter(token_map::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(token_map::Column::Service)
            .into_tuple::<String>()
            .all(self.dbt(tenant_id))
            .await?)
    }

    // --------------------------------------------------------------
    pub async fn list_paginated_api_schema(
        &self,
        tenant_id: i64,
        after: i64,
        limit: u64,
    ) -> Result<Vec<Api>, DbErr> {
        Ok(ApiMap::find()
            .filter(api_map::Column::TenantId.eq(tenant_id))
            .filter(api_map::Column::Id.gt(after))
            .select_only()
            .column(api_map::Column::Id)
            .column(api_map::Column::Mode)
            .column(api_map::Column::Name)
            .column(api_map::Column::Url)
            .column(api_map::Column::Parser)
            .limit(limit)
            .into_tuple::<(i64, i32, String, String, api_map::Parser)>()
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|(id, mode, name, url, parser)| Api {
                id: Some(id),
                mode: Some(ApiType::from(mode)),
                name: Some(name),
                url: Some(url),
                parser: Some(parser.0.clone()),
            })
            .collect())
    }

    pub async fn get_api_schema_by_name(
        &self,
        tenant_id: i64,
        name: &String,
        method: ApiType,
    ) -> Result<Api, DbErr> {
        let cache_key = format!("{name}:{method}");

        match self.cache_api_info_by_name.get(&cache_key) {
            Some(Some(api_info)) => Ok(api_info),
            Some(None) => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found api {} for tenant_id {} ",
                name, tenant_id,
            )))),
            None => {
                let cache_key_after_done = cache_key.clone();

                self.cache_api_info_by_name.put(cache_key, None);

                match ApiMap::find()
                    .select_only()
                    .column(api_map::Column::Id)
                    .column(api_map::Column::Mode)
                    .column(api_map::Column::Name)
                    .column(api_map::Column::Url)
                    .column(api_map::Column::Parser)
                    .filter(api_map::Column::TenantId.eq(tenant_id))
                    .filter(api_map::Column::Name.eq(name))
                    .filter(api_map::Column::Mode.eq(method as i32))
                    .into_tuple::<(i64, i32, String, String, api_map::Parser)>()
                    .one(self.dbt(tenant_id))
                    .await?
                {
                    Some((id, mode, name, url, parser)) => {
                        let api_info = Api {
                            id: Some(id),
                            mode: Some(ApiType::from(mode)),
                            name: Some(name),
                            url: Some(url),
                            parser: Some(parser.0.clone()),
                        };

                        self.cache_api_info_by_name.put(
                            cache_key_after_done,
                            Some(api_info.clone())
                        );
                        Ok(api_info)
                    },
                    None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                        "Not found api {} for tenant_id {} ",
                        name, tenant_id,
                    )))),
                }
            }
        }
    }

    pub async fn get_api_schema_by_id(&self, tenant_id: i64, id: i64) -> Result<Api, DbErr> {
        match self.cache_api_info_by_id.get(&id) {
            Some(Some(api_info)) => Ok(api_info),
            Some(None) => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found api schema for tenant_id {} and id {}",
                            tenant_id, id,
            )))),
            None => {
                match ApiMap::find()
                    .filter(api_map::Column::TenantId.eq(tenant_id))
                    .filter(api_map::Column::Id.eq(id))
                    .into_tuple::<(i32, String, String, api_map::Parser)>()
                    .one(self.dbt(tenant_id))
                    .await?
                    .map(|(mode, name, url, parser)| Api {
                        id: None,
                        mode: Some(ApiType::from(mode)),
                        name: Some(name),
                        url: Some(url),
                        parser: Some(parser.0.clone()),
                    })
                {
                    Some(api_info) => {
                        self.cache_api_info_by_id.put(id, Some(api_info.clone()));
                        Ok(api_info)
                    }
                    None => {
                        self.cache_api_info_by_id.put(id, None);
                        Err(DbErr::Query(RuntimeErr::Internal(format!(
                            "Not found api schema for tenant_id {} and id {}",
                            tenant_id, id,
                        ))))
                    }
                }
            }
        }
    }

    pub async fn create_api_schemas(
        &self,
        tenant_id: i64,
        schemas: Vec<Api>,
    ) -> Result<Vec<Api>, DbErr> {
        let mut active_models = Vec::new();

        for (i, schema) in schemas.iter().enumerate() {
            active_models.push(api_map::ActiveModel {
                tenant_id: Set(tenant_id),
                name: Set(schema
                    .name
                    .clone()
                    .ok_or_else(|| DbErr::Custom(format!("`name` is missing in schema {}", i)))?),
                url: Set(schema
                    .url
                    .clone()
                    .ok_or_else(|| DbErr::Custom(format!("`url` is missing in schema {}", i)))?),
                mode: Set(schema
                    .mode
                    .clone()
                    .ok_or_else(|| DbErr::Custom(format!("`mode` is missing in schema {}", i)))?
                    as i32),
                parser: Set(api_map::Parser(schema.parser.clone().ok_or_else(|| {
                    DbErr::Custom(format!("`parser` is missing in schema {}", i))
                })?)),
                ..Default::default()
            });
        }

        ApiMap::insert_many(active_models)
            .exec(self.dbt(tenant_id))
            .await?;

        Ok(ApiMap::find()
            .filter(api_map::Column::TenantId.eq(tenant_id))
            .filter(
                api_map::Column::Name
                    .is_in(schemas.iter().map(|s| s.name.clone()).collect::<Vec<_>>()),
            )
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|m| Api {
                id: Some(m.id),
                name: Some(m.name),
                mode: Some(ApiType::from(m.mode)),
                url: Some(m.url),
                parser: Some(m.parser.0),
            })
            .collect::<Vec<_>>())
    }

    pub async fn perform_api_by_api_id(
        &self,
        tenant_id: i64,
        query_id: i64,
        args: Vec<String>,
        headers: HashMap<String, String>,
        body: Option<Value>,
    ) -> Result<Vec<Value>, DbErr> {
        self.perform_api_by_api_info(
            &self.get_api_schema_by_id(
                tenant_id,
                query_id,
            )
            .await?,
            args,
            headers,
            body,
        )
        .await
    }

    pub async fn perform_api_by_api_name(
        &self,
        tenant_id: i64,
        name: &String,
        mode: ApiType,
        args: Vec<String>,
        headers: HashMap<String, String>,
        body: Option<Value>,
    ) -> Result<Vec<Value>, DbErr> {
        self.perform_api_by_api_info(
            &self.get_api_schema_by_name(
                tenant_id,
                name,
                mode,
            )
            .await?,
            args,
            headers,
            body,
        )
        .await
    }

    async fn perform_api_by_api_info(
        &self,
        api_info: &Api,
        args: Vec<String>,
        headers: HashMap<String, String>,
        body: Option<Value>,
    ) -> Result<Vec<Value>, DbErr> {
        let mut url = api_info.url.clone()
            .ok_or_else(|| DbErr::Query(RuntimeErr::Internal(
                "Api is broken, missing `url`".into()
            )))?;
        let mode = api_info.mode
            .ok_or_else(|| DbErr::Query(RuntimeErr::Internal(
                "Api is broken, missing `mode`".into()
            )))?;
        let parser = api_info.parser.as_ref()
            .ok_or_else(|| DbErr::Query(RuntimeErr::Internal(
                "Api is broken, missing `template`".into()
            )))?;

        for arg in args {
            url = url.replacen("{}", &arg, 1);
        }

        if body.is_none() {
            if ApiType::from(mode) == ApiType::Create
                || ApiType::from(mode) == ApiType::Update
            {
                return Err(DbErr::Query(RuntimeErr::Internal(format!(
                    "No body provided for create or update API type: {}",
                    mode
                ))));
            }
        }

        match ApiType::from(mode) {
            ApiType::Create => self
                .api
                .create(
                    url.as_str(),
                    &Arc::new(algorithm::JsonQuery::new(parser.clone())),
                    &headers,
                    body.unwrap(),
                )
                .await
                .map_err(|error| {
                    DbErr::Query(RuntimeErr::Internal(format!(
                        "Error creating JSON by query: {}",
                        error
                    )))
                }),
            ApiType::Read => self
                .api
                .read(
                    url.as_str(),
                    &Arc::new(algorithm::JsonQuery::new(parser.clone())),
                    &headers,
                )
                .await
                .map_err(|error| {
                    DbErr::Query(RuntimeErr::Internal(format!(
                        "Error getting JSON by query: {}",
                        error
                    )))
                }),
            ApiType::Update => self
                .api
                .update(
                    url.as_str(),
                    &Arc::new(algorithm::JsonQuery::new(parser.clone())),
                    &headers,
                    body.unwrap(),
                )
                .await
                .map_err(|error| {
                    DbErr::Query(RuntimeErr::Internal(format!(
                        "Error updating JSON by query: {}",
                        error
                    )))
                }),

            ApiType::Delete => self
                .api
                .delete(
                    url.as_str(),
                    &Arc::new(algorithm::JsonQuery::new(parser.clone())),
                    &headers,
                )
                .await
                .map_err(|error| {
                    DbErr::Query(RuntimeErr::Internal(format!(
                        "Error deleting JSON by query: {}",
                        error
                    )))
                }),
            _ => {
                return Err(DbErr::Query(RuntimeErr::Internal(format!(
                    "Unknown API type: {}",
                    mode
                ))));
            }
        }
    }
}
