pub mod server;
pub mod client;

use actix::Addr;
use actix_web::{
    http::Method, 
    guard, 
    middleware, 
    web, 
    App, 
    Error, 
    HttpRequest, 
    HttpResponse, 
    HttpServer,
};
use actix_files::Files;

use influxdb::Client as InfluxClient;
use juniper::http::{graphiql::graphiql_source, GraphQLRequest};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use tokio_schedule::{every, Job};
use tokio::sync::broadcast;

use std::sync::Arc;

use crate::actors::cron::{connect_to_cron, CronActor, CronResolver, TickCommand};
use crate::actors::dnse::{connect_to_dnse, DnseActor};
use crate::actors::fireant::{connect_to_fireant, FireantActor};
use crate::actors::process::{HealthCommand, ProcessActor};
use crate::actors::redis::{connect_to_redis, InfoCommand, RedisActor};
use crate::actors::tcbs::{connect_to_tcbs, TcbsActor};
use crate::actors::vps::{connect_to_vps, list_of_vn30, VpsActor};
use crate::components::simulator::connect_to_simulator;
use crate::components::auth::connect_to_auth;
use crate::components::renderer::connect_to_renderer;
use crate::components::event::connect_to_event;
use crate::actors::websocket::Websocket;
use crate::helpers::{connect_to_postgres_pool, PgPool, Shutdown};
use crate::load::load_and_map_schedulers_with_resolvers;
use crate::interfaces::graphql::{create_graphql_context, create_graphql_schema, SchemaGraphQL};

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

struct Application {
    cache: Arc<Addr<RedisActor>>,
    pool: Arc<PgPool>,
    tsdb: Arc<InfluxClient>, 

    // @NOTE: shutdown manager
    shutdown: Shutdown,

    // @NOTE: shutdown notifier
    notify_shutdown: broadcast::Sender<()>,
}

