mod api_map;
mod article_map;
mod database_map;
mod file_map;
mod sitemap;
mod table_map;
mod tenant;
mod token_map;

pub use api_map::Entity as ApiMap;
pub use article_map::Entity as ArticleMap;
pub use database_map::Entity as DatabaseMap;
pub use file_map::Entity as FileMap;
pub use sitemap::Entity as Sitemap;
pub use table_map::Entity as TableMap;
pub use tenant::Entity as Tenant;
pub use token_map::Entity as TokenMap;

use std::collections::HashMap;
use std::env;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use sea_orm::sea_query::{Alias, Condition, Expr, OnConflict, Query};
use sea_orm::{
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    DatabaseTransaction, DbErr, EntityTrait, ExprTrait, QueryFilter, QuerySelect, RuntimeErr, Set,
    TransactionTrait, Value as OrmValue,
};

use algorithm::{LruCache, Operator};
use chrono::{DateTime, Utc};
use integration::Api as ApiEngine;
use rand::{RngCore, thread_rng};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};

use crate::resolver::Resolver;

static API_PLACEHOLDE_REGEX: OnceLock<Regex> = OnceLock::new();

pub struct Admin {
    // @NOTE: controller
    resolver: Arc<Resolver>,
    api: Arc<ApiEngine>,

    // @NOTE: caching
    cache_unencrypted_tokens: Arc<LruCache<(i64, String), Option<String>, 32>>,
    cache_api_info_by_name: Arc<LruCache<String, Option<Api>, 32>>,
    cache_api_info_by_id: Arc<LruCache<i64, Option<Api>, 32>>,
    cache_connections: Arc<LruCache<i64, DatabaseConnection, 32>>,
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ColumnType {
    Unknown,
    Int32,
    Int64,
    Text,
}

impl From<i32> for ColumnType {
    fn from(value: i32) -> Self {
        match value {
            1 => ColumnType::Int32,
            2 => ColumnType::Int64,
            3 => ColumnType::Text,
            _ => ColumnType::Unknown,
        }
    }
}

impl From<String> for ColumnType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "i32" => ColumnType::Int32,
            "i64" => ColumnType::Int64,
            "string" => ColumnType::Text,
            _ => ColumnType::Unknown,
        }
    }
}

impl Display for ColumnType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ColumnType::Unknown => write!(f, "unknown"),
            ColumnType::Int32 => write!(f, "i32"),
            ColumnType::Int64 => write!(f, "i64"),
            ColumnType::Text => write!(f, "string"),
        }
    }
}

impl<'de> Deserialize<'de> for ColumnType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ColumnType::from(s))
    }
}

impl serde::Serialize for ColumnType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ColumnDescription {
    pub name: Option<String>,
    pub kind: Option<ColumnType>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum BackendType {
    Unknown,
    Rdbms,
    Duckdb,
}

impl From<i32> for BackendType {
    fn from(value: i32) -> Self {
        match value {
            1 => BackendType::Rdbms,
            2 => BackendType::Duckdb,
            _ => BackendType::Unknown,
        }
    }
}

impl From<String> for BackendType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "postgres" => BackendType::Rdbms,
            "duckdb" => BackendType::Duckdb,
            _ => BackendType::Unknown,
        }
    }
}

impl Display for BackendType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            BackendType::Unknown => write!(f, "unknown"),
            BackendType::Rdbms => write!(f, "rdbms"),
            BackendType::Duckdb => write!(f, "duckdb"),
        }
    }
}

impl<'de> Deserialize<'de> for BackendType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(BackendType::from(s))
    }
}

