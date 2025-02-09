use actix_web::{web, guard, App, HttpServer, HttpResponse};
use actix_files::Files;
use std::sync::Arc;

use lib::actors::dnse::connect_to_dnse;
use lib::api::order;

async fn health() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("ok"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    HttpServer::new(move || {
        let dnse = Arc::new(connect_to_dnse());

        App::new()
            .route("/health", web::get().to(health))
            .service(
                web::resource("/api/v1/orders")
                    .name("orders-management")
                    .guard(guard::Header("content-type", "application/json"))
                    .route(web::get().to(order::list))
                    .route(web::post().to(order::inquiry)),
            )
            .service(
                web::resource("/api/v1/orders/{id}")
                    .name("order-detail")
                    .guard(guard::Header("content-type", "application/json"))
                    .route(web::get().to(order::detail))
                    .route(web::delete().to(order::close)),
            )
            .service(Files::new("/", "./static").index_file("index.html"))
            .app_data(web::Data::new(dnse))
    })
    .bind(("0.0.0.0", 3000))
    .unwrap()
    .run()
    .await
}
