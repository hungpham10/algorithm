use std::collections::{HashMap, VecDeque};
use std::io::{Error, ErrorKind};
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
use vnscope::actors::UpdateStocksCommand;
use vnscope::algorithm::fuzzy::Variables;
use vnscope::schemas::{Portal, CRONJOB, WATCHLIST};

struct AppState {
    // @NOTE: monitoring
    crontime: Arc<Mutex<VecDeque<i64>>>,
    running: Arc<AtomicI64>,
    done: Arc<AtomicI64>,
    timeframe: usize,

    // @NOTE: shared components
    portal: Arc<Portal>,
    tcbs: Arc<Addr<TcbsActor>>,
    vps: Arc<Addr<VpsActor>>,
    cron: Arc<Addr<CronActor>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Status {
    crontime: Vec<i64>,
    current: i64,
    running: i64,
    done: i64,
    status: bool,
}

async fn health(appstate: Data<Arc<AppState>>) -> Result<HttpResponse> {
    let current = Utc::now().timestamp();
    match appstate.crontime.lock() {
        Ok(crontime) => {
            let running = appstate.running.load(Ordering::SeqCst);
            let done = appstate.done.load(Ordering::SeqCst);
            let inflight = running.saturating_sub(done);
            let last_ok = crontime
                .back()
                .map_or(true, |updated| current - updated <= 120)
                && inflight <= 2;
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

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // @NOTE: server configuration
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SERVER_PORT"))?;

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
                std::env::var("AIRTABLE_TABLE_WATCHLIST").unwrap_or_else(|_| WATCHLIST.to_string()),
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
    let cronjob: Vec<ScheduleCommand> = portal
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

    for command in cronjob {
        let cron = cron.clone();
        let route = command.route.clone();
        let cronjob = command.cron.clone();

        match cron.send(command).await {
            Ok(_) => info!(
                "Registry {} running with cronjob {} success",
                route, cronjob
            ),
            Err(err) => {
                error!("Fail in mailbox when schedule {}: {:?}", route, err);
                break;
            }
        }
    }

    // @NOTE: control cron
    let (txstop, mut rxstop) = oneshot::channel::<()>();
    let (txcron, rxcron) = oneshot::channel::<()>();
    let (txserver, rxserver) = oneshot::channel::<()>();

    // @NOTE: store appstate
    let appstate_for_control = Arc::new(AppState {
        crontime: Arc::new(Mutex::new(VecDeque::new())),
        running: Arc::new(AtomicI64::new(0)),
        done: Arc::new(AtomicI64::new(0)),
        timeframe: std::env::var("APPSTATE_TIMEFRAME")
            .unwrap_or_else(|_| "4".to_string())
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid APPSTATE_TIMEFRAME"))?,

        // @NOTE:
        portal: portal.clone(),
        cron: cron.clone(),
        vps: vps.clone(),
        tcbs: tcbs.clone(),
    });
    let appstate_for_config = appstate_for_control.clone();

    // @NOTE: start cron
    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let appstate = appstate_for_config.clone();

        info!(
            "Cron started at {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );

        loop {
            let appstate = appstate.clone();

            tokio::select! {
                _ = interval.tick() => {
                    let mut cron_on_updated = true;
                    let appstate = appstate.clone();
                    let cron = appstate.cron.clone();

                    match cron.send(TickCommand{
                        running: appstate.running.clone(),
                        done: appstate.done.clone(),
                    }).await {
                        Ok(Ok(cnt)) => {
                            cron_on_updated = cnt > 0;
                            debug!("Success trigger {} jobs", cnt)
                        },
                        Ok(Err(err)) => error!("Tick command failed: {:?}", err),
                        Err(error) => panic!("Panic: Fail to send command: {:?}", error),
                    }

                    if cron_on_updated {
                        match appstate.crontime.lock() {
                            Ok(mut crontime) => {
                                crontime.push_back(Utc::now().timestamp());
                                if crontime.len() > appstate.timeframe {
                                    crontime.pop_front();
                                }
                            }
                            Err(_) => {
                                error!("Failed to lock crontime mutex - skipping timestamp update");
                            }
                        }
                    }
                }
                _ = &mut rxstop => {
                    info!("Cron is down...");

                    txcron.send(()).unwrap();
                    break;
                }
            }
        }
    });

    // @NOTE: spawn new http server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(prometheus.clone())
            .wrap(Logger::default())
            .route("/health", get().to(health))
            .route("/api/v1/config/synchronize", put().to(synchronize))
            .app_data(Data::new(appstate_for_control.clone()))
    })
    .bind((host.as_str(), port))
    .map_err(|e| {
        Error::new(
            ErrorKind::AddrInUse,
            format!("Failed to bind to {}:{}: {}", host, port, e),
        )
    })?
    .shutdown_timeout(30)
    .run();

    let handler = server.handle();

    info!(
        "Server started at {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
    );

    // @NOTE: graceful shutdown
    actix_rt::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }

        info!("Shutting down...");
        let _ = txstop.send(());

        tokio::select! {
            _ = rxcron => {
                info!("Cron is downed gracefully...");

                handler.stop(true).await;
            }
        }

        info!("Server is going to shutdown...");
        let _ = txserver.send(());
    });

    // @NOTE: wait for everything to finish
    let ok = tokio::select! {
        result = server => result,
    };

    tokio::select! {
        _ = rxserver => {
            info!("Server is downed gracefully...");

            tcbs_vars.clone()
                .lock()
                .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?
                .flush_all()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.message))?;

            vps_vars.clone()
                .lock()
                .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?
                .flush_all()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.message))?;
            info!("Finish flushing variables");
            ok
        }
    }
}