impl Application {
    pub async fn new() -> Application {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    
        // @NOTE: configure external actors
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
            tsdb,
            pool,
            cache,

            // @NOTE: shutdown manager
            shutdown: Shutdown::new(shutdown_rx),

            // @NOTE: shutdown notifier
            notify_shutdown: shutdown_tx,
        }
    }

    pub async fn handon_auth_server(&self, port: u16) -> std::io::Result<()> {
        HttpServer::new(move || {
            let mut ws = Websocket::new();
            let (auth, _) = connect_to_auth(
            
            );

            App::new()
                .route("/health", web::get().to(health))
                .service(
                    ws.configure(
                        "/auth.v1/Auth.Login", 
                        Box::pin(auth.clone()),
                    )
                )
                .service(
                    ws.configure(
                        "/auth.v1/Auth.Registry", 
                        Box::pin(auth.clone()),
                    )
                )
                .app_data(web::Data::new(Arc::new(ws)))
        })
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await
    }

    pub async fn handon_render_server(&self, port: u16) -> std::io::Result<()> {
        HttpServer::new(move || {
            let mut ws = Websocket::new();
            let (renderer, _) = connect_to_renderer(
            
            );

            App::new()
                .route("/health", web::get().to(health))
                .service(
                    ws.configure(
                        "/render.v1/Render.FetchPageInfomation", 
                        Box::pin(renderer.clone()),
                    )
                )
                .service(
                    ws.configure(
                        "/render.v1/Render.FetchComponentInfomation", 
                        Box::pin(renderer.clone()),
                    )
                )
                .app_data(web::Data::new(Arc::new(ws)))
        })
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await
    }

    pub async fn handon_event_server(&self, port: u16) -> std::io::Result<()> {
        HttpServer::new(move || {
            let mut ws = Websocket::new();
            let (event, _) = connect_to_event(
                
            );

            App::new()
                .route("/health", web::get().to(health))
                .service(
                    ws.configure(
                        "/event.v1/Event.FetchPageInfomation", 
                        Box::pin(event.clone()),
                    )
                )
                .service(
                    ws.configure(
                        "/event.v1/Event.FetchComponentInfomation", 
                        Box::pin(event.clone()),
                    )
                )
                .app_data(web::Data::new(Arc::new(ws)))
        })
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await
    }

    pub async fn handon_bff_server(
        &self, 
        is_monolith: bool, 
        port: u16,
    ) -> std::io::Result<()> {
        if is_monolith {
            let mut resolver = CronResolver::new();
            let cache = self.cache.clone();
            let pool = self.pool.clone();
            let tsdb = self.tsdb.clone();

            // @NOTE: configure datasources
            let dnse = Arc::new(connect_to_dnse());
            let vps = Arc::new(connect_to_vps(&mut resolver, tsdb.clone(), list_of_vn30().await));
            let tcbs = Arc::new(connect_to_tcbs(&mut resolver, pool.clone().into(), list_of_vn30().await));
            let fireant = Arc::new(connect_to_fireant(
                &mut resolver,
                self.pool.clone(),
                std::env::var("FIREANT_TOKEN").unwrap()),
            );
            
            // @NOTE: configure internal actors
            let simulator = connect_to_simulator(
                &mut resolver,
                dnse.clone(),
                1000,
                10,
                true,
            );

            // @NOTE: configure cron actor and apply resolver
            let cron = Arc::new(connect_to_cron(resolver.into()));

            load_and_map_schedulers_with_resolvers(pool.clone(), cron.clone())
                .await;

            actix_rt::spawn(async move {
                let every_second = every(1)
                    .seconds()
                    .in_timezone(&Utc)
                    .perform(|| async {
                        let _ = cron.clone().send(TickCommand).await;
                    });
                every_second.await;
            });

            HttpServer::new(move || {
                let mut ws = Websocket::new();

                let cache = cache.clone();
                let pool = pool.clone();
                let vps = vps.clone();
                let dnse = dnse.clone();
                let tcbs = tcbs.clone();
                let fireant = fireant.clone();
                let simulator = simulator.clone();

                App::new()
                    .wrap(middleware::Logger::default())
                    .route("/health", web::get().to(health))
                    .service(
                        ws.configure(
                            "/simulate.v1/Simulator.StoreSimulateSession",
                            Box::pin(simulator.clone()),
                        )
                    )
                    .service(
                        ws.configure(
                            "/simulate.v1/Simulator.FetchSimulateSession", 
                            Box::pin(simulator.clone()),
                        )
                    )
                    .service(
                        ws.configure(
                            "/simulate.v1/Simulator.ConsumeSimulateStrategySession", 
                            Box::pin(simulator.clone()),
                        )
                    )
                
                    // @TODO: implement handler for each resource bellow
                    .service(
                        web::resource("/auth/v1/login")
                            .name("auth_login")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/callback")
                            .name("auth")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/logout")
                            .name("auth_logout")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/event/v1/{page}")
                            .name("event_page")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/event/v1/{page}/{component}")
                            .name("event_component")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/ui/v1/{page}")
                            .name("render_page")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/ui/v1/{page}/{component}")
                            .name("render_component")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/{user}")
                            .name("user")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok))
                            .route(web::put().to(HttpResponse::Ok)),
                    )
                    .service(Files::new("/", "./static").index_file("index.html"))
                    .app_data(web::Data::new(Arc::new(ws)))
                    .app_data(web::Data::new(vps))
                    .app_data(web::Data::new(dnse))
                    .app_data(web::Data::new(tcbs))
                    .app_data(web::Data::new(fireant))
                    .app_data(web::Data::new(cache))
                    .app_data(web::Data::new(pool))
            })
            .bind(("0.0.0.0", port))
            .unwrap()
            .run()
            .await
        } else {
            HttpServer::new(move || {
                App::new()
                    .wrap(middleware::Logger::default())

                    // @TODO: implement handler for each resource bellow
                    .service(
                        web::resource("/auth/v1/login")
                            .name("auth_login")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/callback")
                            .name("auth")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/logout")
                            .name("auth_logout")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/event/v1/{page}")
                            .name("event_page")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/event/v1/{page}/{component}")
                            .name("event_component")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/ui/v1/{page}")
                            .name("render_page")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/ui/v1/{page}/{component}")
                            .name("render_component")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok)),
                    )
                    .service(
                        web::resource("/auth/v1/{user}")
                            .name("user")
                            .guard(guard::Header("content-type", "application/json"))
                            .route(web::get().to(HttpResponse::Ok))
                            .route(web::put().to(HttpResponse::Ok)),
                    )
                    .service(Files::new("/", "./static").index_file("index.html"))
                    .route("/health", web::get().to(health))
            })
            .bind(("0.0.0.0", port))
            .unwrap()
            .run()
            .await
        }
    }

    pub async fn handon_datasource_server(&self, port: u16) -> std::io::Result<()> {
        let mut resolver = CronResolver::new();
        let cache = self.cache.clone();
        let pool = self.pool.clone();
        let tsdb = self.tsdb.clone();

        // @NOTE: configure datasources
        let dnse = Arc::new(connect_to_dnse());
        let vps = Arc::new(connect_to_vps(&mut resolver, tsdb.clone(), list_of_vn30().await));
        let tcbs = Arc::new(connect_to_tcbs(&mut resolver, pool.clone().into(), list_of_vn30().await));
        let fireant = Arc::new(connect_to_fireant(
            &mut resolver,
            self.pool.clone(),
            std::env::var("FIREANT_TOKEN").unwrap()),
        );
        
        // @NOTE: configure internal actors
        let graphql = Arc::new(create_graphql_schema());
        let simulator = connect_to_simulator(
            &mut resolver,
            dnse.clone(),
            1000,
            10,
            true,
        );

        // @NOTE: configure cron actor and apply resolver
        let cron = Arc::new(connect_to_cron(resolver.into()));

        load_and_map_schedulers_with_resolvers(pool.clone(), cron.clone())
            .await;

        actix_rt::spawn(async move {
            let every_second = every(1)
                .seconds()
                .in_timezone(&Utc)
                .perform(|| async {
                    let _ = cron.clone().send(TickCommand).await;
                });
            every_second.await;
        });

        // @NOTE: mapping http routes
        HttpServer::new(move || {
            let mut ws = Websocket::new();

            let graphql = graphql.clone();
            let cache = cache.clone();
            let pool = pool.clone();
            let vps = vps.clone();
            let dnse = dnse.clone();
            let tcbs = tcbs.clone();
            let fireant = fireant.clone();
            let simulator = simulator.clone();

            App::new()
                .wrap(middleware::Logger::default())
                .route("/health", web::get().to(health))
                .service(
                    ws.configure(
                        "/simulate.v1/Simulator.StoreSimulateSession",
                        Box::pin(simulator.clone()),
                    )
                )
                .service(
                    ws.configure(
                        "/simulate.v1/Simulator.FetchSimulateSession", 
                        Box::pin(simulator.clone()),
                    )
                )
                .service(
                    ws.configure(
                        "/simulate.v1/Simulator.ConsumeSimulateStrategySession", 
                        Box::pin(simulator.clone()),
                    )
                )
                .app_data(web::Data::new(Arc::new(ws)))
                .app_data(web::Data::new(graphql))
                .app_data(web::Data::new(vps))
                .app_data(web::Data::new(dnse))
                .app_data(web::Data::new(tcbs))
                .app_data(web::Data::new(fireant))
                .app_data(web::Data::new(cache))
                .app_data(web::Data::new(pool))
        })
        .bind(("0.0.0.0", port))
        .unwrap()
        .run()
        .await
    }
}