impl serde::Serialize for BackendType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Table {
    pub id: Option<i64>,
    pub table: Option<String>,
    pub backend: Option<BackendType>,
    pub columns: Option<Vec<ColumnDescription>>,
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
            cache_connections: Arc::new(LruCache::new(10 * 32)),
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
    pub async fn insert_or_update_sites(
        &self,
        tenant_id: i64,
        sites: Vec<Site>,
    ) -> Result<(), DbErr> {
        sitemap::Entity::insert_many(
            sites
                .iter()
                .map(|site| sitemap::ActiveModel {
                    tenant_id: Set(tenant_id),
                    loc: Set(site.loc.clone()),
                    freq: Set(site.freq.clone()),
                    priority: Set(site.priority),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        )
        .on_conflict(
            OnConflict::column(sitemap::Column::Loc)
                .update_columns([
                    sitemap::Column::Freq,
                    sitemap::Column::Priority,
                    sitemap::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(self.dbt(tenant_id))
        .await?;

        Ok(())
    }

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

    pub async fn insert_or_update_acticles(
        &self,
        tenant_id: i64,
        articles: Vec<Article>,
    ) -> Result<(), DbErr> {
        article_map::Entity::insert_many(
            articles
                .into_iter()
                .map(|article| article_map::ActiveModel {
                    tenant_id: Set(tenant_id),
                    loc: Set(article.loc),
                    title: Set(article.title),
                    name: Set(article.name),
                    language: Set(article.language),
                    keywords: Set(article.keywords),
                    ..Default::default() // id tự sinh
                })
                .collect::<Vec<_>>(),
        )
        .on_conflict(
            OnConflict::column(article_map::Column::Loc)
                .update_columns([
                    article_map::Column::Title,
                    article_map::Column::Name,
                    article_map::Column::Language,
                    article_map::Column::Keywords,
                    article_map::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(self.dbt(tenant_id))
        .await?;
        Ok(())
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
                let key = Key::<Aes256Gcm>::from_slice(master_key_str.as_slice());
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

                self.cache_unencrypted_tokens
                    .put(cache_key_after_done, Some(token.clone()));
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
        let txn = self.dbt(tenant_id).begin().await?;
        self.put_unencrypted_token_txn(&txn, tenant_id, service_name, token_plain)
            .await?;
        txn.commit().await?;

        self.cache_unencrypted_tokens
            .put((tenant_id, service_name.clone()), Some(token_plain.clone()));
        Ok(())
    }

    async fn put_unencrypted_token_txn(
        &self,
        txn: &DatabaseTransaction,
        tenant_id: i64,
        service_name: &String,
        token_plain: &String,
    ) -> Result<(), DbErr> {
        let master_key_str = self.get_master_key().await?;

        let key = Key::<Aes256Gcm>::from_slice(master_key_str.as_slice());
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
        .exec(txn)
        .await?;
        Ok(())
    }

    pub async fn list_supported_services(&self, tenant_id: i64) -> Result<Vec<String>, DbErr> {
        TokenMap::find()
            .filter(token_map::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(token_map::Column::Service)
            .into_tuple::<String>()
            .all(self.dbt(tenant_id))
            .await
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

                        self.cache_api_info_by_name
                            .put(cache_key_after_done, Some(api_info.clone()));
                        Ok(api_info)
                    }
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
                    }) {
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
        let re = API_PLACEHOLDE_REGEX.get_or_init(|| Regex::new(r"(?P<key>[^?=&]+)=\{\}").unwrap());

        for (i, schema) in schemas.iter().enumerate() {
            let url = schema
                .url
                .clone()
                .ok_or_else(|| DbErr::Custom(format!("`url` is missing in schema {}", i,)))?;

            active_models.push(api_map::ActiveModel {
                tenant_id: Set(tenant_id),
                name: Set(schema
                    .name
                    .clone()
                    .ok_or_else(|| DbErr::Custom(format!("`name` is missing in schema {}", i)))?),
                url: Set(if let Some((base, query)) = url.split_once('?') {
                    let mut keys = re
                        .captures_iter(query)
                        .map(|cap| cap["key"].to_string())
                        .collect::<Vec<_>>();

                    keys.sort();

                    format!(
                        "{base}?{}",
                        keys.iter()
                            .map(|key| format!("{key}={{}}"))
                            .collect::<Vec<_>>()
                            .join("&")
                    )
                } else {
                    url
                }),
                mode: Set(schema
                    .mode
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
        paths: Vec<String>,
        args: Vec<String>,
        headers: HashMap<String, String>,
        body: Option<JsonValue>,
    ) -> Result<Vec<JsonValue>, DbErr> {
        self.perform_api_by_api_info(
            &self.get_api_schema_by_id(tenant_id, query_id).await?,
            paths,
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
        body: Option<JsonValue>,
    ) -> Result<Vec<JsonValue>, DbErr> {
        self.perform_api_by_api_info(
            &self.get_api_schema_by_name(tenant_id, name, mode).await?,
            vec![],
            args,
            headers,
            body,
        )
        .await
    }

    async fn perform_api_by_api_info(
        &self,
        api_info: &Api,
        paths: Vec<String>,
        args: Vec<String>,
        headers: HashMap<String, String>,
        body: Option<JsonValue>,
    ) -> Result<Vec<JsonValue>, DbErr> {
        let mut url = api_info.url.clone().ok_or_else(|| {
            DbErr::Query(RuntimeErr::Internal("Api is broken, missing `url`".into()))
        })?;

        let api_type = api_info.mode.ok_or_else(|| {
            DbErr::Query(RuntimeErr::Internal("Api is broken, missing `mode`".into()))
        })?;

        let parser = api_info.parser.as_ref().ok_or_else(|| {
            DbErr::Query(RuntimeErr::Internal(
                "Api is broken, missing `template`".into(),
            ))
        })?;

        for (i, path) in paths.iter().enumerate() {
            url = url.replacen(format!(":{i}").as_str(), path, 1);
        }

        for arg in args {
            url = url.replacen("{}", &arg, 1);
        }

        if body.is_none() && matches!(api_type, ApiType::Create | ApiType::Update) {
            return Err(DbErr::Query(RuntimeErr::Internal(format!(
                "No body provided for create or update API type: {api_type}",
            ))));
        }

        let query_parser = Arc::new(algorithm::JsonQuery::new(parser.clone()));

        match api_type {
            ApiType::Create => self
                .api
                .create(url.as_str(), &query_parser, &headers, body.unwrap())
                .await
                .map_err(|e| DbErr::Query(RuntimeErr::Internal(format!("Error creating: {e}")))),

            ApiType::Read => self
                .api
                .read(url.as_str(), &query_parser, &headers)
                .await
                .map_err(|e| DbErr::Query(RuntimeErr::Internal(format!("Error reading: {e}")))),

            ApiType::Update => self
                .api
                .update(url.as_str(), &query_parser, &headers, body.unwrap())
                .await
                .map_err(|e| DbErr::Query(RuntimeErr::Internal(format!("Error updating: {e}")))),

            ApiType::Delete => self
                .api
                .delete(url.as_str(), &query_parser, &headers)
                .await
                .map_err(|e| DbErr::Query(RuntimeErr::Internal(format!("Error deleting: {e}")))),

            _ => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Unknown API type: {api_type}"
            )))),
        }
    }

    // --------------------------------------------------------------
    pub async fn list_paginated_table_schema(
        &self,
        tenant_id: i64,
        after: i64,
        limit: u64,
    ) -> Result<Vec<Table>, DbErr> {
        Ok(TableMap::find()
            .filter(table_map::Column::TenantId.eq(tenant_id))
            .filter(table_map::Column::Id.gt(after))
            .select_only()
            .column(table_map::Column::Id)
            .column(table_map::Column::Name)
            .column(table_map::Column::Backend)
            .column(table_map::Column::Schema)
            .limit(limit)
            .into_tuple::<(i64, String, i32, table_map::Schema)>()
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|(id, table, backend, schema)| Table {
                id: Some(id),
                backend: Some(BackendType::from(backend)),
                table: Some(table.clone()),
                columns: Some(schema.0.columns.clone()),
            })
            .collect())
    }

    pub async fn is_database_connection_setup(&self, tenant_id: i64) -> Result<bool, DbErr> {
        let result = DatabaseMap::find()
            .filter(database_map::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(database_map::Column::Token)
            .one(self.dbt(tenant_id))
            .await?;
        Ok(result.is_some())
    }

    pub async fn setup_database_connection(
        &self,
        tenant_id: i64,
        token: String,
        dsn: String,
    ) -> Result<(), DbErr> {
        let txn = self.dbt(tenant_id).begin().await?;

        self.put_unencrypted_token_txn(&txn, tenant_id, &dsn, &token)
            .await?;

        DatabaseMap::insert(database_map::ActiveModel {
            tenant_id: Set(tenant_id),
            token: Set(token.clone()),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

        txn.commit().await?;
        self.cache_unencrypted_tokens
            .put((tenant_id, token.clone()), Some(dsn.clone()));
        Ok(())
    }

    pub async fn create_table_schemas(
        &self,
        tenant_id: i64,
        tables: Vec<Table>,
    ) -> Result<Vec<Table>, DbErr> {
        let mut active_models = Vec::new();

        for table in tables.iter() {
            if let Some(columns) = &table.columns {
                for (i, column) in columns.iter().enumerate() {
                    if i == 0 && column.kind != Some(ColumnType::Int64) {}
                }
            }

            active_models.push(table_map::ActiveModel {
                tenant_id: Set(tenant_id),
                name: Set(table.table.clone().unwrap()),
                backend: Set(table.backend.unwrap_or(BackendType::Unknown) as i32),
                schema: Set(table_map::Schema(table_map::SchemaDesciption {
                    columns: table.columns.clone().unwrap(),
                })),
                ..Default::default()
            });
        }

        TableMap::insert_many(active_models)
            .exec(self.dbt(tenant_id))
            .await?;

        Ok(TableMap::find()
            .filter(table_map::Column::TenantId.eq(tenant_id))
            .filter(
                table_map::Column::Name
                    .is_in(tables.iter().map(|t| t.table.clone()).collect::<Vec<_>>()),
            )
            .all(self.dbt(tenant_id))
            .await?
            .into_iter()
            .map(|m| Table {
                id: Some(m.id),
                backend: Some(BackendType::from(m.backend)),
                table: Some(m.name.clone()),
                columns: Some(m.schema.0.columns.clone()),
            })
            .collect::<Vec<_>>())
    }

    pub async fn get_connection_by_id(&self, tenant_id: i64) -> Result<DatabaseConnection, DbErr> {
        if let Some(conn) = self.cache_connections.get(&tenant_id) {
            return Ok(conn);
        }

        match DatabaseMap::find()
            .select_only()
            .column(database_map::Column::Token)
            .filter(database_map::Column::TenantId.eq(tenant_id))
            .into_tuple::<String>()
            .one(self.dbt(tenant_id))
            .await?
        {
            Some(token) => {
                let dsn = self.get_unencrypted_token(tenant_id, &token).await?;
                let mut opt = ConnectOptions::new(dsn.to_string());

                opt.max_connections(100)
                    .min_connections(5)
                    .connect_timeout(Duration::from_secs(8))
                    .idle_timeout(Duration::from_secs(8))
                    .max_lifetime(Duration::from_secs(8))
                    .sqlx_logging(true);

                let conn = Database::connect(opt).await?;

                self.cache_connections.put(tenant_id, conn.clone());
                Ok(conn)
            }
            None => Err(DbErr::Custom(format!(
                "No token found for tenant {}",
                tenant_id,
            ))),
        }
    }

    pub async fn get_table_info_by_id(
        &self,
        tenant_id: i64,
        table_id: i64,
    ) -> Result<Table, DbErr> {
        match TableMap::find()
            .select_only()
            .column(table_map::Column::Id)
            .column(table_map::Column::Name)
            .column(table_map::Column::Backend)
            .column(table_map::Column::Schema)
            .filter(table_map::Column::TenantId.eq(tenant_id))
            .filter(table_map::Column::Id.eq(table_id))
            .into_tuple::<(i64, String, i32, table_map::Schema)>()
            .one(self.dbt(tenant_id))
            .await?
            .map(|(id, table, backend, schema)| Table {
                id: Some(id),
                backend: Some(BackendType::from(backend)),
                table: Some(table.clone()),
                columns: Some(schema.0.columns.clone()),
            }) {
            Some(table) => Ok(table),
            None => Err(DbErr::Query(RuntimeErr::Internal(format!(
                "Not found table for tenant_id {} and id {}",
                tenant_id, table_id,
            )))),
        }
    }

    pub async fn read_from_table_by_id(
        &self,
        tenant_id: i64,
        table_id: i64,
        after: i64,
        limit: u64,
    ) -> Result<HashMap<String, Vec<JsonValue>>, DbErr> {
        let table_info = self.get_table_info_by_id(tenant_id, table_id).await?;

        match table_info.backend {
            Some(BackendType::Rdbms) => {
                self.read_from_rdbms_table(tenant_id, &table_info, after, limit)
                    .await
            }
            Some(backend) => Err(DbErr::Custom(format!("Not support `{backend}`"))),
            None => Err(DbErr::Custom("field `backend` is required".to_string())),
        }
    }

    pub async fn write_to_table_by_id(
        &self,
        tenant_id: i64,
        table_id: i64,
        body: Option<JsonValue>,
    ) -> Result<usize, DbErr> {
        let table_info = self.get_table_info_by_id(tenant_id, table_id).await?;

        match table_info.backend {
            Some(BackendType::Rdbms) => {
                self.write_to_rdbms_table(tenant_id, &table_info, body)
                    .await
            }
            Some(backend) => Err(DbErr::Custom(format!("Not support `{backend}`"))),
            None => Err(DbErr::Custom("field `backend` is required".to_string())),
        }
    }

    pub async fn write_to_rdbms_table(
        &self,
        tenant_id: i64,
        table_info: &Table,
        body: Option<JsonValue>,
    ) -> Result<usize, DbErr> {
        let mut stmt = Query::insert();
        let mut columns_to_insert = Vec::new();
        let mut values_to_insert = Vec::new();

        let body = match body {
            Some(JsonValue::Object(map)) => map,
            _ => return Err(DbErr::Custom("Body must be a JSON object".into())),
        };
        let table_name = table_info
            .table
            .as_deref()
            .ok_or_else(|| DbErr::Custom("Table name is required".into()))?;
        let columns_schema = table_info
            .columns
            .as_ref()
            .ok_or_else(|| DbErr::Custom("Table schema not found".into()))?;

        stmt.into_table(Alias::new(table_name));

        for col in columns_schema {
            let col_name = match &col.name {
                Some(name) => name,
                None => continue,
            };

            if let Some(value) = body.get(col_name) {
                match (col.kind.unwrap_or(ColumnType::Unknown), value) {
                    (ColumnType::Int32 | ColumnType::Int64, JsonValue::Number(n)) if n.is_i64() => {
                    }
                    (ColumnType::Text, JsonValue::String(_)) => {}

                    (kind, _) => {
                        return Err(DbErr::Custom(format!(
                            "Column '{}' (type {:?}) only accepts plain numbers or strings",
                            col_name, kind
                        )));
                    }
                }

                columns_to_insert.push(Alias::new(col_name));
                values_to_insert.push(
                    match value {
                        JsonValue::String(s) => OrmValue::String(Some(s.clone())),
                        JsonValue::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                OrmValue::BigInt(Some(i))
                            } else if let Some(f) = n.as_f64() {
                                OrmValue::Double(Some(f))
                            } else {
                                OrmValue::BigInt(None)
                            }
                        }
                        _ => OrmValue::String(Some(value.to_string())),
                    }
                    .into(),
                );
            }
        }

        if columns_to_insert.is_empty() {
            return Err(DbErr::Custom("No valid columns to insert".into()));
        }

        stmt.columns(columns_to_insert)
            .values_panic(values_to_insert);

        let result = self
            .get_connection_by_id(tenant_id)
            .await?
            .execute(&stmt)
            .await?;

        Ok(result.rows_affected() as usize)
    }

    pub async fn read_from_rdbms_table(
        &self,
        tenant_id: i64,
        table_info: &Table,
        after: i64,
        limit: u64,
    ) -> Result<HashMap<String, Vec<JsonValue>>, DbErr> {
        let mut result = HashMap::new();
        let mut stmt = Query::select();

        let table_name = match &table_info.table {
            Some(name) if !name.trim().is_empty() => name.trim(),
            _ => return Err(DbErr::Custom("Table name is required".into())),
        };
        let columns = match &table_info.columns {
            Some(cols) if !cols.is_empty() => cols,
            _ => return Err(DbErr::Custom("No columns provided".into())),
        };
        let pkey = columns
            .first()
            .and_then(|c| c.name.as_deref())
            .unwrap_or("id");

        for col in columns.iter().filter_map(|c| c.name.as_deref()) {
            stmt.column(Alias::new(col));
        }

        stmt.from(Alias::new(table_name))
            .cond_where(Condition::all().add(Expr::col(Alias::new(pkey)).gt(Expr::val(after))))
            .order_by(Alias::new(pkey), sea_orm::Order::Asc)
            .limit(limit);

        let rows = self
            .get_connection_by_id(tenant_id)
            .await?
            .query_all(&stmt)
            .await?;

        let col_names = columns
            .iter()
            .filter_map(|c| c.name.clone())
            .collect::<Vec<_>>();

        for col in &col_names {
            result.insert(col.clone(), Vec::with_capacity(rows.len()));
        }

        for row in rows {
            for col_name in &col_names {
                let val = row.try_get("", col_name).unwrap_or(JsonValue::Null);

                if let Some(vec) = result.get_mut(col_name) {
                    vec.push(val);
                }
            }
        }

        Ok(result)
    }
}
