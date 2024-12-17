use actix::prelude::*;
use gluesql::core::data::{Key, Schema};
use gluesql::core::store::DataRow;

use std::collections::BTreeMap;

pub mod lru;
pub mod cron;
pub mod dnse;
pub mod fireant;
pub mod process;
pub mod prometheus;
pub mod redis;
pub mod simulator;
pub mod tcbs;
pub mod vps;
pub mod vietcap;

#[derive(Message, Debug)]
#[rtype(result = "bool")]
pub struct HealthCommand;

#[derive(Message, Debug)]
#[rtype(result = "Vec<Schema>")]
pub struct ListSchemaCommand;

#[derive(Message, Debug)]
#[rtype(result = "BTreeMap<Key, DataRow>")]
pub struct ScanDataCommand {
    pub table: String,
}

#[derive(Message, Debug)]
#[rtype(result = "Option<DataRow>")]
pub struct FetchDataCommand {
    pub namespace: String,
    pub table: String,
    pub target: Key,
}

#[derive(Message, Debug)]
#[rtype(result = "Option<i64>")]
pub struct SaveDataCommand {
    pub namespace: String,
    pub table: String,
    pub targets: BTreeMap<Key, DataRow>,
}