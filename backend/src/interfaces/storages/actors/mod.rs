#![deny(clippy::str_to_string)]

use actix::Addr;
use async_trait::async_trait;
use log::{info, error, debug};
use std::sync::Arc;
use futures::stream::iter;

use gluesql::core::ast::{Expr, AstLiteral};
use gluesql::core::store::{AlterTable, Index, IndexMut, Metadata, StoreMut, Transaction};
use gluesql::core::store::{CustomFunction, CustomFunctionMut};
use gluesql::core::data::{CustomFunction as StructCustomFunction, Key, Schema};
use gluesql::core::error::{Error, Result};
use gluesql::core::store::{DataRow, RowIter, Store};

use crate::actors::{FetchDataCommand, ListSchemaCommand, SaveDataCommand, ScanDataCommand};
use crate::actors::dnse::DnseActor;
use crate::actors::fireant::FireantActor;
use crate::actors::tcbs::TcbsActor;
use crate::actors::vps::VpsActor;
use crate::actors::lru::{LruActor, connect_to_lru};
use crate::algorithm::binarysearch::binary_search;

pub struct PgServerDatasource {
    // @NOTE: datasource
    vps: Arc<Addr<VpsActor>>,
    dnse: Arc<Addr<DnseActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    fireant: Arc<Addr<FireantActor>>,

    // @NOTE: caching
    cache_data: Arc<Addr<LruActor>>,

    // @NOTE: function version
    version: StructCustomFunction,
}

impl PgServerDatasource {
    pub fn new(
        capacity: usize,
        vps: Arc<Addr<VpsActor>>,
        dnse: Arc<Addr<DnseActor>>,
        tcbs: Arc<Addr<TcbsActor>>,
        fireant: Arc<Addr<FireantActor>>,
    ) -> Self {
        PgServerDatasource {
            vps,
            dnse,
            tcbs,
            fireant,

            // @NOTE: caching
            cache_data: Arc::new(connect_to_lru(capacity)),

            // @NOTE: function version
            version: StructCustomFunction{ 
                func_name: "VERSION".to_string(), 
                args: Vec::new(), 
                body: Expr::Literal(AstLiteral::QuotedString("Algorithm".to_string())),
            },
        }
    }
}

impl AlterTable for PgServerDatasource {}

impl Index for PgServerDatasource {}
impl IndexMut for PgServerDatasource {}

impl Transaction for PgServerDatasource {}
impl Metadata for PgServerDatasource {}

