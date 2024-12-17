use actix::Addr;
use actix_web::{http::Method, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};

use influxdb::Client as InfluxClient;
use juniper::http::{graphiql::graphiql_source, GraphQLRequest};
use pgwire::tokio::process_socket;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use tokio_schedule::{every, Job};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use std::io;
use std::sync::Arc;

use crate::actors::cron::{connect_to_cron, CronActor, CronResolver, TickCommand};
use crate::actors::dnse::{connect_to_dnse, DnseActor};
use crate::actors::fireant::{connect_to_fireant, FireantActor};
use crate::actors::process::{HealthCommand, ProcessActor};
use crate::actors::redis::{connect_to_redis, InfoCommand, RedisActor};
use crate::actors::tcbs::{connect_to_tcbs, TcbsActor};
use crate::actors::vps::{connect_to_vps, list_of_vn30, VpsActor};
use crate::helpers::{connect_to_postgres_pool, create_graphql_schema, PgPool, SchemaGraphQL, Shutdown};
use crate::load::load_and_map_schedulers_with_resolvers;
use crate::schemas::graphql::create_graphql_context;
use crate::interfaces::pgwire::create_sql_context;

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
    let cron_ok = cron.send(crate::actors::HealthCommand).await.unwrap();
    let vps_ok = vps.send(crate::actors::HealthCommand).await.unwrap();
    let dnse_ok = dnse.send(crate::actors::HealthCommand).await.unwrap();
    let tcbs_ok = tcbs.send(crate::actors::HealthCommand).await.unwrap();
    let fireant_ok = fireant.send(crate::actors::HealthCommand).await.unwrap();

    let process_stats = processer.send(HealthCommand).await.unwrap();

    if cache_status.len() > 0
        && cron_ok
        && vps_ok
        && dnse_ok
        && tcbs_ok
        && fireant_ok
        && process_stats
    {
        Ok(HttpResponse::Ok().body("OK").into())
    } else {
        Ok(HttpResponse::ServiceUnavailable().body("Failed").into())
    }
}

struct DataSource {
    dnse: Arc<Addr<DnseActor>>,
    vps: Arc<Addr<VpsActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    fireant: Arc<Addr<FireantActor>>,
}

impl DataSource {
    async fn new(
        resolver: &mut CronResolver, 
        pool: Arc<PgPool>,
        cache: Arc<Addr<RedisActor>>,
        tsdb: Arc<InfluxClient>, 
    ) -> DataSource {
        let dnse = Arc::new(connect_to_dnse());
        let vps = Arc::new(connect_to_vps(resolver, tsdb.clone(), list_of_vn30().await));
        let tcbs = Arc::new(connect_to_tcbs(resolver, pool.clone().into(), list_of_vn30().await));
        let fireant = Arc::new(connect_to_fireant(
            resolver,
            pool.clone(),
            cache.clone(),
            std::env::var("FIREANT_TOKEN").unwrap()),
        );

        DataSource {
            dnse,
            vps,
            tcbs,
            fireant,
        }
    }
}

struct Application {
    ds: DataSource,
    schema: Arc<SchemaGraphQL>,
    cache: Arc<Addr<RedisActor>>,
    pool: Arc<PgPool>,
    tsdb: Arc<InfluxClient>,
    cron: Arc<Addr<CronActor>>,

    // @NOTE: shutdown manager
    shutdown: Shutdown,

    // @NOTE: shutdown notifier
    notify_shutdown: broadcast::Sender<()>,
}

