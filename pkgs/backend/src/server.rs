use std::io::{Error, ErrorKind};
use std::sync::Arc;

use actix_web::middleware::Logger;
use actix_web::web::{get, post, put, scope, Data};
use actix_web::{App, HttpServer};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::oneshot;

use chrono::Utc;
use log::{error, info};

use crate::api::{flush, health, lock, synchronize, unlock, AppState};

pub async fn run() -> std::io::Result<()> {
    // @NOTE: server configuration
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SERVER_PORT"))?;
    let concurrent = std::env::var("SERVER_CONCURRENT")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<usize>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SERVER_CONCURRENT"))?;

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
            // @NOTE: health-check
            .route("/health", get().to(health))
            // @NOTE: APIs for configuration
            .service(
                scope("/api/config")
                    .route("/v1/variables/flush", put().to(flush))
                    .route("/v1/cronjobs/synchronize", put().to(synchronize))
                    .route("/v1/cronjobs/lock", put().to(lock))
                    .route("/v1/cronjobs/unlock", put().to(unlock))
                    .route("/v1/seo/files/{path:.*}", get().to(crate::api::seo::file))
                    .route(
                        "/v1/seo/tenant/{host}/id",
                        get().to(crate::api::seo::tenant_id),
                    )
                    .route("/v1/seo/sitemap", get().to(crate::api::seo::sitemap)),
            )
            // @NOTE: APIs of OHCL
            .service(
                scope("/api/investing")
                    .route(
                        "/v1/ohcl/products/{broker}",
                        get().to(crate::api::ohcl::v1::get_list_of_product_by_broker),
                    )
                    .route(
                        "/v1/ohcl/{broker}/{symbol}",
                        get().to(crate::api::ohcl::v1::get_ohcl_from_broker),
                    )
                    .route(
                        "/v1/ohcl/{broker}/{symbol}/heatmap",
                        get().to(crate::api::ohcl::v1::get_heatmap_from_broker),
                    )
                    .route(
                        "/v1/ohcl/resolutions",
                        get().to(crate::api::ohcl::v1::get_list_of_resolutions),
                    )
                    .route(
                        "/v1/ohcl/brokers",
                        get().to(crate::api::ohcl::v1::get_list_of_brokers),
                    )
                    .route(
                        "/v1/ohcl/symbols/{broker}/{product}",
                        get().to(crate::api::ohcl::v1::get_list_of_symbols_by_product),
                    )
                    .route(
                        "/v1/ohcl/{broker}/symbols",
                        get().to(crate::api::ohcl::v1::get_list_of_symbols),
                    ),
            )
            // @NOTE: APIs of WMS
            .service(
                scope("/api/ecommerce")
                    .route("/v1/wms/stock", get().to(crate::api::wms::v1::list_stocks))
                    .route(
                        "/v1/wms/stocks",
                        post().to(crate::api::wms::v1::create_stocks),
                    )
                    .route(
                        "/v1/wms/stocks/{stock_id}",
                        get().to(crate::api::wms::v1::get_stock),
                    )
                    .route(
                        "/v1/wms/stocks/{stock_id}/flashsale",
                        get().to(crate::api::wms::v1::process_flash_sale),
                    )
                    .route(
                        "/v1/wms/stocks/{stock_id}/lots",
                        get().to(crate::api::wms::v1::list_lots),
                    )
                    .route(
                        "/v1/wms/lots/{lot_id}",
                        get().to(crate::api::wms::v1::get_lot),
                    )
                    .route("/v1/wms/lots", post().to(crate::api::wms::v1::create_lots))
                    .route(
                        "/v1/wms/lots/{lot_id}/plan",
                        post().to(crate::api::wms::v1::plan_item_for_new_lot),
                    )
                    .route(
                        "/v1/wms/lots/{lot_id}/import",
                        post().to(crate::api::wms::v1::import_item_to_warehouse),
                    )
                    .route(
                        "/v1/wms/shelves",
                        get().to(crate::api::wms::v1::list_shelves),
                    )
                    .route(
                        "/v1/wms/shelves/{shelve_id}",
                        get().to(crate::api::wms::v1::list_stocks_in_shelf),
                    )
                    .route(
                        "/v1/wms/shelves/{shelve_id}",
                        post().to(crate::api::wms::v1::assign_item_to_shelf),
                    )
                    .route(
                        "/v1/wms/shelves",
                        post().to(crate::api::wms::v1::create_shelves),
                    )
                    .route(
                        "/v1/wms/sale",
                        post().to(crate::api::wms::v1::process_normal_sale),
                    )
                    .route(
                        "/v1/wms/stock/barcode/{barcode}",
                        get().to(crate::api::wms::v1::get_item_by_barcode),
                    )
                    .route("/v1/wms/sync", post().to(crate::api::wms::v1::sync_data))
                    .route(
                        "/v1/wms/stock/near-expiry",
                        get().to(crate::api::wms::v1::get_near_expiry),
                    )
                    .route(
                        "/v1/wms/stock/outdated",
                        get().to(crate::api::wms::v1::get_outdated),
                    )
                    .route(
                        "/v1/wms/stock/high-turnover",
                        get().to(crate::api::wms::v1::get_high_turnover),
                    ),
            )
            // @NOTE: AppState
            .app_data(Data::new(appstate_for_control.clone()))
    })
    .workers(concurrent)
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
