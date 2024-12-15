use std::collections::BTreeMap;

use gluesql::core::store::DataRow;
use gluesql::prelude::{Error, Key};

use actix::prelude::*;
use actix::Addr;

use crate::algorithm::lru::LruCache;

pub struct LruActor {
    data_row_cache: LruCache<String, DataRow>,
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

impl LruActor {
    fn new(capacity: usize) -> Self {
        Self { 
            data_row_cache: LruCache::new(capacity),
        }
    }
}

impl Actor for LruActor {
    type Context = Context<Self>;
}

impl Handler<super::FetchDataCommand> for LruActor {
    type Result = ResponseFuture<Option<DataRow>>;

    fn handle(&mut self, msg: super::FetchDataCommand, _: &mut Self::Context) -> Self::Result {
        let namespace = msg.namespace.clone();
        let table = msg.table.clone();
        let target = msg.target.clone();
        let cache = &mut self.data_row_cache;

        if let Ok(key_name) = lru_cache_generate_key(namespace.as_str(), table.as_str(), &msg.target) {
            match cache.get(&key_name) {
                Some(result) => {
                    let result = result.clone();

                    Box::pin(async move { Some(result) })
                },
                None => Box::pin(async move { None }),
            }
        } else {
            Box::pin(async move { None })
        }
    }
}

impl Handler<super::SaveDataCommand> for LruActor {
    type Result = ResponseFuture<Option<i64>>;

    fn handle(&mut self, msg: super::SaveDataCommand, _: &mut Self::Context) -> Self::Result {
        // @TODO
        let namespace = msg.namespace;
        let table = msg.table;
        let cache = &mut self.data_row_cache;

        msg.targets
            .into_iter()
            .map(|(key, row)| {
                if let Ok(name) = lru_cache_generate_key(namespace.as_str(), table.as_str(), &key) {
                    cache.put(name, row);
                }
                true
            })
            .collect::<Vec<bool>>();
        Box::pin(async move { None })
    }
}

pub fn connect_to_lru(capacity: usize) -> Addr<LruActor> {
    LruActor::new(capacity).start()
}
