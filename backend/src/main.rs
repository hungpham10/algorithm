#![feature(portable_simd)]

use ::lib::cmds::server::{
    server,
    bff,
    auth,
    render,
    event,
    datasource,
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
        // @NOTE: this kind of sub-commands represent specific deployments
        "server" => server(),
        "bff" => bff(),
        "auth" => auth(),
        "render" => render(),
        "event" => event(),

        // @NOTE: this kind of sub-commands represent specific deployments
        "datasource" => datasource(),
        unknown => panic!("Unknown command: {}", unknown),
    }.unwrap();
}
