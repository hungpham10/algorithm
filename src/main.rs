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

use chrono::Utc;
use tokio_schedule::{every, Job};

use ::lib::actors::redis::{
    RedisActor, InfoCommand,
    connect_to_redis,
};
use ::lib::actors::vps::{
    VpsActor, GetPriceCommand,
    connect_to_vps, list_of_vn30,
};
use ::lib::actors::cron::{
    CronResolver, 
    TickCommand, ScheduleCommand, 
    connect_to_cron,
};
use ::lib::helpers::{
    create_graphql_schema, create_graphql_context, 
    SchemaGraphQL,

    connect_to_postgres_pool,
    PgPool, 
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
    db:     web::Data<PgPool>,
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
        (*db).clone(),
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
    let pool = connect_to_postgres_pool(
        std::env::var("POSTGRES_DSN").unwrap()
    );
    let cache = connect_to_redis(
        std::env::var("REDIS_DSN").unwrap(),
    ).await;
    let schema = std::sync::Arc::new(
        create_graphql_schema(),
    );
    let tsdb = Arc::new(
        InfluxClient::new(
            std::env::var("INFLUXDB_URI").unwrap(),
            std::env::var("INFLUXDB_BUCKET").unwrap(),
        ) 
        .with_token(std::env::var("INFLUXDB_TOKEN").unwrap())
    );

    let vps  = connect_to_vps(
        &mut resolver,
        tsdb.clone(),
        list_of_vn30().await,
    );

    let cron = connect_to_cron(
        resolver.into(),
        pool.clone().into(),
        cache.clone().into(),
    );
    let scheduler = cron.clone();

    scheduler.send(ScheduleCommand{
        cron:  "* 2-10 * * 1-5".to_string(), 
        route: "vps.get_price_command".to_string(),
    }).await.unwrap();

    // @NOTE: mapping cronjobs
    actix_rt::spawn(async move {
        let every_second = every(1)
            .seconds()
            .in_timezone(&Utc)
            .perform(|| async {
                cron.clone().send(TickCommand).await;
            });
        every_second.await;
    });

    // @NOTE: mapping routes
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(schema.clone()))
            .wrap(middleware::Logger::default())
            .service(health)
            .service(graphql)
            .service(playground)
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
