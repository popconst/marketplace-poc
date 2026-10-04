use std::io::IsTerminal;
use std::net::SocketAddr;

use anyhow::anyhow;
use api::AppState;
use clap::Parser;
use db::config::SettingsArgs;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// HTTP API server.
#[derive(Parser)]
struct Args {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,

    /// Address to listen on.
    #[arg(long, env = "API_BIND_ADDR", default_value = "0.0.0.0:3000")]
    bind_addr: SocketAddr,

    /// Maximum number of pooled database connections.
    #[arg(
        long,
        env = "DATABASE_MAX_CONNECTIONS",
        default_value_t = 10,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    database_max_connections: u32,

    /// Also serve the demo-only routes under `/api/dev`, such as the creator bot.
    #[arg(long, env = "DEV_TOOLS")]
    dev_tools: bool,

    #[command(flatten)]
    settings: SettingsArgs,
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
    let mut settings = args
        .settings
        .into_settings()
        .map_err(|problem| anyhow!("invalid settings: {problem}"))?;

    let pool = db::connect(&args.database_url, args.database_max_connections).await?;
    let listener = TcpListener::bind(args.bind_addr).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    if args.dev_tools {
        tracing::warn!(
            "serving the demo-only routes under /api/dev; bidding has no minimum length"
        );
        // So that a demo campaign can close minutes after publishing.
        settings.limits.min_bidding_hours = 0;
    }
    let state = AppState {
        pool: pool.clone(),
        settings,
        dev_tools: args.dev_tools,
    };
    let app = api::router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal()?)
        .await?;

    pool.close().await;
    tracing::info!("shut down");
    Ok(())
}

/// Installs SIGINT and SIGTERM handlers and resolves when either arrives.
fn shutdown_signal() -> std::io::Result<impl Future<Output = ()>> {
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
        }
        tracing::info!("shutdown signal received");
    })
}
