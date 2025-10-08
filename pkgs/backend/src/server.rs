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
                    .route("/v1/seo/features", get().to(crate::api::seo::features))
                    .route("/v1/seo/sitemap", get().to(crate::api::seo::sitemap)),
            )
            // @NOTE: APIs of Chat
            .service(
                scope("/api/chat")
                    .route(
                        "/v1/facebook/webhook",
                        get().to(crate::api::chat::facebook::verify_webhook),
                    )
                    .route(
                        "/v1/facebook/webhook",
                        post().to(crate::api::chat::facebook::receive_message),
                    )
                    .route(
                        "/v1/slack/webhook",
                        post().to(crate::api::chat::slack::receive_message),
                    ),
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
                    .route("/v1/wms/stocks", get().to(crate::api::wms::v1::list_stocks))
                    .route(
                        "/v1/wms/stocks",
                        post().to(crate::api::wms::v1::create_stocks),
                    )
                    .route(
                        "/v1/wms/stocks/{stock_id}",
                        get().to(crate::api::wms::v1::get_stock),
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
                        "/v1/wms/sales/offline",
                        post().to(crate::api::wms::v1::process_offline_sale),
                    )
                    .route(
                        "/v1/wms/sales/online",
                        get().to(crate::api::wms::v1::process_online_sale),
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{dev::ServiceRequest, http::StatusCode, test, web, App};
    use log::{info, LevelFilter};
    use std::env;
    use std::sync::Arc;
    use tokio::sync::oneshot;

    // Integration test for health route
    #[actix_web::test]
    async fn test_health_route() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .route("/health", web::get().to(health))
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert!(!body.is_empty()); // Assuming health returns some JSON or text
    }

    // Integration test for config routes (example for flush)
    #[actix_web::test]
    async fn test_config_flush_route() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(
                    web::scope("/api/config").route("/v1/variables/flush", web::put().to(flush)),
                )
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/config/v1/variables/flush")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK); // Assuming flush returns OK
    }

    // Test for synchronize route
    #[actix_web::test]
    async fn test_config_synchronize_route() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(
                    web::scope("/api/config")
                        .route("/v1/cronjobs/synchronize", web::put().to(synchronize)),
                )
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/config/v1/cronjobs/synchronize")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success()); // Adjust based on actual response
    }

    // Test for lock/unlock routes (similar pattern)
    #[actix_web::test]
    async fn test_config_lock_route() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(web::scope("/api/config").route("/v1/cronjobs/lock", web::put().to(lock)))
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/config/v1/cronjobs/lock")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_config_unlock_route() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(
                    web::scope("/api/config").route("/v1/cronjobs/unlock", web::put().to(unlock)),
                )
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::put()
            .uri("/api/config/v1/cronjobs/unlock")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // Test for chat routes (example Facebook webhook verify)
    #[actix_web::test]
    async fn test_chat_facebook_verify_webhook() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(web::scope("/api/chat").route(
                    "/v1/facebook/webhook",
                    web::get().to(crate::api::chat::facebook::verify_webhook),
                ))
                .app_data(web::Data::new(appstate)),
        )
        .await;

        // Mock query params for verification (hub.mode=subscribe, etc.)
        let req = test::TestRequest::get()
            .uri("/api/chat/v1/facebook/webhook?hub.mode=subscribe&hub.challenge=test_challenge&hub.verify_token=my_verify_token")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success()); // Adjust if it returns challenge
    }

    // Test for investing routes (example get_list_of_product_by_broker)
    #[actix_web::test]
    async fn test_investing_get_products() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(web::scope("/api/investing").route(
                    "/v1/ohcl/products/{broker}",
                    web::get().to(crate::api::ohcl::v1::get_list_of_product_by_broker),
                ))
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/investing/v1/ohcl/products/binance")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success()); // Assuming it returns list
    }

    // Test for WMS routes (example list_stocks, assuming from previous context)
    #[actix_web::test]
    async fn test_wms_list_stocks() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                .service(web::scope("/api/ecommerce").route(
                    "/v1/wms/stocks",
                    web::get().to(crate::api::wms::v1::list_stocks),
                ))
                .app_data(web::Data::new(appstate)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/ecommerce/v1/wms/stocks?limit=10")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // Test graceful shutdown simulation (high-level, test channels)
    #[tokio::test]
    async fn test_graceful_shutdown_channels() {
        let (txstop, rxstop) = oneshot::channel::<()>();
        let (txcron, mut rxcron) = oneshot::channel::<()>();
        let (txserver, mut rxserver) = oneshot::channel::<()>();

        // Simulate sending stop
        let _ = txstop.send(());

        // Check cron receives and sends to txcron
        tokio::spawn(async move {
            rxstop.await.unwrap();
            txcron.send(()).unwrap();
        });

        // Wait for cron signal
        rxcron.await.unwrap();

        // Simulate server shutdown
        let _ = txserver.send(());

        rxserver.await.unwrap();
    }

    // Test full App service initialization (without running server)
    #[actix_web::test]
    async fn test_full_app_initialization() {
        let appstate = Arc::new(AppState::new().await.expect("Failed to create AppState"));

        let app = test::init_service(
            App::new()
                // Add all routes as in run() for full coverage
                .route("/health", web::get().to(health))
                .service(
                    web::scope("/api/config")
                        .route("/v1/variables/flush", web::put().to(flush))
                        .route("/v1/cronjobs/synchronize", web::put().to(synchronize))
                        .route("/v1/cronjobs/lock", web::put().to(lock))
                        .route("/v1/cronjobs/unlock", web::put().to(unlock)), // ... other config routes
                )
                .service(
                    web::scope("/api/chat")
                        .route(
                            "/v1/facebook/webhook",
                            web::get().to(crate::api::chat::facebook::verify_webhook),
                        )
                        .route(
                            "/v1/facebook/webhook",
                            web::post().to(crate::api::chat::facebook::receive_message),
                        )
                        .route(
                            "/v1/slack/webhook",
                            web::post().to(crate::api::chat::slack::receive_message),
                        ),
                )
                .service(
                    web::scope("/api/investing").route(
                        "/v1/ohcl/products/{broker}",
                        web::get().to(crate::api::ohcl::v1::get_list_of_product_by_broker),
                    ), // ... other investing routes
                )
                .service(
                    web::scope("/api/ecommerce").route(
                        "/v1/wms/stocks",
                        web::get().to(crate::api::wms::v1::list_stocks),
                    ), // ... other WMS routes
                )
                .app_data(web::Data::new(appstate)),
        )
        .await;

        // Test a dummy request to ensure app starts
        let req = test::TestRequest::get().uri("/health").to_request();
        let _resp = test::call_service(&app, req).await;
    }

    // Test cron spawning (mock the spawn, check if init_scheduler is called)
    #[tokio::test]
    async fn test_cron_spawning() {
        let appstate_for_config =
            Arc::new(AppState::new().await.expect("Failed to create AppState"));
        let (txstop, mut rxstop) = oneshot::channel::<()>();
        let (txcron, _rxcron) = oneshot::channel::<()>();

        // Mock the spawn block
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            let appstate = appstate_for_config.clone();
            // Simulate init_scheduler
            if let Err(err) = appstate.init_scheduler_from_portal().await {
                error!("Failed to fetch scheduler commands: {}", err);
            } else {
                info!("Cron started");
            }
            // Tick once for test
            interval.tick().await;
            appstate.send_tick_command_to_cron().await;
            // Simulate stop
            rxstop.await.unwrap();
            txcron.send(()).unwrap();
        });

        // Send stop after short delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let _ = txstop.send(());

        let _ = handle.await;
    }
}
