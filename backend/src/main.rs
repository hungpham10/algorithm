use std::rc::Rc;
use std::sync::Arc;

use actix_web::middleware::Logger;
use actix_web::web::{get, Data};
use actix_web::{App, HttpResponse, HttpServer, Result};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use log::info;

use vnscope::actors::cron::{connect_to_cron, CronResolver, TickCommand};
use vnscope::actors::tcbs::resolve_tcbs_routes;

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // @NOTE: setup cron and its resolvers
    let mut resolver = CronResolver::new();
    let tcbs = resolve_tcbs_routes(&mut resolver, &Vec::new());
    let cron = Arc::new(connect_to_cron(Rc::new(resolver)));

    // @NOTE: control cron
    let (txstop, mut rxstop) = oneshot::channel::<()>();
    let (txcron, rxcron) = oneshot::channel::<()>();
    let (txserver, rxserver) = oneshot::channel::<()>();

    // @NOTE: start cron
    actix_rt::spawn(async move {
        let mut sigtime = signal(SignalKind::alarm()).unwrap();

        unsafe {
            libc::alarm(1);
        }

        loop {
            tokio::select! {
                _ = sigtime.recv() => {
                    unsafe {
                        libc::alarm(1);
                    }

                    match cron.send(TickCommand).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(_)) => {}
                        Err(error) => panic!("Panic: Fail to send command: {:?}", error),
                    }

                }
                _ = &mut rxstop => {
                    info!("Cron is down...");
                    unsafe {
                        libc::alarm(0);
                    }

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
            .app_data(Data::new(tcbs.clone()))
    })
    .bind(("0.0.0.0", 8000))
    .unwrap()
    .shutdown_timeout(30)
    .run();

    let handler = server.handle();

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
