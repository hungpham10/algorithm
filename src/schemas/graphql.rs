use std::sync::Arc;
use actix::Addr;
use juniper::{
    graphql_object, 
    GraphQLObject,
    FieldResult,
};

use diesel::prelude::*;
use crate::helpers::{PgConn, PgPool};
use crate::actors::redis::{RedisActor, InfoCommand};

#[derive(Clone)]
pub struct Context {
    pub pool:  Arc<PgPool>,
    pub cache: Arc<Addr<RedisActor>>,
}

impl juniper::Context for Context {}

#[derive(GraphQLObject, Queryable, Clone)]
#[graphql(description = "user")]
pub struct User {
    id:       i32,
    username: String,
    password: String,
}

impl User {
    pub fn list(dbconn: &mut PgConn) -> QueryResult<Vec<User>> {
        use crate::schemas::database::tbl_users::dsl::*;

        tbl_users
            .limit(20)
            .load::<User>(dbconn)
    }
}

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    async fn users(ctx: &Context) -> FieldResult<Vec<User>> {
        let mut dbconn = ctx.pool.get()?;
        let results = User::list(&mut dbconn)?;
        Ok(results)
    }

    async fn info(ctx: &Context) -> FieldResult<String> {
        let res = ctx.cache.send(InfoCommand)
            .await
            .unwrap().unwrap().unwrap();
        Ok(res)
    }
}

pub struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    fn hello(ctx: &Context, _data: String) -> FieldResult<&str> {
        Ok("hello")
    }
}