impl Application {
    pub async fn new() -> Application {
        let mut resolver = CronResolver::new();
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    
        let cache = Arc::new(
            connect_to_redis(std::env::var("REDIS_DSN").unwrap()).await
        );
        let pool = Arc::new(
            connect_to_postgres_pool(std::env::var("POSTGRES_DSN").unwrap())
        );
        let tsdb = Arc::new(
            InfluxClient::new(
                std::env::var("INFLUXDB_URI").unwrap(),
                std::env::var("INFLUXDB_BUCKET").unwrap(),
            )
            .with_token(std::env::var("INFLUXDB_TOKEN").unwrap()),
        );
    
        Application {
            ds: DataSource::new(&mut resolver, pool.clone(), cache.clone(), tsdb.clone()).await,
            cron: Arc::new(connect_to_cron(resolver.into())),
            schema: Arc::new(create_graphql_schema()),
            pool,
            tsdb,
            cache,

            // @NOTE: shutdown manager
            shutdown: Shutdown::new(shutdown_rx),

            // @NOTE: shutdown notifier
            notify_shutdown: shutdown_tx,
        }
    }

    pub async fn start_cron(&self) {
        // @NOTE: mapping cronjobs
        let cron = self.cron.clone();
        let pool = self.pool.clone();

        actix_rt::spawn(async move {
            load_and_map_schedulers_with_resolvers(pool, cron.clone()).await;

            let every_second = every(1)
                .seconds()
                .in_timezone(&Utc)
                .perform(|| async {
                    let _ = cron.clone().send(TickCommand).await;
                });
            every_second.await;
        });
    }

    pub async fn handon_sql_server(&self, port: usize, cache_capacity: usize) -> io::Result<()>{
        // @NOTE: configure and start sql server
        let factory = create_sql_context(
            cache_capacity,
            self.cron.clone(),
            self.ds.vps.clone(),
            self.ds.dnse.clone(),
            self.ds.tcbs.clone(),
            self.ds.fireant.clone(),
        );

        let server_addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(server_addr).await.unwrap();

        while !self.shutdown.is_shutdown() {
            let (socket, _) = listener.accept()
                .await
                .unwrap();
            let factory_ref = factory.clone();

            actix_rt::spawn(async move {
                let _ = process_socket(
                        socket, 
                        None, 
                        factory_ref
                    )
                    .await;
            });
        }

        Ok(())
    }

    pub async fn handon_bff_server(&self, port: u16) -> std::io::Result<()> {
        // @NOTE: mapping http routes
        let cron = self.cron.clone();
        let pool = self.pool.clone();
        let cache = self.cache.clone();
        let schema = self.schema.clone();
        let vps = self.ds.vps.clone();
        let dnse = self.ds.dnse.clone();
        let tcbs = self.ds.tcbs.clone();
        let fireant = self.ds.fireant.clone();
        let ret = HttpServer::new(move || {
            App::new()
                .wrap(middleware::Logger::default())
                .app_data(web::Data::new(cron.clone()))
                .app_data(web::Data::new(cache.clone()))
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(schema.clone()))
                .app_data(web::Data::new(vps.clone()))
                .app_data(web::Data::new(dnse.clone()))
                .app_data(web::Data::new(tcbs.clone()))
                .app_data(web::Data::new(fireant.clone()))
                .route("/health", web::get().to(health))
                .route("/graphql", web::post().to(graphql))
                .route("/playground", web::get().to(playground))
        })
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await;

        let _ = self.notify_shutdown.send(());
        return ret;
    }

}

#[actix_rt::main]
pub async fn graphql_server() -> std::io::Result<()> {
    let app = Application::new().await;

    app.start_cron()
        .await;
    app.handon_bff_server(3000)
        .await
}

#[actix_rt::main]
pub async fn sql_server() -> std::io::Result<()> {
    let app = Application::new().await;
    let capacity = std::env::var("SQL_CAPACITY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

    app.start_cron()
        .await;

    app.handon_sql_server(3001, capacity)
        .await
}

#[actix_rt::main]
pub async fn monolith_server() -> std::io::Result<()> {
    // @NOTE: configure application
    let app = Arc::new(Application::new().await);
    let capacity = std::env::var("SQL_CAPACITY")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

    // @NOTE: start cron first
    app.start_cron().await;

    let sql_server = app.clone();
    let http_server = app.clone();

    actix_rt::spawn(async move {
        sql_server
            .handon_sql_server(5432, capacity)
            .await
    });

    http_server
        .handon_bff_server(3000)
        .await
}
