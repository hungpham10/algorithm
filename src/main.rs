use influxdb::Client as InfluxClient;
use std::sync::Arc;

use ::lib::helpers::connect_to_postgres_pool;
use ::lib::cmds::{chat::chat, ggcolab::ggcolab, collect::collect};

fn main() {
    dotenvy::dotenv().ok();

    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let pool = Arc::new(connect_to_postgres_pool(std::env::var("POSTGRES_DSN").unwrap()));
    let tsdb = Arc::new(
        InfluxClient::new(
            std::env::var("INFLUXDB_URI").unwrap(),
            std::env::var("INFLUXDB_BUCKET").unwrap(),
        )
        .with_token(std::env::var("INFLUXDB_TOKEN").unwrap()),
    );

    match std::env::args().nth(2).unwrap().as_str() {
        "collect" => collect(tsdb, pool),
        "chat" => chat(),
        "ggcolab" => ggcolab(),
        unknown => todo!("not yet implement {}", unknown),
    }.unwrap();
}
