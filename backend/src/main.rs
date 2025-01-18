#![feature(portable_simd)]

use ::lib::cmds::{
    client::background_job_client, 
    server::monolith_server,
    server::graphql_server, 
    server::sql_server
};

fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").unwrap(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    match std::env::args().nth(1).unwrap().as_str() {
        "graphql-server" => graphql_server(),
        "sql-server" => sql_server(),
        "server" => monolith_server(),
        "job" => background_job_client(),
        unknown => panic!("Unknown command: {}", unknown),
    }.unwrap();
}
