use actix::Addr;
use pgwire::messages::data::DataRow;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::stream;

use gluesql::prelude::*;

use pgwire::api::auth::noop::NoopStartupHandler;
use pgwire::api::copy::NoopCopyHandler;
use pgwire::api::query::{PlaceholderExtendedQueryHandler, SimpleQueryHandler};
use pgwire::api::results::{DataRowEncoder, FieldFormat, FieldInfo, QueryResponse, Response, Tag};
use pgwire::api::{ClientInfo, PgWireHandlerFactory, Type};
use pgwire::error::{PgWireError, PgWireResult};

use crate::actors::cron::CronActor;
use crate::actors::dnse::DnseActor;
use crate::actors::fireant::FireantActor;
use crate::actors::tcbs::TcbsActor;
use crate::actors::vps::VpsActor;

use crate::interfaces::storages::actors::PgServerDatasource;

const SHOW_TABLE_COLUMN_TABLE_NAME: &str = "table_name";

pub struct PgServerHandler {
    glue: Arc<Mutex<Glue<PgServerDatasource>>>,
}

impl NoopStartupHandler for PgServerHandler {}

#[async_trait]
impl SimpleQueryHandler for PgServerHandler {
    async fn do_query<'a, C>(
        &self,
        _client: &mut C,
        query: &'a str,
    ) -> PgWireResult<Vec<Response<'a>>>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        let query = query.to_string().clone();
        let glue = self.glue.clone();

        println!("{}", query);
        tokio::task::spawn_blocking(move || { 
                let mut glue: std::sync::MutexGuard<'_, Glue<PgServerDatasource>> = glue.lock().unwrap();

                futures::executor::block_on(glue.execute(query.clone())) 
            })
            .await
            .map_err(|err| PgWireError::ApiError(Box::new(err)))
            .and_then(|response| {
                response
                .map_err(|err| PgWireError::ApiError(Box::new(err)))
                .and_then(|payloads| {
                payloads
                    .iter()
                    .map(|payload| match payload {
                        Payload::Select { labels, rows } => {
                            let fields = labels
                                .iter()
                                .map(|label| {
                                    FieldInfo::new(
                                        label.into(),
                                        None,
                                        None,
                                        Type::UNKNOWN,
                                        FieldFormat::Text,
                                    )
                                })
                                .collect::<Vec<_>>();
                            let fields = Arc::new(fields);

                            let mut results = Vec::with_capacity(rows.len());
                            for row in rows {
                                let mut encoder = DataRowEncoder::new(fields.clone());
                                for field in row.iter() {
                                    match field {
                                        Value::Bool(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::BOOL,
                                                FieldFormat::Text,
                                            )?,
                                        Value::I8(v) => encoder.encode_field_with_type_and_format(
                                            v,
                                            &Type::CHAR,
                                            FieldFormat::Text,
                                        )?,
                                        Value::I16(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::INT2,
                                                FieldFormat::Text,
                                            )?,
                                        Value::I32(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::INT4,
                                                FieldFormat::Text,
                                            )?,
                                        Value::I64(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::INT8,
                                                FieldFormat::Text,
                                            )?,
                                        Value::U8(v) => encoder.encode_field_with_type_and_format(
                                            &(*v as i8),
                                            &Type::CHAR,
                                            FieldFormat::Text,
                                        )?,
                                        Value::F64(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::FLOAT8,
                                                FieldFormat::Text,
                                            )?,
                                        Value::Str(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::VARCHAR,
                                                FieldFormat::Text,
                                            )?,
                                        Value::Bytea(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::BYTEA,
                                                FieldFormat::Text,
                                            )?,
                                        Value::Date(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::DATE,
                                                FieldFormat::Text,
                                            )?,
                                        Value::Time(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::TIME,
                                                FieldFormat::Text,
                                            )?,
                                        Value::Timestamp(v) => encoder
                                            .encode_field_with_type_and_format(
                                                v,
                                                &Type::TIMESTAMP,
                                                FieldFormat::Text,
                                            )?,
                                        _ => unimplemented!(),
                                    }
                                }
                                results.push(encoder.finish());
                            }

                            Ok(Response::Query(QueryResponse::new(
                                fields,
                                stream::iter(results.into_iter()),
                            )))
                        }
                        Payload::Insert(rows) => Ok(Response::Execution(
                            Tag::new("INSERT").with_oid(0).with_rows(*rows),
                        )),
                        Payload::Delete(rows) => {
                            Ok(Response::Execution(Tag::new("DELETE").with_rows(*rows)))
                        }
                        Payload::Update(rows) => {
                            Ok(Response::Execution(Tag::new("UPDATE").with_rows(*rows)))
                        }
                        Payload::Create => Ok(Response::Execution(Tag::new("CREATE TABLE"))),
                        Payload::AlterTable => Ok(Response::Execution(Tag::new("ALTER TABLE"))),
                        Payload::DropTable => Ok(Response::Execution(Tag::new("DROP TABLE"))),
                        Payload::CreateIndex => Ok(Response::Execution(Tag::new("CREATE INDEX"))),
                        Payload::DropIndex => Ok(Response::Execution(Tag::new("DROP INDEX"))),
                        Payload::ShowVariable(PayloadVariable::Tables(tables)) => {
                            let mut results = Vec::with_capacity(tables.len());
                            let fields = Arc::new(vec![
                                FieldInfo::new(
                                    SHOW_TABLE_COLUMN_TABLE_NAME.to_string(), 
                                    None, 
                                    None, 
                                    Type::UNKNOWN, 
                                    FieldFormat::Text,
                                ),
                            ]);

                            for item in tables {
                                let mut encoder = DataRowEncoder::new(fields.clone());
 
                                encoder.encode_field_with_type_and_format(
                                    item,
                                    &Type::VARCHAR,
                                    FieldFormat::Text,
                                ).unwrap();
                                results.push(encoder.finish());
                            }

                            Ok(Response::Query(QueryResponse::new(
                                fields,
                                stream::iter(results.into_iter()),
                            )))
                        },
                        _ => {
                             unimplemented!()
                        }
                    })
                    .collect::<Result<Vec<Response>, PgWireError>>()
                })
            })    
    }
}

pub struct PgServerFactory {
    handler: Arc<PgServerHandler>,
}

impl PgWireHandlerFactory for PgServerFactory {
    type StartupHandler = PgServerHandler;
    type SimpleQueryHandler = PgServerHandler;
    type ExtendedQueryHandler = PlaceholderExtendedQueryHandler;
    type CopyHandler = NoopCopyHandler;

    fn simple_query_handler(&self) -> Arc<Self::SimpleQueryHandler> {
        self.handler.clone()
    }

    fn extended_query_handler(&self) -> Arc<Self::ExtendedQueryHandler> {
        Arc::new(PlaceholderExtendedQueryHandler)
    }

    fn startup_handler(&self) -> Arc<Self::StartupHandler> {
        self.handler.clone()
    }

    fn copy_handler(&self) -> Arc<Self::CopyHandler> {
        Arc::new(NoopCopyHandler)
    }
}

pub fn create_sql_context(
    capacity: usize,
    cron: Arc<Addr<CronActor>>,
    vps: Arc<Addr<VpsActor>>,
    dnse: Arc<Addr<DnseActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    fireant: Arc<Addr<FireantActor>>,
) -> Arc<PgServerFactory> {
    Arc::new(PgServerFactory {
        handler: Arc::new(PgServerHandler {
            glue: Arc::new(
                Mutex::new(
                    Glue::new(
                        PgServerDatasource::new(capacity, vps, dnse, tcbs, fireant),
                    ),
                ),
            ),
        }),
    })
}
