use std::io::IsTerminal;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// Applies pending database migrations, then exits.
#[derive(Parser)]
struct Args {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Colour codes only help on a terminal; they are noise in container logs.
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    let pool = db::connect(&args.database_url, 1).await?;
    db::MIGRATOR.run(&pool).await?;
    pool.close().await;

    tracing::info!("migrations applied");
    Ok(())
}
