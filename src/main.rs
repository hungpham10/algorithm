#[macro_use]
extern crate serde;

use std::sync::Arc;
use actix_web::{
    web, get, post, middleware, 
    App, HttpServer, 
    http::Method,
    Error, HttpRequest, HttpResponse,
};
use influxdb::{Client as InfluxClient};
use actix::Addr;
use juniper::http::{
    graphiql::graphiql_source, 
    GraphQLRequest,
};

use log::{info, error};
use chrono::Utc;
use tokio_schedule::{every, Job};

use ::lib::actors::redis::{
    RedisActor, InfoCommand,
    connect_to_redis,
};
use ::lib::actors::fireant::{
    connect_to_fireant,
};
use ::lib::actors::dnse::{
    DnseActor,
    connect_to_dnse,
};
use ::lib::actors::vps::{
    VpsActor,
    connect_to_vps, list_of_vn30,
};
use ::lib::actors::tcbs::{
    TcbsActor,
    connect_to_tcbs, 
};
use ::lib::actors::cron::{
    CronResolver, CronActor,
    TickCommand, ScheduleCommand, 
    connect_to_cron,
};
use ::lib::helpers::{
    create_graphql_schema, create_graphql_context, 
    SchemaGraphQL,

    connect_to_postgres_pool,
    PgPool, 
};
use ::lib::load::{
    load_and_map_schedulers_with_resolvers,
};

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

#[get("/")]
pub async fn playground() -> HttpResponse {
    let html = graphiql_source("/graphql", None);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[post("/graphql")]
async fn graphql(
    req:    HttpRequest,
schema: web::Data<Arc<SchemaGraphQL>>,
    query:  Option<web::Query<GraphQLRequest>>,
    body:   Option<web::Json<GraphQLRequest>>,
    vps:    web::Data<Addr<VpsActor>>,
    dnse:   web::Data<Addr<DnseActor>>,
    pool:   web::Data<PgPool>,
    cache:  web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, Error> {
    //let headers = req.headers();

    // fetch data from
    // query string if this is a GET
    // body if this is a POST
    let data = match *req.method() {
        Method::GET => query.unwrap().into_inner(),
        _           => body.unwrap().into_inner(),
    };

    let ctx = create_graphql_context(
        (*vps).clone(),
        (*dnse).clone(),
        (*pool).clone(),
        (*cache).clone(),
    );

    Ok(HttpResponse::Ok()
        .json(
            data.execute(
                &schema,
                &ctx,
            ).await,
        ),
    )
}

#[get("/health")]
async fn health(
    cache:  web::Data<Addr<RedisActor>>,
) -> actix_web::Result<HttpResponse> {
    let _ = cache.send(InfoCommand)
        .await
        .unwrap().unwrap().unwrap();

    Ok(HttpResponse::Ok()
        .body("OK")
        .into())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    // @NOTE: configure logging
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let mut resolver = CronResolver::new();
    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let pool = connect_to_postgres_pool(
        std::env::var("POSTGRES_DSN").unwrap()
    );
    let tsdb = Arc::new(
        InfluxClient::new(
            std::env::var("INFLUXDB_URI").unwrap(),
            std::env::var("INFLUXDB_BUCKET").unwrap(),
        ) 
        .with_token(std::env::var("INFLUXDB_TOKEN").unwrap())
    );
    let cache = connect_to_redis(
        std::env::var("REDIS_DSN").unwrap(),
    ).await;

    let schema = std::sync::Arc::new(
        create_graphql_schema(),
    );

    let dnse    = connect_to_dnse();
    let vps     = connect_to_vps(
        &mut resolver,
        tsdb.clone(),
        list_of_vn30().await,
    );
    let tcbs    = connect_to_tcbs(
        &mut resolver,
        pool.clone().into(),
        list_of_vn30().await,
    );
    let fireant = connect_to_fireant(
        &mut resolver,
        pool.clone().into(),
        cache.clone().into(),
        std::env::var("FIREANT_TOKEN").unwrap(),
    );

    let cron = connect_to_cron(
        resolver.into(),
        pool.clone().into(),
        cache.clone().into(),
    );

    load_and_map_schedulers_with_resolvers(
        pool.clone(),
        cron.clone(),
    ).await;

    // @NOTE: mapping cronjobs
    actix_rt::spawn(async move {
        let every_second = every(1)
            .seconds()
            .in_timezone(&Utc)
            .perform(|| async {
                let _ = cron.clone().send(TickCommand)
                    .await;
            });
        every_second.await;
    });

    // @NOTE: mapping routes
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(schema.clone()))
            .app_data(web::Data::new(vps.clone()))
            .app_data(web::Data::new(dnse.clone()))
            .app_data(web::Data::new(tcbs.clone()))
            .app_data(web::Data::new(fireant.clone()))
            .wrap(middleware::Logger::default())
            .service(health)
            .service(graphql)
            .service(playground)
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
