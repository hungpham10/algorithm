use clap::{Parser, Subcommand};

mod api;
mod crawl;
mod entities;
mod server;

#[derive(Parser, Debug)]
#[command(name = "algorithm", about = "An all in one solution")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Server {},

    #[cfg(feature = "full")]
    Crawl {
        #[arg(default_value = "ecommerce")]
        domain: String,
    },
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Khởi tạo logging
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().json().init();

    // Phân tích subcommand
    let cli = Cli::parse();

    match cli.command {
        Commands::Server {} => server::run().await,

        #[cfg(feature = "full")]
        Commands::Crawl { domain } => crawl::run(&domain).await,
    }
}
