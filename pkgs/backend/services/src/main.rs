mod api;
mod gateway;
mod vector;

use clap::{Parser, Subcommand};
use std::io::{Error, ErrorKind};

#[derive(Parser, Debug)]
#[command(name = "algorithm", about = "An all in one solution")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand, Debug)]
enum Commands {
    Gateway {},
}

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let _tls = rustls::crypto::ring::default_provider().install_default();
    let _guard = sentry::init((
        std::env::var("SENTRY_DSN")
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SENTRY_DSN"))?,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            // Capture all traces and spans. Set to a lower value in production
            traces_sample_rate: 1.0,
            // Capture user IPs and potentially sensitive headers when using HTTP server integrations
            // see https://docs.sentry.io/platforms/rust/data-management/data-collected for more info
            send_default_pii: true,
            // Capture all HTTP request bodies, regardless of size
            max_request_body_size: sentry::MaxRequestBodySize::Always,
            ..Default::default()
        },
    ));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            match &(Cli::parse().command) {
                Some(Commands::Gateway {}) => gateway::run().await.unwrap(),
                None => gateway::run().await.unwrap(),
            }
        });
    Ok(())
}
