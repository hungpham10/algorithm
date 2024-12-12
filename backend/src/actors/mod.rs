use actix::prelude::*;
use gluesql::core::data::{Key, Schema};
use gluesql::core::store::DataRow;
use gluesql::core::error::Error;

use std::collections::BTreeMap;

pub mod cron;
pub mod dnse;
pub mod fireant;
pub mod process;
pub mod prometheus;
pub mod redis;
pub mod simulator;
pub mod tcbs;
pub mod vps;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

#[derive(Message, Debug)]
#[rtype(result = "Vec<Schema>")]
pub struct ListSchemaCommand;

#[derive(Message, Debug)]
#[rtype(result = "Option<DataRow>")]
pub struct FetchDataCommand {
    pub table: String,
    pub target: Key,
}

#[derive(Message, Debug)]
#[rtype(result = "BTreeMap<Key, DataRow>")]
pub struct ScanDataCommand {
    pub table: String,
}

fn lru_cache_generate_key(namespace: &str, table_name: &str, key: &Key) -> Result<String, Error> {
    match serde_json::to_string(key).map_err(|e| {
        Error::StorageMsg(format!(
            "[LruCache] failed to serialize key key:{:?}, error={}",
            key, e
        ))
    }) {
        Ok(k) => Ok(format!("{}#{}#{}", namespace, table_name, k)),
        Err(error) => Err(error),
    } 
}