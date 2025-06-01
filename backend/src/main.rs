use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use actix_web::middleware::Logger;
use actix_web::web::{get, put, Data};
use actix_web::{App, HttpResponse, HttpServer, Result};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use chrono::Utc;
use log::{error, info};

use vnscope::actors::cron::{connect_to_cron, CronResolver, ScheduleCommand, TickCommand};
use vnscope::actors::tcbs::resolve_tcbs_routes;
use vnscope::actors::vps::resolve_vps_routes;
use vnscope::algorithm::Variables;
use vnscope::schemas::Portal;

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

async fn synchronize() -> Result<HttpResponse> {
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

    let portal = Portal::new(
        std::env::var("AIRTABLE_API_KEY")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_API_KEY"))?
            .as_str(),
        std::env::var("AIRTABLE_BASE_ID")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid AIRTABLE_BASE_ID"))?
            .as_str(),
    );

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

    let variables = Arc::new(Mutex::new(Variables::new(6 * 60)));

    // @NOTE: setup cron and its resolvers
    let mut resolver = CronResolver::new();
    let tcbs = resolve_tcbs_routes(&mut resolver, &symbols);
    let vps = resolve_vps_routes(&mut resolver, &symbols, variables.clone());
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
            .app_data(Data::new(tcbs.clone()))
            .app_data(Data::new(vps.clone()))
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
