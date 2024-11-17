use actix::Addr;
use chrono::{NaiveDateTime, Utc};
use juniper::{graphql_object, FieldResult, GraphQLInputObject};

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::actors::cron::{CronActor, PerformCommand};
use crate::actors::dnse::{CandleStick, DnseActor, GetOHCLCommand};
use crate::actors::fireant::FireantActor;
use crate::actors::redis::RedisActor;
use crate::actors::tcbs::TcbsActor;
use crate::actors::vps::{UpdateStocksCommand, VpsActor};
use crate::helpers::PgPool;

#[derive(Clone)]
pub struct Context {
    pub cron: Arc<Addr<CronActor>>,
    pub vps: Arc<Addr<VpsActor>>,
    pub dnse: Arc<Addr<DnseActor>>,
    pub tcbs: Arc<Addr<TcbsActor>>,
    pub fireant: Arc<Addr<FireantActor>>,
    pub pool: Arc<PgPool>,
    pub cache: Arc<Addr<RedisActor>>,
}

impl juniper::Context for Context {}

pub struct Query;

#[derive(GraphQLInputObject)]
struct Pair {
    key: String,
    value: String,
}

#[graphql_object(context = Context)]
impl Query {
    async fn cron_perform(
        ctx: &Context,
        target: String,
        timeout: i32,
        arguments: Option<Vec<Pair>>,
        from: Option<i32>,
        to: Option<i32>,
    ) -> FieldResult<i32> {
        let mut mapping = BTreeMap::<String, String>::new();

        if let Some(arguments) = arguments {
            for pair in arguments {
                mapping.insert(pair.key, pair.value);
            }
        }

        Ok(ctx
            .cron
            .send(PerformCommand {
                target,
                timeout,
                mapping,
                from: from.unwrap_or(0),
                to: to.unwrap_or(0),
            })
            .await
            .unwrap()
            .unwrap() as i32)
    }

    async fn status(ctx: &Context) -> FieldResult<String> {
        Ok(Utc::now().to_string())
    }

    async fn ohcl(
        ctx: &Context,
        resolution: String,
        stock: String,
        from: f64,
        to: f64,
    ) -> FieldResult<Vec<CandleStick>> {
        // @NOTE: cache OHCL to redis and reuse it later if needs
        let res = ctx
            .dnse
            .send(GetOHCLCommand {
                resolution,
                stock,
                from: from as i64,
                to: to as i64,
            })
            .await
            .unwrap();

        match res {
            Ok(res) => Ok(res),
            Err(_) => Ok(Vec::<CandleStick>::new()),
        }
    }
}

pub struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    async fn watch(ctx: &Context, stocks: Vec<String>) -> FieldResult<Vec<String>> {
        let ok = ctx
            .vps
            .send(UpdateStocksCommand {
                stocks: stocks.clone(),
            })
            .await
            .unwrap();

        if ok {
            Ok(stocks.clone())
        } else {
            Err("cannot update list stocks watching".into())
        }
    }
}