#[async_trait(?Send)]
impl Store for PgServerDatasource {
    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        match self.fetch_all_schemas().await {
            Ok(schemas) => {
                for schema in schemas {
                    if schema.table_name == table_name {
                        return Ok(Some(schema));
                    }
                }

                Ok(None)
            }

            Err(error) => Err(error),
        }
    }

    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {
        let dnse_schemas = self.dnse.send(ListSchemaCommand).await.unwrap();
        let tcbs_schemas = self.tcbs.send(ListSchemaCommand).await.unwrap();
        let fireant_schemas = self.fireant.send(ListSchemaCommand).await.unwrap();

        Ok(dnse_schemas
            .into_iter()
            .chain(tcbs_schemas)
            .chain(fireant_schemas)
            .collect())
    }

    async fn fetch_data(&self, table_name: &str, target: &Key) -> Result<Option<DataRow>> {
        let datasources = vec!["dnse"];

        for datasource in datasources {
            match self.cache_data.send(FetchDataCommand {
                        target:    (*target).clone(),
                        table:     table_name.to_string(),
                        namespace: datasource.to_string(),
                    })
                    .await 
                    .unwrap() {
                Some(result) => {
                    return Ok(Some(result));
                },

                None => {},
            };

            match self.dnse.send(FetchDataCommand {
                        target:    (*target).clone(),
                        table:     table_name.to_string(),
                        namespace: datasource.to_string(),
                    })
                    .await 
                    .unwrap() {
                Some(result) => {
                    return Ok(Some(result));
                },

                None => {},
            };
        }

        // @NOTE: fails just in case we don't see any approviated table
        return Err(Error::StorageMsg(format!("not found table {}, target {:?}", table_name, target)));
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        let table_name_in_string = table_name.to_string();

        // @NOTE: fetch from Lru cache
        let list_dnse_schemas = self.dnse.send(ListSchemaCommand)
            .await
            .unwrap();

        match binary_search(
            &list_dnse_schemas, 
            &table_name_in_string, 
            |target: &String, object: &Schema| {
                target.cmp(&object.table_name)
            }
        ) {
            Some(index) => {
                let table = list_dnse_schemas[index].table_name.clone();
                let mut sorted_data = self.cache_data.send(ScanDataCommand{
                    namespace: "dnse".to_string(),
                    table: table.clone(),
                })
                .await
                .unwrap();

                if sorted_data.len() == 0 {
                    sorted_data = self.dnse.send(ScanDataCommand{
                            namespace: "dnse".to_string(),
                            table: table.clone(),
                        })
                        .await
                        .unwrap();

                    match self.cache_data.send(SaveDataCommand{
                                namespace: "dnse".to_string(),
                                table,
                                targets: sorted_data.clone()
                            })
                            .await
                            .unwrap() {
                        Some(cnt) => if cnt as usize == sorted_data.len() {
                            debug!("number of updated records to LRU cache is {}", cnt);
                        } else {
                            error!("LRU cache is full and cannot update any more")
                        },
                        None => {
                            error!("Seem to be cannot update records to LRU cache");
                        },
                    }
                } else {
                    debug!("number of records in LRU cache is {}", sorted_data.len());
                }

                let ret = sorted_data
                    .into_iter()
                    .map(Ok); 
                return Ok(Box::pin(iter(ret)));
            },
            None => {}
        }

        let list_fireant_schemas = self.fireant.send(ListSchemaCommand)
            .await
            .unwrap();

        match binary_search(
            &list_fireant_schemas, 
            &table_name_in_string, 
            |target: &String, object: &Schema| {
                target.cmp(&object.table_name)
            },
        ) {
            Some(index) => {
                let table = list_fireant_schemas[index].table_name.clone();
                let mut sorted_data = self.cache_data.send(ScanDataCommand{
                    namespace: "fireant".to_string(),
                    table: table.clone(),
                })
                .await
                .unwrap();

                if sorted_data.len() == 0 {
                    sorted_data = self.fireant.send(ScanDataCommand{
                            namespace: "fireant".to_string(),
                            table: table.clone(),
                        })
                        .await
                        .unwrap();

                    match self.cache_data.send(SaveDataCommand{
                                namespace: "fireant".to_string(),
                                table,
                                targets: sorted_data.clone()
                            })
                            .await
                            .unwrap() {
                        Some(cnt) => if cnt as usize == sorted_data.len() {
                            debug!("number of updated records to LRU cache is {}", cnt);
                        } else {
                            error!("LRU cache is full and cannot update any more")
                        },
                        None => {
                            error!("Seem to be cannot update records to LRU cache");
                        },
                    }
                } else {
                    debug!("number of records in LRU cache is {}", sorted_data.len());
                }

                let ret = sorted_data
                    .into_iter()
                    .map(Ok); 
                return Ok(Box::pin(iter(ret)));
            },
            None => {}
        }
        return Err(Error::StorageMsg(format!("not found table {}", table_name)));
    }
}

impl StoreMut for PgServerDatasource {}

#[async_trait(?Send)]
impl CustomFunction for PgServerDatasource {
    async fn fetch_function(&self, func_name: &str) -> Result<Option<&StructCustomFunction>> {
        match func_name {
            "VERSION" => Ok(Some(&self.version)),
            _ => {
                Err(Error::StorageMsg("[Storage] CustomFunction is not supported".to_owned()))
            }
        }
    }

    async fn fetch_all_functions(&self) -> Result<Vec<&StructCustomFunction>> {
        Err(Error::StorageMsg(
            "[Storage] CustomFunction is not supported".to_owned(),
        ))
    }
}

impl CustomFunctionMut for PgServerDatasource {}
