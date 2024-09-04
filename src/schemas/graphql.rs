use std::sync::Arc;
use actix::Addr;
use juniper::{
    graphql_object, 
    GraphQLObject,
    FieldResult,
};

use diesel::prelude::*;
use crate::helpers::{PgConn, PgPool};
use crate::actors::redis::RedisActor;
use crate::actors::vps::{VpsActor, UpdateStocksCommand};
use crate::actors::dnse::{DnseActor, GetOHCLCommand, CandleStick};

#[derive(Clone)]
pub struct Context {
    pub vps:   Arc<Addr<VpsActor>>,
    pub dnse:  Arc<Addr<DnseActor>>,
    pub pool:  Arc<PgPool>,
    pub cache: Arc<Addr<RedisActor>>,
}

impl juniper::Context for Context {}

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    async fn ohcl(
        ctx:        &Context, 
        resolution: String,
        stock:      String,
        from:       i32,
        to:         i32,
    ) -> FieldResult<Vec<CandleStick>> {
        // @NOTE: cache OHCL to redis and reuse it later if needs
        let res = ctx.dnse.send(GetOHCLCommand{
                resolution: resolution,
                stock:      stock,
                from:       from,
                to:         to,
            })
            .await
            .unwrap();

        match res {
            Ok(res) => Ok(res),
            Err(error) => Ok(Vec::<CandleStick>::new()),
        }
    }
}

pub struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    async fn watch(ctx: &Context, stocks: Vec<String>) -> FieldResult<Vec<String>> {
        let ok = ctx.vps.send(UpdateStocksCommand{ stocks: stocks.clone() })
            .await
            .unwrap();

        if ok {
            Ok(stocks.clone())
        } else {
            Err("cannot update list stocks watching".into())
        }
    }
}

