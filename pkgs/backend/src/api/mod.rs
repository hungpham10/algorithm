use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind as AppErrorKind, Result as AppStateResult};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actix::Addr;
use actix_web::web::Data;
use actix_web::{HttpResponse, Result as HttpResult};
use actix_web_prometheus::{PrometheusMetrics, PrometheusMetricsBuilder};

use aws_config::{
    meta::region::RegionProviderChain, timeout::TimeoutConfig, BehaviorVersion, Region,
};
use aws_sdk_s3::Client as S3Client;

use infisical::secrets::GetSecretRequest;
use infisical::{AuthMethod, Client as InfiscalClient};

use chrono::Utc;
use log::{debug, error};
use redis::{AsyncCommands, Client as RedisClient, ErrorKind as RedisErrorKind, RedisResult};
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};

use vnscope::actors::cron::{
    connect_to_cron, CronActor, CronResolver, ScheduleCommand, TickCommand,
};
use vnscope::actors::price::{connect_to_price, PriceActor};
use vnscope::actors::tcbs::{resolve_tcbs_routes, TcbsActor};
use vnscope::actors::vps::{resolve_vps_routes, VpsActor};
use vnscope::actors::{FlushVariablesCommand, UpdateStocksCommand};
use vnscope::algorithm::fuzzy::Variables;
use vnscope::schemas::{Portal, CRONJOB, WATCHLIST};

use crate::entities;

pub mod chat;
pub mod ohcl;
pub mod seo;
pub mod wms;

use chat::Chat;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Status {
    crontime: Vec<i64>,
    current: i64,
    running: i64,
    done: i64,
    status: bool,
}

pub struct AppState {
    // @NOTE: monitoring
    crontime: Arc<Mutex<VecDeque<i64>>>,
    running: Arc<AtomicI64>,
    done: Arc<AtomicI64>,
    timeframe: usize,
    s3: S3Client,
    db: Option<Arc<DatabaseConnection>>,
    redis: Option<RedisClient>,
    prometheus: PrometheusMetrics,

    // @NOTE: state management
    locked: Arc<Mutex<bool>>,

    // @NOTE: database models
    ohcl_entity: Option<entities::ohcl::Ohcl>,
    wms_entity: Option<entities::wms::Wms>,
    seo_entity: Option<entities::seo::Seo>,
    chat_entity: Option<entities::chat::Chat>,

    // @NOTE: shared components
    portal: Arc<Portal>,
    price: Arc<Addr<PriceActor>>,
    tcbs: Arc<Addr<TcbsActor>>,
    vps: Arc<Addr<VpsActor>>,
    cron: Arc<Addr<CronActor>>,
    chat: Arc<Chat>,
    infisical_client: InfiscalClient,

    // @NOTE: variables
    tcbs_vars: Arc<Mutex<Variables>>,
    vps_vars: Arc<Mutex<Variables>>,
}

