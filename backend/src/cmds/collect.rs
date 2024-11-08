use actix::Addr;
use actix_files::Files;
use actix_web::{http::Method, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use influxdb::Client as InfluxClient;
use juniper::http::{graphiql::graphiql_source, GraphQLRequest};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use chrono::Utc;
use tokio_schedule::{every, Job};

use crate::actors::cron::{connect_to_cron, CronActor, CronResolver, TickCommand};
use crate::actors::dnse::{connect_to_dnse, DnseActor};
use crate::actors::fireant::{connect_to_fireant, FireantActor};
use crate::actors::process::{connect_to_process_manager, HealthCommand, ProcessActor, RunCommand};
use crate::actors::redis::{connect_to_redis, InfoCommand, RedisActor};
use crate::actors::tcbs::{connect_to_tcbs, TcbsActor};
use crate::actors::vps::{connect_to_vps, list_of_vn30, VpsActor};
use crate::helpers::{
    connect_to_postgres_pool, create_graphql_context, create_graphql_schema, PgPool, SchemaGraphQL,
};
use crate::load::{load_and_map_schedulers_with_resolvers, load_sub_processes_from_pgpool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQlErrorLocation {
    pub line: i32,
    pub column: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Vec<GraphQlErrorLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLErrors {
    pub errors: Vec<GraphQLError>,
}

impl GraphQLErrors {
    pub fn new(message: &str) -> GraphQLErrors {
        GraphQLErrors {
            errors: vec![GraphQLError {
                message: message.to_owned(),
                locations: Vec::new(),
            }],
        }
    }
}

pub async fn playground() -> HttpResponse {
    let html = graphiql_source("/graphql", None);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

async fn graphql(
    req: HttpRequest,
    schema: web::Data<Arc<SchemaGraphQL>>,
    query: Option<web::Query<GraphQLRequest>>,
    body: Option<web::Json<GraphQLRequest>>,
    cron: web::Data<Addr<CronActor>>,
    vps: web::Data<Addr<VpsActor>>,
    tcbs: web::Data<Addr<TcbsActor>>,
    dnse: web::Data<Addr<DnseActor>>,
    fireant: web::Data<Addr<FireantActor>>,
    pool: web::Data<PgPool>,
    cache: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, Error> {
    //let headers = req.headers();

    let data = match *req.method() {
        Method::GET => query.unwrap().into_inner(),
        _ => body.unwrap().into_inner(),
    };

    let ctx = create_graphql_context(
        (*cron).clone(),
        (*vps).clone(),
        (*dnse).clone(),
        (*tcbs).clone(),
        (*fireant).clone(),
        (*pool).clone(),
        (*cache).clone(),
    );

    Ok(HttpResponse::Ok().json(data.execute(&schema, &ctx).await))
}

async fn health(
    cache: web::Data<Addr<RedisActor>>,
    cron: web::Data<Addr<CronActor>>,
    processer: web::Data<Arc<Addr<ProcessActor>>>,
    vps: web::Data<Addr<VpsActor>>,
    dnse: web::Data<Addr<DnseActor>>,
    tcbs: web::Data<Addr<TcbsActor>>,
    fireant: web::Data<Addr<FireantActor>>,
) -> actix_web::Result<HttpResponse> {
    let cache_status = cache.send(InfoCommand).await.unwrap().unwrap().unwrap();
    let cron_tick = cron.send(crate::actors::cron::HealthCommand).await.unwrap();
    let vps_ok = vps.send(crate::actors::vps::HealthCommand).await.unwrap();
    let dnse_ok = dnse.send(crate::actors::dnse::HealthCommand).await.unwrap();
    let tcbs_ok = tcbs.send(crate::actors::tcbs::HealthCommand).await.unwrap();
    let fireant_ok = fireant
        .send(crate::actors::fireant::HealthCommand)
        .await
        .unwrap();

    let process_stats = processer.send(HealthCommand)
        .await
        .unwrap();

    if cache_status.len() > 0 && cron_tick > 0 && 
            vps_ok && dnse_ok && tcbs_ok && fireant_ok &&
            process_stats {
        Ok(HttpResponse::Ok().body("OK").into())
    } else {
        Ok(HttpResponse::ServiceUnavailable().body("Failed").into())
    }
}

async fn simulate() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("OK").into())
}

#[actix_rt::main]
pub async fn collect() -> std::io::Result<()> {
    env_logger::init();

    let mut resolver = CronResolver::new();
    let processer = connect_to_process_manager();
    let pool = connect_to_postgres_pool(std::env::var("POSTGRES_DSN").unwrap());
    let tsdb = Arc::new(
        InfluxClient::new(
            std::env::var("INFLUXDB_URI").unwrap(),
            std::env::var("INFLUXDB_BUCKET").unwrap(),
        )
        .with_token(std::env::var("INFLUXDB_TOKEN").unwrap()),
    );

    let cache = connect_to_redis(std::env::var("REDIS_DSN").unwrap()).await;
    let schema = std::sync::Arc::new(create_graphql_schema());
    let dnse = connect_to_dnse();
    let vps = connect_to_vps(&mut resolver, tsdb.clone(), list_of_vn30().await);
    let tcbs = connect_to_tcbs(&mut resolver, pool.clone().into(), list_of_vn30().await);
    let fireant = connect_to_fireant(
        &mut resolver,
        pool.clone().into(),
        cache.clone().into(),
        std::env::var("FIREANT_TOKEN").unwrap(),
    );
    let cron = connect_to_cron(resolver.into());
    let background: Addr<CronActor> = cron.clone();

    load_and_map_schedulers_with_resolvers(pool.clone(), cron.clone()).await;
    load_sub_processes_from_pgpool(pool.clone(), std::env::var("INSTANCE").unwrap(), processer.clone()).await;

    // @NOTE: mapping cronjobs
    actix_rt::spawn(async move {
        let every_second = every(1).seconds().in_timezone(&Utc).perform(|| async {
            let _ = cron.clone().send(TickCommand).await;
        });
        every_second.await;
    });

    // @NOTE: mapping routes
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(background.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(schema.clone()))
            .app_data(web::Data::new(vps.clone()))
            .app_data(web::Data::new(dnse.clone()))
            .app_data(web::Data::new(tcbs.clone()))
            .app_data(web::Data::new(fireant.clone()))
            .app_data(web::Data::new(processer.clone()))
            .route("/health", web::get().to(health))
            .route("/graphql", web::post().to(graphql))
            .route("/api/v1/simulate", web::post().to(simulate))
            .route("/playground", web::get().to(playground))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

