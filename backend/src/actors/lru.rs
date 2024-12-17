use std::collections::{BTreeMap, HashMap, HashSet};

use gluesql::core::store::DataRow;
use gluesql::prelude::{Error, Key};

use actix::prelude::*;
use actix::Addr;

use crate::algorithm::lru::LruCache;

struct Namespace {
    tables: HashMap<String, HashSet<Key>>
}

impl Namespace {
    fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }
}

pub struct LruActor {
    data_row_cache: LruCache<String, DataRow>,
    namespaces: HashMap<String, Namespace>,
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
            namespaces: HashMap::new(),
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

        if let Ok(key_name) = lru_cache_generate_key(namespace.as_str(), table.as_str(), &target) {
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

impl Handler<super::ScanDataCommand> for LruActor {
    type Result = ResponseFuture<BTreeMap<Key, DataRow>>;

    fn handle(&mut self, msg: super::ScanDataCommand, _: &mut Self::Context) -> Self::Result {
        let namespace_in_str = msg.namespace.clone();
        let table_in_str = msg.table.clone();

        if let Some(namespace) = self.namespaces.get_mut(&namespace_in_str) {
            if let Some(table_for_update) = namespace.tables.get_mut(&table_in_str) {
                let mut list_key_to_remove = Vec::new();
                let mut ret = BTreeMap::<Key, DataRow>::new();
                let table_for_scan = table_for_update.clone();

                for key in table_for_scan.into_iter() {
                    if let Ok(keyname) = lru_cache_generate_key(namespace_in_str.as_str(), table_in_str.as_str(), &key) {
                        match self.data_row_cache.get(&keyname) {
                            Some(row) => { ret.insert(key.clone(), row.clone()); },
                            None => { list_key_to_remove.push(key.clone()); },
                        }
                    }
                }

                for key_to_remove in list_key_to_remove {
                    table_for_update.remove(&key_to_remove);
                }

                return Box::pin(async move { ret });
            }
        }

        return Box::pin(async move { BTreeMap::<Key, DataRow>::new() });
    }
}

impl Handler<super::SaveDataCommand> for LruActor {
    type Result = ResponseFuture<Option<i64>>;

    fn handle(&mut self, msg: super::SaveDataCommand, _: &mut Self::Context) -> Self::Result {
        let mut cnt = 0;
        let cache = &mut self.data_row_cache;

        let namespace_str = msg.namespace
            .as_str();
        let namespace = self.namespaces
            .entry(msg.namespace.clone())
            .or_insert_with(Namespace::new);

        let table_str = msg.table
            .as_str();
        let table = namespace.tables
            .entry(msg.table.clone())
            .or_insert_with(HashSet::new);

        let list_status = msg.targets
            .into_iter()
            .map(|(key, row)| {
                if let Ok(name) = lru_cache_generate_key(namespace_str, table_str, &key) {
                    table.insert(key.clone());
                    cache.put(name, row)
                } else {
                    false
                }
            })
            .collect::<Vec<bool>>();

        for status in list_status {
            if status {
                cnt += 1;
            }
        }

        Box::pin(async move { Some(cnt) })
    }
}

pub fn connect_to_lru(capacity: usize) -> Addr<LruActor> {
    LruActor::new(capacity).start()
}
