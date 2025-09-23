use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind, Result as AppStateResult};
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
use redis::{AsyncCommands, Client as RedisClient};
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
        let portal = Arc::new(Portal::new(
            std::env::var("AIRTABLE_API_KEY")
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_API_KEY"))?
                .as_str(),
            std::env::var("AIRTABLE_BASE_ID")
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_BASE_ID"))?
                .as_str(),
            &HashMap::from([
                (
                    WATCHLIST.to_string(),
                    std::env::var("AIRTABLE_TABLE_WATCHLIST")
                        .unwrap_or_else(|_| WATCHLIST.to_string()),
                ),
                (
                    CRONJOB.to_string(),
                    std::env::var("AIRTABLE_TABLE_CRONJOB").unwrap_or_else(|_| CRONJOB.to_string()),
                ),
            ]),
        ));

        // @NOTE: cronjob configuration
        let vps_symbols: Vec<String> = portal
            .watchlist()
            .await
            .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
            .iter()
            .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
            .collect::<Vec<String>>();
        let tcbs_symbols: Vec<String> = portal
            .watchlist()
            .await
            .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
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
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid TCBS_DEPTH"))?;

        let tcbs_timeseries = std::env::var("TCBS_TIMESERIES")
            .unwrap_or_else(|_| "360".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid TCBS_TIMESERIES"))?;
        let tcbs_flush = std::env::var("TCBS_FLUSH")
            .unwrap_or_else(|_| "360".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid TCBS_FLUSH"))?;
        let vps_timeseries = std::env::var("VPS_TIMESERIES")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid VPS_TIMESERIES"))?;
        let vps_flush = std::env::var("VPS_FLUSH")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid VPS_FLUSH"))?;
        let s3_vps_name = std::env::var("S3_VPS_NAME").unwrap_or_else(|_| "vps".to_string());
        let s3_tcbs_name = std::env::var("S3_TCBS_NAME").unwrap_or_else(|_| "tcbs".to_string());
        let s3_bucket = std::env::var("S3_BUCKET")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_BUCKET"))?;
        let s3_region = std::env::var("S3_REGION")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_REGION"))?;
        let s3_endpoint = std::env::var("S3_ENDPOINT")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_ENDPOINT"))?;

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

        let redis = match RedisClient::open(format!(
            "redis://{}:{}@{}:{}",
            redis_username, redis_password, redis_host, redis_port
        )) {
            Ok(redis) => Some(redis),
            Err(_) => None,
        };

        // @NOTE: setup secret management client
        let mut infisical_client = InfiscalClient::builder().build().await.map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Fail to build infisical client: {:?}", error),
            )
        })?;

        let auth_method = AuthMethod::new_universal_auth(
            std::env::var("INFISICAL_CLIENT_ID")
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_ID"))?,
            std::env::var("INFISICAL_CLIENT_SECRET").map_err(|_| {
                Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_CLIENT_SECRET")
            })?,
        );
        infisical_client.login(auth_method).await.map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Fail to login to infisical: {:?}", error),
            )
        })?;

        let chat = Arc::new(chat::Chat {
            fb: chat::Facebook {
                token: get_secret_from_infisical(&infisical_client, "FACEBOOK_TOKEN").await?,
                incomming_secret: get_secret_from_infisical(
                    &infisical_client,
                    "FACEBOOK_INCOMMING_SECRET",
                )
                .await?,
                outgoing_secret: get_secret_from_infisical(
                    &infisical_client,
                    "FACEBOOK_OUTGOING_SECRET",
                )
                .await?,
            },
            slack: chat::Slack {
                token: get_secret_from_infisical(&infisical_client, "SLACK_TOKEN").await?,
            },
        });

        let db = match std::env::var("MYSQL_DSN") {
            Ok(dsn) => Some(Arc::new(Database::connect(dsn).await.map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Failed to connect database: {}", error),
                )
            })?)),
            Err(_) => None,
        };

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

        let ohcl_entity = match db {
            Some(ref db) => Some(entities::ohcl::Ohcl::new(db.clone())),
            None => None,
        };

        let wms_entity = match db {
            Some(ref db) => Some(entities::wms::Wms::new(db.clone())),
            None => None,
        };

        let seo_entity = match db {
            Some(ref db) => Some(entities::seo::Seo::new(db.clone())),
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
                    ErrorKind::Other,
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
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid APPSTATE_TIMEFRAME"))?,

            // @NOTE: database entities
            ohcl_entity,
            wms_entity,
            seo_entity,

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
            .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
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
                .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?;
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
            .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?
            .flush_all()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.message))?;

        self.vps_vars
            .clone()
            .lock()
            .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?
            .flush_all()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.message))?;
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
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MAX_INFLIGHT"))?;

    let max_updated_time = std::env::var("MAX_UPDATED_TIME")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<_>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MAX_UPDATED_TIME"))?;

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
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
        .collect::<Vec<String>>();
    let tcbs_symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
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
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?;
    vps.send(UpdateStocksCommand {
        stocks: vps_symbols.clone(),
    })
    .await
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?;
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

async fn get_secret_from_infisical(client: &InfiscalClient, key: &str) -> Result<String, Error> {
    let request = GetSecretRequest::builder(
        key,
        std::env::var("INFISICAL_PROJECT_ID")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid INFISICAL_PROJECT_ID"))?,
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
    )
    .build();

    let secret = client.secrets().get(request).await.map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Fail fetching secret: {:?}", error),
        )
    })?;

    Ok(secret.secret_value)
}
