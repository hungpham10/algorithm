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

    Simulate {
        #[arg(long = "model", default_value = "single-symbol-with-trend-following")]
        model: String,

        #[arg(long = "market", default_value = "stock")]
        market: String,

        #[arg(long = "resolution", default_value = "1D")]
        resolution: String,

        #[arg(long = "lookback-to-year", default_value_t = 1)]
        backtest_year_ago: u8,

        #[arg(long = "symbols")]
        symbols: String,
    },
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

        Commands::Simulate {
            model,
            market,
            resolution,
            symbols,
            backtest_year_ago,
        } => {
            let symbol_list = symbols
                .split(',')
                .map(|it| it.trim())
                .filter(|it| !it.is_empty())
                .map(|it| it.to_string())
                .collect::<Vec<_>>();

            if symbol_list.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "At least one non-empty symbol is required",
                ));
            }

            simulate::run(
                model.as_str(),
                market.as_str(),
                &symbol_list,
                resolution.as_str(),
                backtest_year_ago as i64,
            )
            .await
        }
    }
}
