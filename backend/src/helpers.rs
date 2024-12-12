use actix::prelude::*;
use juniper::{EmptySubscription, RootNode};
use std::sync::Arc;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use crate::actors::cron::CronActor;
use crate::actors::dnse::DnseActor;
use crate::actors::fireant::FireantActor;
use crate::actors::redis::RedisActor;
use crate::actors::tcbs::TcbsActor;
use crate::actors::vps::VpsActor;
use crate::schemas::graphql::{Context, Mutation, Query};

pub type SchemaGraphQL = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;
pub type PgConnMgr = ConnectionManager<PgConnection>;
pub type PgConn = PooledConnection<ConnectionManager<PgConnection>>;
pub type PgPool = Pool<PgConnMgr>;

pub fn connect_to_postgres_pool(pg_dsn: String) -> PgPool {
    // @NOTE: establish connection pool with our database
    PgPool::builder()
        .max_size(2)
        .build(PgConnMgr::new(pg_dsn))
        .unwrap()
}

pub fn create_graphql_schema() -> SchemaGraphQL {
    SchemaGraphQL::new(Query {}, Mutation {}, EmptySubscription::new())
}
