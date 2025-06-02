use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use actix::Addr;
use actix_web::middleware::Logger;
use actix_web::web::{get, put, Data};
use actix_web::{App, HttpResponse, HttpServer, Result};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use chrono::Utc;
use log::{error, info};

use vnscope::actors::cron::{connect_to_cron, CronResolver, ScheduleCommand, TickCommand};
use vnscope::actors::tcbs::{resolve_tcbs_routes, TcbsActor};
use vnscope::actors::vps::{resolve_vps_routes, VpsActor};
use vnscope::actors::UpdateStocksCommand;
use vnscope::algorithm::Variables;
use vnscope::schemas::Portal;

/// Health check endpoint that returns a 200 OK response.
///
/// # Examples
///
/// ```
/// let resp = health().await.unwrap();
/// assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
/// ```
async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

/// Synchronizes the stock symbol list between the portal and both the TcbsActor and VpsActor.
///
/// Fetches the current watchlist of stock symbols from the portal, then sends an update command with these symbols to both the TcbsActor and VpsActor. Returns an HTTP 200 OK response if synchronization succeeds; otherwise, returns an error if fetching the watchlist or communicating with the actors fails.
///
/// # Examples
///
/// ```
/// // This handler is registered as a PUT endpoint at `/api/v1/config/synchronize`.
/// // It is typically called via an HTTP client:
/// let response = client.put("/api/v1/config/synchronize").send().await?;
/// assert_eq!(response.status(), 200);
/// ```
async fn synchronize(
    portal: Data<Arc<Portal>>,
    tcbs: Data<Arc<Addr<TcbsActor>>>,
    vps: Data<Arc<Addr<VpsActor>>>,
) -> Result<HttpResponse> {
    let symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
        .collect::<Vec<String>>();

    tcbs.send(UpdateStocksCommand {
        stocks: symbols.clone(),
    })
    .await
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?;
    vps.send(UpdateStocksCommand {
        stocks: symbols.clone(),
    })
    .await
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?;
    Ok(HttpResponse::Ok().body("ok"))
}

#[actix_rt::main]
/// Starts the Actix-web server, initializes the cron job system, and manages graceful shutdown.
///
/// Loads configuration from environment variables, sets up logging, initializes the Airtable-backed portal,
/// fetches cronjob schedules and watchlist symbols, and registers cronjobs. Spawns asynchronous tasks to run
/// the cron loop and handle HTTP requests. Listens for system signals to coordinate an orderly shutdown of
/// both the cron system and the HTTP server.
///
/// # Returns
///
/// Returns `Ok(())` if the server and cron system shut down gracefully, or an error if initialization or binding fails.
///
/// # Errors
///
/// Returns an error if required environment variables are missing or invalid, if the server fails to bind to the specified address, or if there are issues communicating with the Airtable API.
///
/// # Examples
///
/// ```no_run
/// #[tokio::main]
/// async fn main() -> std::io::Result<()> {
///     my_crate::main().await
/// }
/// Initializes and runs the Actix-web server with integrated cron job scheduling and graceful shutdown.
///
/// Loads environment variables, configures logging, initializes shared resources, and sets up HTTP routes and cron jobs. Handles asynchronous task coordination and ensures orderly shutdown on receiving system signals.
///
/// # Returns
///
/// An `Ok(())` result if the server shuts down gracefully, or an error if initialization or binding fails.
///
/// # Examples
///
/// ```no_run
/// #[actix_rt::main]
/// async fn main() -> std::io::Result<()> {
///     main().await
/// }
/// Initializes and runs the Actix-web server with integrated cron job scheduling and graceful shutdown.
///
/// Loads configuration from environment variables, sets up the portal and actor system, registers HTTP endpoints, and manages cron jobs based on dynamic schedules. Handles Unix signals for coordinated shutdown of both the cron system and the HTTP server.
///
/// # Returns
/// Returns `Ok(())` if the server and cron system shut down gracefully, or an error if initialization or runtime fails.
///
/// # Examples
///
/// ```no_run
/// #[actix_rt::main]
/// async fn main() -> std::io::Result<()> {
///     main().await
/// }
/// ```
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
    ));

    // @NOTE: cronjob configuration
    let symbols: Vec<String> = portal
        .watchlist()
        .await
        .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{:?}", error)))?
        .iter()
        .filter_map(|record| Some(record.fields.symbol.as_ref()?.clone()))
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
    let s3_bucket = std::env::var("S3_BUCKET")
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_BUCKET"))?;
    let s3_name = std::env::var("S3_NAME")
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_NAME"))?;
    let s3_region = std::env::var("S3_REGION")
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_REGION"))?;
    let s3_endpoint = std::env::var("S3_ENDPOINT")
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid S3_ENDPOINT"))?;

    let tcbs_vars = Arc::new(Mutex::new(
        Variables::new_with_s3(
            tcbs_timeseries,
            tcbs_flush,
            s3_bucket.as_str(),
            s3_name.as_str(),
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
            s3_name.as_str(),
            Some(s3_region.as_str()),
            Some(s3_endpoint.as_str()),
        )
        .await,
    ));

    // @NOTE: setup cron and its resolvers
    let mut resolver = CronResolver::new();
    let tcbs = resolve_tcbs_routes(&mut resolver, &symbols, tcbs_vars.clone());
    let vps = resolve_vps_routes(&mut resolver, &symbols, vps_vars.clone());
    let cron = connect_to_cron(Rc::new(resolver));

    for command in cronjob {
        let route = command.route.clone();
        let cronjob = command.cron.clone();

        match cron.send(command).await {
            Ok(Ok(_)) => info!(
                "Registry {} running with cronjob {} success",
                route, cronjob
            ),
            Ok(Err(err)) => error!("Registry schedule {} failed: {:?}", route, err),
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

    // @NOTE: start cron
    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        info!(
            "Cron started at {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match cron.send(TickCommand).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(err)) => error!("Tick command failed: {:?}", err),
                        Err(error) => panic!("Panic: Fail to send command: {:?}", error),
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
            .wrap(Logger::default())
            .route("/health", get().to(health))
            .route("/api/v1/config/synchronize", put().to(synchronize))
            .app_data(Data::new(portal.clone()))
            .app_data(Data::new(tcbs.clone()))
            .app_data(Data::new(vps.clone()))
            .app_data(Data::new(tcbs_vars.clone()))
            .app_data(Data::new(vps_vars.clone()))
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
    tokio::select! {
        result = server => result,
        _ = rxserver => {
            info!("Server is downed gracefully...");
            Ok(())
        }
    }
}
