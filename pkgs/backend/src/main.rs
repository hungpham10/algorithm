use clap::{Parser, Subcommand};

mod api;
mod entities;
mod server;

#[cfg(feature = "crawl")]
mod crawl;

mod simulate;

#[derive(Parser, Debug)]
#[command(name = "algorithm", about = "An all in one solution")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Server {},

    #[cfg(feature = "crawl")]
    Crawl {
        #[arg(default_value = "ecommerce")]
        domain: String,
    },

    Simulate {},
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Khởi tạo logging
    dotenvy::dotenv().ok();
    env_logger::init();

    // Phân tích subcommand
    let cli = Cli::parse();

    match cli.command {
        Commands::Server {} => server::run().await,

        #[cfg(feature = "crawl")]
        Commands::Crawl { domain } => crawl::run(&domain).await,

        Commands::Simulate {} => simulate::run().await,
    }
}
