use actix::Addr;
use actix_web::{web, App, HttpResponse, HttpServer};

use chrono::Utc;

use tokio_schedule::{every, Job};

use crate::actors::cron::{connect_to_cron, CronActor, CronResolver, TickCommand};
use crate::actors::process::{connect_to_process_manager};

async fn health(
    cron: web::Data<Addr<CronActor>>,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("OK").into())
}

#[actix_rt::main]
pub async fn farm() -> std::io::Result<()> {
    env_logger::init();

    let resolver = CronResolver::new();
    let processer = connect_to_process_manager();

    let cron = connect_to_cron(resolver.into());
    let background: Addr<CronActor> = cron.clone();

    // @NOTE: mapping cronjobs
    actix_rt::spawn(async move {
        let every_second = every(1).seconds().in_timezone(&Utc).perform(|| async {
            let _ = cron.clone().send(TickCommand).await;
        });
        every_second.await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(background.clone()))
            .route("/health", web::get().to(health))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
