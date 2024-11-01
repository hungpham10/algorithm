use actix::Addr;
use chrono::Utc;
use juniper::{graphql_object, FieldResult};
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

#[graphql_object(context = Context)]
impl Query {
    async fn cron_perform(ctx: &Context, target: String, timeout: i32) -> FieldResult<i32> {
        Ok(ctx.cron
            .send(PerformCommand { target, timeout })
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
        from: i32,
        to: i32,
    ) -> FieldResult<Vec<CandleStick>> {
        // @NOTE: cache OHCL to redis and reuse it later if needs
        let res = ctx
            .dnse
            .send(GetOHCLCommand {
                resolution,
                stock,
                from,
                to,
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
