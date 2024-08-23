use std::sync::Arc;
use actix::prelude::*;
use juniper::{RootNode, EmptySubscription};

use diesel::r2d2::{Pool, PooledConnection, ConnectionManager};
use diesel::pg::PgConnection;

use crate::schemas::graphql::{Query, Mutation, Context};
use crate::actors::vps::VpsActor;
use crate::actors::dnse::DnseActor;
use crate::actors::redis::RedisActor;

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
    SchemaGraphQL::new(
        Query {}, 
        Mutation {},
        EmptySubscription::new(),
    )
}

pub fn create_graphql_context(
    vps:   Arc<Addr<VpsActor>>,
    dnse:  Arc<Addr<DnseActor>>,
    pool:  Arc<PgPool>, 
    cache: Arc<Addr<RedisActor>>,
) -> Context {
    Context {
        vps:   vps,
        dnse:  dnse,
        pool:  pool,
        cache: cache,
    }
}
