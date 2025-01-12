use actix::Addr;
use chrono::{NaiveDate, Utc};
use juniper::{graphql_object, EmptySubscription, FieldResult, GraphQLInputObject, RootNode};

use sentry::capture_error;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::actors::cron::{CronActor, PerformCommand};
use crate::actors::dnse::{DnseActor, GetOHCLCommand};
use crate::actors::fireant::FireantActor;
use crate::actors::redis::RedisActor;
use crate::actors::tcbs::TcbsActor;
use crate::actors::vps::{UpdateStocksCommand, VpsActor};
use crate::helpers::PgPool;
use crate::schemas::CandleStick;

#[derive(GraphQLInputObject)]
pub struct Pair {
    key: String,
    value: String,
}

pub struct Query;

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
        from: String,
        to: String,
    ) -> FieldResult<Vec<CandleStick>> {
        let from_in_int = NaiveDate::parse_from_str(from.as_str(), "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let to_in_int = NaiveDate::parse_from_str(to.as_str(), "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        // @NOTE: cache OHCL to redis and reuse it later if needs
        let res = ctx
            .dnse
            .send(GetOHCLCommand {
                resolution,
                stock,
                from: from_in_int,
                to: to_in_int,
            })
            .await
            .unwrap();

        match res {
            Ok(res) => Ok(res),
            Err(error) => {
                capture_error(&error);

                Ok(Vec::<CandleStick>::new())
            }
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

#[derive(Clone)]
pub struct Context {
    cron: Arc<Addr<CronActor>>,
    vps: Arc<Addr<VpsActor>>,
    dnse: Arc<Addr<DnseActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    fireant: Arc<Addr<FireantActor>>,
    pool: Arc<PgPool>,
    cache: Arc<Addr<RedisActor>>,
}

impl juniper::Context for Context {}

pub fn create_graphql_context(
    cron: Arc<Addr<CronActor>>,
    vps: Arc<Addr<VpsActor>>,
    dnse: Arc<Addr<DnseActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    fireant: Arc<Addr<FireantActor>>,
    pool: Arc<PgPool>,
    cache: Arc<Addr<RedisActor>>,
) -> Context {
    Context {
        cron,
        vps,
        dnse,
        tcbs,
        fireant,
        pool,
        cache,
    }
}

pub type SchemaGraphQL = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

pub fn create_graphql_schema() -> SchemaGraphQL {
    SchemaGraphQL::new(Query {}, Mutation {}, EmptySubscription::new())
}