impl AppState {
    pub async fn new() -> AppStateResult<AppState> {
        // @NOTE: setup secret management client
        let mut infisical_client = InfiscalClient::builder().build().await.map_err(|error| {
            Error::new(
                AppErrorKind::InvalidInput,
                format!("Fail to build infisical client: {:?}", error),
            )
        })?;

        infisical_client
            .login(AuthMethod::new_universal_auth(
                std::env::var("INFISICAL_CLIENT_ID").map_err(|_| {
                    Error::new(AppErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_ID")
                })?,
                std::env::var("INFISICAL_CLIENT_SECRET").map_err(|_| {
                    Error::new(
                        AppErrorKind::InvalidInput,
                        "Invalid INFISICAL_CLIENT_SECRET",
                    )
                })?,
            ))
            .await
            .map_err(|error| {
                Error::new(
                    AppErrorKind::InvalidInput,
                    format!("Fail to login to infisical: {:?}", error),
                )
            })?;

        let redis_host = match std::env::var("REDIS_HOST") {
            Ok(redis_host) => redis_host,
            Err(_) => "".to_string(),
        };
        let redis_port = match std::env::var("REDIS_PORT") {
            Ok(redis_port) => redis_port,
            Err(_) => "".to_string(),
        };
        let redis_password = match std::env::var("REDIS_PASSWORD") {
            Ok(redis_password) => redis_password,
            Err(_) => "".to_string(),
        };
        let redis_username = match std::env::var("REDIS_USERNAME") {
            Ok(redis_password) => redis_password,
            Err(_) => "".to_string(),
        };

        let redis_dsn = get_secret_from_infisical(&infisical_client, "REDIS_DSN", "/")
            .await
            .unwrap_or("".to_string());
        let redis = match RedisClient::open(if redis_dsn.len() > 0 {
            redis_dsn
        } else {
            format!(
                "redis://{}:{}@{}:{}",
                redis_username, redis_password, redis_host, redis_port,
            )
        }) {
            Ok(redis) => Some(redis),
            Err(_) => None,
        };

        let portal = Arc::new(Portal::new(
            get_secret_from_infisical(&infisical_client, "AIRTABLE_API_KEY", "/feature-flags/")
                .await
                .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid AIRTABLE_API_KEY"))?
                .as_str(),
            get_secret_from_infisical(&infisical_client, "AIRTABLE_BASE_ID", "/feature-flags/")
                .await
                .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid AIRTABLE_BASE_ID"))?
                .as_str(),
            &HashMap::from([
                (
                    WATCHLIST.to_string(),
                    get_secret_from_infisical(
                        &infisical_client,
                        "AIRTABLE_TABLE_WATCHLIST",
                        "/feature-flags/",
                    )
                    .await
                    .unwrap_or_else(|_| WATCHLIST.to_string()),
                ),
                (
                    CRONJOB.to_string(),
                    get_secret_from_infisical(
                        &infisical_client,
                        "AIRTABLE_TABLE_CRONJOB",
                        "/feature-flags/",
                    )
                    .await
                    .unwrap_or_else(|_| CRONJOB.to_string()),
                ),
            ]),
            redis.clone(),
            get_secret_from_infisical(&infisical_client, "USE_AIRTABLE", "/feature-flags/")
                .await
                .unwrap_or_else(|_| "false".to_string())
                .parse::<bool>()
                .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid USE_AIRTABLE"))?,
        ));

        // @NOTE: cronjob configuration
        let vps_symbols: Vec<String> = portal
            .watchlist()
            .await
            .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?
            .iter()
            .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
            .collect::<Vec<String>>();
        let tcbs_symbols: Vec<String> = portal
            .watchlist()
            .await
            .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?
            .iter()
            .filter_map(|record| {
                if *(record.fields.use_order_flow.as_ref()?) {
                    Some(record.fields.symbol.as_ref()?.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        let tcbs_depth = std::env::var("TCBS_DEPTH")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid TCBS_DEPTH"))?;

        let tcbs_timeseries = std::env::var("TCBS_TIMESERIES")
            .unwrap_or_else(|_| "360".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid TCBS_TIMESERIES"))?;
        let tcbs_flush = std::env::var("TCBS_FLUSH")
            .unwrap_or_else(|_| "360".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid TCBS_FLUSH"))?;
        let vps_timeseries = std::env::var("VPS_TIMESERIES")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid VPS_TIMESERIES"))?;
        let vps_flush = std::env::var("VPS_FLUSH")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid VPS_FLUSH"))?;
        let s3_vps_name = std::env::var("S3_VPS_NAME").unwrap_or_else(|_| "vps".to_string());
        let s3_tcbs_name = std::env::var("S3_TCBS_NAME").unwrap_or_else(|_| "tcbs".to_string());
        let s3_bucket = std::env::var("S3_BUCKET")
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid S3_BUCKET"))?;
        let s3_region = std::env::var("S3_REGION")
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid S3_REGION"))?;
        let s3_endpoint = std::env::var("S3_ENDPOINT")
            .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid S3_ENDPOINT"))?;

        let chat = Arc::new(chat::Chat {
            fb: chat::Facebook {
                webhook_access_token: get_secret_from_infisical(
                    &infisical_client,
                    "FACEBOOK_WEBHOOK_VERIFY_TOKEN",
                    "/facebook/",
                )
                .await?,
                incomming_secret: get_secret_from_infisical(
                    &infisical_client,
                    "FACEBOOK_INCOMMING_SECRET",
                    "/facebook/",
                )
                .await?,
                outgoing_secret: get_secret_from_infisical(
                    &infisical_client,
                    "FACEBOOK_OUTGOING_SECRET",
                    "/facebook/",
                )
                .await?,
            },
            slack: chat::Slack {
                token: get_secret_from_infisical(&infisical_client, "SLACK_BOT_TOKEN", "/slack/")
                    .await?,
                channel: get_secret_from_infisical(&infisical_client, "SLACK_CHANNEL", "/slack/")
                    .await?,
            },
        });

        let db_dsn = get_secret_from_infisical(&infisical_client, "DB_DSN", "/")
            .await
            .unwrap_or("".to_string());
        let db = match std::env::var("DB_DSN") {
            Ok(dsn) => Some(Arc::new(
                Database::connect(if db_dsn.len() > 0 { db_dsn } else { dsn })
                    .await
                    .map_err(|error| {
                        Error::new(
                            AppErrorKind::InvalidInput,
                            format!("Failed to connect database: {}", error),
                        )
                    })?,
            )),
            Err(_) => {
                if db_dsn.len() > 0 {
                    Some(Arc::new(Database::connect(db_dsn).await.map_err(
                        |error| {
                            Error::new(
                                AppErrorKind::InvalidInput,
                                format!("Failed to connect database: {}", error),
                            )
                        },
                    )?))
                } else {
                    None
                }
            }
        };

        match get_secret_from_infisical(&infisical_client, "AWS_ACCESS_KEY_ID", "/").await {
            Ok(access_key) => {
                std::env::set_var("AWS_ACCESS_KEY_ID", &access_key);
            }
            Err(_) => {}
        }

        match get_secret_from_infisical(&infisical_client, "AWS_SECRET_ACCESS_KEY", "/").await {
            Ok(secret_key) => {
                std::env::set_var("AWS_SECRET_ACCESS_KEY", &secret_key);
            }
            Err(_) => {}
        }

        let s3 = S3Client::new(
            &(aws_config::defaults(BehaviorVersion::latest())
                .timeout_config(
                    TimeoutConfig::builder()
                        .operation_timeout(Duration::from_secs(30))
                        .operation_attempt_timeout(Duration::from_millis(10000))
                        .build(),
                )
                .region(
                    RegionProviderChain::first_try(Region::new(s3_region.clone()))
                        .or_default_provider(),
                )
                .endpoint_url(s3_endpoint.clone())
                .load()
                .await),
        );

        // @TODO: implement new flow to open multiple db connection pool
        //        to support sharding in multiple instances
        let ohcl_entity = match db {
            Some(ref db) => Some(entities::ohcl::Ohcl::new(db.clone())),
            None => None,
        };

        let wms_entity = match db {
            Some(ref db) => Some(entities::wms::Wms::new(vec![db.clone()])),
            None => None,
        };

        let seo_entity = match db {
            Some(ref db) => Some(entities::seo::Seo::new(vec![db.clone()])),
            None => None,
        };

        let chat_entity = match db {
            Some(ref db) => Some(entities::chat::Chat::new(vec![db.clone()])),
            None => None,
        };

        let tcbs_vars = Arc::new(Mutex::new(
            Variables::new_with_s3(
                tcbs_timeseries,
                tcbs_flush,
                s3_bucket.as_str(),
                s3_tcbs_name.as_str(),
                Some(s3_region.as_str()),
                Some(s3_endpoint.as_str()),
            )
            .await,
        ));
        let vps_vars = Arc::new(Mutex::new(
            Variables::new_with_s3(
                vps_timeseries,
                vps_flush,
                s3_bucket.as_str(),
                s3_vps_name.as_str(),
                Some(s3_region.as_str()),
                Some(s3_endpoint.as_str()),
            )
            .await,
        ));

        // @NOTE: setup cron and its resolvers
        let mut resolver = CronResolver::new();
        let prometheus = PrometheusMetricsBuilder::new("api")
            .endpoint("/metrics")
            .build()
            .map_err(|e| {
                Error::new(
                    AppErrorKind::Other,
                    format!("Failed to build prometheus metrics: {:?}", e),
                )
            })?;
        let tcbs = resolve_tcbs_routes(
            &prometheus,
            &mut resolver,
            &tcbs_symbols,
            tcbs_vars.clone(),
            tcbs_depth,
        )
        .await;
        let vps = resolve_vps_routes(&prometheus, &mut resolver, &vps_symbols, vps_vars.clone());
        let price = Arc::new(connect_to_price());
        let cron = Arc::new(connect_to_cron(Rc::new(resolver)));

        Ok(AppState {
            // @NOTE: shared paramters
            crontime: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(AtomicI64::new(0)),
            done: Arc::new(AtomicI64::new(0)),
            timeframe: std::env::var("APPSTATE_TIMEFRAME")
                .unwrap_or_else(|_| "4".to_string())
                .parse::<usize>()
                .map_err(|_| {
                    Error::new(AppErrorKind::InvalidInput, "Invalid APPSTATE_TIMEFRAME")
                })?,

            // @NOTE: database entities
            ohcl_entity,
            wms_entity,
            seo_entity,
            chat_entity,

            // @NOTE: monitors
            locked: Arc::new(Mutex::new(true)),
            s3,
            db,
            redis,
            prometheus,
            infisical_client,

            // @NOTE: shared actors
            portal: portal.clone(),
            price: price.clone(),
            cron: cron.clone(),
            vps: vps.clone(),
            tcbs: tcbs.clone(),
            chat: chat.clone(),

            // @NOTE: variables
            tcbs_vars,
            vps_vars,
        })
    }

    pub async fn init_scheduler_from_portal(&self) -> AppStateResult<()> {
        let cronjob: Vec<ScheduleCommand> = self
            .portal
            .cronjob()
            .await
            .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?
            .iter()
            .filter_map(|record| {
                Some(ScheduleCommand {
                    timeout: record.fields.timeout?,
                    cron: record.fields.cron.as_ref()?.clone(),
                    route: record.fields.route.as_ref()?.clone(),
                    jsfuzzy: Some(record.fields.fuzzy.as_ref()?.clone()),
                })
            })
            .collect::<Vec<ScheduleCommand>>();

        for command in cronjob {
            self.cron
                .send(command)
                .await
                .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?;
        }
        Ok(())
    }

    pub async fn send_tick_command_to_cron(&self) {
        let mut cron_on_updated = true;
        let locked = match self.locked.lock() {
            Ok(locked) => *locked,
            Err(_) => false,
        };

        if !locked {
            match self
                .cron
                .send(TickCommand {
                    running: self.running.clone(),
                    done: self.done.clone(),
                })
                .await
            {
                Ok(Ok(cnt)) => {
                    cron_on_updated = cnt > 0;
                    if cnt > 0 {
                        debug!("Success trigger {} jobs", cnt)
                    }
                }
                Ok(Err(err)) => error!("Tick command failed: {:?}", err),
                Err(error) => panic!("Panic: Fail to send command: {:?}", error),
            }
        }

        if cron_on_updated {
            match self.crontime.lock() {
                Ok(mut crontime) => {
                    crontime.push_back(Utc::now().timestamp());
                    if crontime.len() > self.timeframe {
                        crontime.pop_front();
                    }
                }
                Err(_) => {
                    error!("Failed to lock crontime mutex - skipping timestamp update");
                }
            }
        }
    }

    pub async fn flush_all_variables(&self) -> AppStateResult<()> {
        self.tcbs_vars
            .clone()
            .lock()
            .map_err(|e| Error::new(AppErrorKind::Other, format!("{:?}", e)))?
            .flush_all()
            .await
            .map_err(|e| Error::new(AppErrorKind::Other, e.message))?;

        self.vps_vars
            .clone()
            .lock()
            .map_err(|e| Error::new(AppErrorKind::Other, format!("{:?}", e)))?
            .flush_all()
            .await
            .map_err(|e| Error::new(AppErrorKind::Other, e.message))?;
        Ok(())
    }

    pub fn prometheus(&self) -> &PrometheusMetrics {
        &self.prometheus
    }

    pub fn wms_entity(&self) -> &Option<entities::wms::Wms> {
        &self.wms_entity
    }

    pub fn ohcl_entity(&self) -> &Option<entities::ohcl::Ohcl> {
        &self.ohcl_entity
    }

    pub fn seo_entity(&self) -> &Option<entities::seo::Seo> {
        &self.seo_entity
    }

    pub fn chat_entity(&self) -> &Option<entities::chat::Chat> {
        &self.chat_entity
    }

    pub async fn ping(&self) -> bool {
        let redis_ok = match &self.redis {
            Some(client) => {
                if let Ok(mut conn) = client.get_multiplexed_tokio_connection().await {
                    if let Ok(resp) = conn.ping::<String>().await {
                        resp == "PONG"
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            None => true,
        };
        let db_ok = match &self.db {
            Some(client) => client.ping().await.is_ok(),
            None => true,
        };

        redis_ok && db_ok
    }

    pub async fn get(&self, key: &String) -> RedisResult<String> {
        let client = self
            .redis
            .as_ref()
            .ok_or_else(|| (RedisErrorKind::IoError, "No Redis client available"))?;

        let mut conn = client.get_multiplexed_tokio_connection().await?;

        conn.get(key).await
    }

    pub async fn set(&self, key: &String, value: &String, ttl: usize) -> RedisResult<()> {
        let client = self
            .redis
            .as_ref()
            .ok_or_else(|| (RedisErrorKind::IoError, "No Redis client available"))?;

        let mut conn = client.get_multiplexed_tokio_connection().await?;

        conn.set_ex(key, value, ttl as u64).await
    }
}

pub async fn unlock(appstate: Data<Arc<AppState>>) -> HttpResult<HttpResponse> {
    match appstate.locked.lock() {
        Ok(mut locked) => {
            *locked = false;
            Ok(HttpResponse::Ok().body("ok"))
        }
        Err(_) => Ok(HttpResponse::InternalServerError().body("Cannot unlock system")),
    }
}

pub async fn lock(appstate: Data<Arc<AppState>>) -> HttpResult<HttpResponse> {
    match appstate.locked.lock() {
        Ok(mut locked) => {
            *locked = true;
            Ok(HttpResponse::Ok().body("ok"))
        }
        Err(_) => Ok(HttpResponse::InternalServerError().body("Cannot lock system")),
    }
}

pub async fn health(appstate: Data<Arc<AppState>>) -> HttpResult<HttpResponse> {
    let current = Utc::now().timestamp();

    let max_inflight = std::env::var("MAX_INFLIGHT")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<_>()
        .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid MAX_INFLIGHT"))?;

    let max_updated_time = std::env::var("MAX_UPDATED_TIME")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<_>()
        .map_err(|_| Error::new(AppErrorKind::InvalidInput, "Invalid MAX_UPDATED_TIME"))?;

    if appstate.ping().await {
        match appstate.crontime.lock() {
            Ok(crontime) => {
                let running = appstate.running.load(Ordering::SeqCst);
                let done = appstate.done.load(Ordering::SeqCst);
                let inflight = running.saturating_sub(done);
                let last_ok = crontime
                    .back()
                    .map_or(true, |updated| current - updated <= max_updated_time)
                    && inflight <= max_inflight;
                let builder = if last_ok {
                    HttpResponse::Ok
                } else {
                    HttpResponse::GatewayTimeout
                };

                Ok(builder().json(Status {
                    crontime: crontime.iter().cloned().collect(),
                    status: last_ok,
                    running,
                    done,
                    current,
                }))
            }
            Err(_) => Ok(HttpResponse::InternalServerError().json(Status {
                crontime: Vec::new(),
                status: false,
                running: 0,
                done: 0,
                current,
            })),
        }
    } else {
        Ok(HttpResponse::InternalServerError().json(Status {
            crontime: Vec::new(),
            status: false,
            running: 0,
            done: 0,
            current,
        }))
    }
}

pub async fn synchronize(appstate: Data<Arc<AppState>>) -> HttpResult<HttpResponse> {
    let portal = appstate.portal.clone();
    let vps_symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
        .collect::<Vec<String>>();
    let tcbs_symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| {
            if *(record.fields.use_order_flow.as_ref()?) {
                Some(record.fields.symbol.as_ref()?.clone())
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    let tcbs = appstate.tcbs.clone();
    let vps = appstate.vps.clone();

    tcbs.send(UpdateStocksCommand {
        stocks: tcbs_symbols.clone(),
    })
    .await
    .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?;
    vps.send(UpdateStocksCommand {
        stocks: vps_symbols.clone(),
    })
    .await
    .map_err(|error| Error::new(AppErrorKind::InvalidInput, format!("{:?}", error)))?;
    Ok(HttpResponse::Ok().body("ok"))
}

pub async fn flush(appstate: Data<Arc<AppState>>) -> HttpResult<HttpResponse> {
    let tcbs = appstate.tcbs.clone();
    let vps = appstate.vps.clone();

    match tcbs.send(FlushVariablesCommand).await {
        Ok(Ok(_)) => match vps.send(FlushVariablesCommand).await {
            Ok(Ok(_)) => Ok(HttpResponse::Ok().body("ok")),
            Ok(Err(error)) => Ok(HttpResponse::BadRequest().body(format!("{}", error))),
            Err(error) => Ok(HttpResponse::BadRequest().body(format!("{}", error))),
        },
        Ok(Err(error)) => Ok(HttpResponse::BadRequest().body(format!("{}", error))),
        Err(error) => Ok(HttpResponse::BadRequest().body(format!("{}", error))),
    }
}

pub async fn get_secret_from_infisical(
    client: &InfiscalClient,
    key: &str,
    path: &str,
) -> Result<String, Error> {
    if let Ok(value) = std::env::var(key) {
        Ok(value)
    } else {
        let request = GetSecretRequest::builder(
            key,
            std::env::var("INFISICAL_PROJECT_ID").map_err(|_| {
                Error::new(AppErrorKind::InvalidInput, "Invalid INFISICAL_PROJECT_ID")
            })?,
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
        )
        .path(path)
        .build();

        let secret = client.secrets().get(request).await.map_err(|error| {
            Error::new(
                AppErrorKind::InvalidInput,
                format!("Fail fetching secret: {:?}", error),
            )
        })?;

        Ok(secret.secret_value)
    }
}
