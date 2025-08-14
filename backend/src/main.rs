use std::io::{Error, ErrorKind};
use std::sync::Arc;

use actix_web::middleware::Logger;
use actix_web::web::{get, put, Data};
use actix_web::{App, HttpServer};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use chrono::Utc;
use log::{error, info};

mod api;

use crate::api::investing::ohcl;
use crate::api::{echo, flush, health, lock, synchronize, unlock, AppState};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().json().init();

    // @NOTE: server configuration
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SERVER_PORT"))?;

    // @NOTE: control cron
    let (txstop, mut rxstop) = oneshot::channel::<()>();
    let (txcron, rxcron) = oneshot::channel::<()>();
    let (txserver, rxserver) = oneshot::channel::<()>();

    // @NOTE: store appstate
    let appstate_for_control = Arc::new(AppState::new().await?);
    let appstate_for_config = appstate_for_control.clone();
    let appstate_for_release = appstate_for_control.clone();

    // @NOTE: start cron
    actix_rt::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let appstate = appstate_for_config.clone();

        match appstate.init_scheduler_from_portal().await {
            Ok(_) => info!(
                "Cron started at {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            ),
            Err(err) => error!("Failed to fetch scheduler commands: {}", err),
        }

        loop {
            let appstate = appstate.clone();

            tokio::select! {
                _ = interval.tick() => {
                    appstate.send_tick_command_to_cron().await;
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
            .wrap(appstate_for_control.prometheus().clone())
            .wrap(Logger::default())
            .route("/health", get().to(health))
            .route("/api/v1/variables/flush", put().to(flush))
            .route("/api/v1/config/synchronize", put().to(synchronize))
            .route("/api/v1/config/lock", put().to(lock))
            .route("/api/v1/config/unlock", put().to(unlock))
            .route("/api/investing/v1/ohcl/{broker}", get().to(ohcl))
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

    #[cfg(not(feature = "bff"))]
    tokio::select! {
        _ = rxserver => {
            info!("Server is downed gracefully...");

            appstate_for_release.flush_all_variables().await?;
            info!("Finish flushing variables");
            ok
        }
    }

    #[cfg(feature = "bff")]
    ok
}
