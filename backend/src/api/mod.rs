use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind, Result as AppStateResult};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use actix::Addr;
use actix_web::middleware::Logger;
use actix_web::web::{get, put, Data};
use actix_web::{App, HttpResponse, HttpServer, Result};
use actix_web_prometheus::PrometheusMetricsBuilder;

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use chrono::Utc;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

use vnscope::actors::cron::{
    connect_to_cron, CronActor, CronResolver, ScheduleCommand, TickCommand,
};
use vnscope::actors::tcbs::{resolve_tcbs_routes, TcbsActor};
use vnscope::actors::vps::{resolve_vps_routes, VpsActor};
use vnscope::actors::{FlushVariablesCommand, UpdateStocksCommand};
use vnscope::algorithm::fuzzy::Variables;
use vnscope::schemas::{Portal, CRONJOB, WATCHLIST};

pub struct AppState {
    // @NOTE: monitoring
    crontime: Arc<Mutex<VecDeque<i64>>>,
    running: Arc<AtomicI64>,
    done: Arc<AtomicI64>,
    timeframe: usize,

    // @NOTE: state management
    locked: Arc<Mutex<bool>>,

    // @NOTE: shared components
    portal: Arc<Portal>,
    tcbs: Arc<Addr<TcbsActor>>,
    vps: Arc<Addr<VpsActor>>,
    cron: Arc<Addr<CronActor>>,

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
        let cron = Arc::new(connect_to_cron(Rc::new(resolver)));

        Ok(AppState {
            // @NOTE
            crontime: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(AtomicI64::new(0)),
            done: Arc::new(AtomicI64::new(0)),
            timeframe: std::env::var("APPSTATE_TIMEFRAME")
                .unwrap_or_else(|_| "4".to_string())
                .parse::<usize>()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid APPSTATE_TIMEFRAME"))?,

            // @NOTE:
            locked: Arc::new(Mutex::new(true)),

            // @NOTE:
            portal: portal.clone(),
            cron: cron.clone(),
            vps: vps.clone(),
            tcbs: tcbs.clone(),

            // @NOTE:
            tcbs_vars,
            vps_vars,
        })
    }

    pub async fn init_scheduler_from_portal(&self) -> Result<()> {
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

    pub async fn flush_all_variables(&self) -> Result<()> {
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
}

pub async fn unlock(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    match appstate.locked.lock() {
        Ok(mut locked) => {
            *locked = false;
            Ok(HttpResponse::Ok().body("ok"))
        }
        Err(_) => Ok(HttpResponse::InternalServerError().body("Cannot unlock system")),
    }
}

pub async fn lock(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    match appstate.locked.lock() {
        Ok(mut locked) => {
            *locked = true;
            Ok(HttpResponse::Ok().body("ok"))
        }
        Err(_) => Ok(HttpResponse::InternalServerError().body("Cannot lock system")),
    }
}

pub async fn health(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    let current = Utc::now().timestamp();

    let max_inflight = std::env::var("MAX_INFLIGHT")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<_>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MAX_INFLIGHT"))?;

    let max_updated_time = std::env::var("MAX_UPDATED_TIME")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<_>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid MAX_UPDATED_TIME"))?;

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
}

async fn synchronize(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
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

pub async fn flush(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
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
