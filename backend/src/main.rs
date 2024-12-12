use ::lib::cmds::{server::graphql_server, server::sql_server};

fn main() {
    dotenvy::dotenv().ok();

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
        unknown => todo!("not yet implement {}", unknown),
    }.unwrap();
}
