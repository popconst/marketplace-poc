//! Closes each campaign the moment its bidding deadline passes, with [`db::closing::close`],
//! which uses the deal rules frozen on the campaign, never the current settings.
//!
//! Every `WORKER_REFRESH_SECS` the worker loads the deadlines coming up into memory and sleeps
//! until the next one. The database stays the only record, so after a restart the worker loads
//! them again and closes overdue campaigns right away. Several workers are safe: `close` locks
//! the campaign and closes it only if it is still active.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::pin::pin;
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::Parser;
use db::closing;
use db::models::CampaignStatus;
use sqlx::PgPool;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::{self, MissedTickBehavior};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// (deadline, campaign id) pairs, soonest first.
type Deadlines = BTreeSet<(DateTime<Utc>, i64)>;

/// Background worker that closes campaigns once their bidding deadline passes.
#[derive(Parser)]
struct Args {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,

    /// Seconds between loads of the coming deadlines.
    #[arg(
        long,
        env = "WORKER_REFRESH_SECS",
        default_value_t = 30,
        value_parser = clap::value_parser!(u64).range(1..=3600)
    )]
    refresh_secs: u64,

    /// Failed attempts to close a campaign before the worker marks it failed and stops trying.
    #[arg(
        long,
        env = "CLOSE_MAX_ATTEMPTS",
        default_value_t = 5,
        value_parser = clap::value_parser!(i16).range(1..)
    )]
    close_max_attempts: i16,

    /// Maximum number of pooled database connections.
    #[arg(
        long,
        env = "DATABASE_MAX_CONNECTIONS",
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    database_max_connections: u32,
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

    let pool = db::connect(&args.database_url, args.database_max_connections).await?;
    let refresh = Duration::from_secs(args.refresh_secs);
    let result = run(&pool, refresh, args.close_max_attempts).await;
    pool.close().await;
    result
}

/// Reloads the coming deadlines every `refresh` and closes each campaign when its deadline
/// passes, until SIGINT or SIGTERM. A signal never interrupts a close.
async fn run(pool: &PgPool, refresh: Duration, close_max_attempts: i16) -> anyhow::Result<()> {
    let mut shutdown = pin!(shutdown_signal()?);
    let mut reload = time::interval(refresh);
    reload.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut deadlines = Deadlines::new();

    tracing::info!(?refresh, "worker started");
    loop {
        let next = deadlines.first().map(|&(deadline, _)| deadline);
        tokio::select! {
            biased;
            () = &mut shutdown => break,
            // The first tick fires at once, so a fresh start loads the deadlines straight away.
            _ = reload.tick() => match load_deadlines(pool, refresh).await {
                Ok(loaded) => deadlines = loaded,
                // The next reload tries again.
                Err(error) => tracing::error!(%error, "loading deadlines failed"),
            },
            () = sleep_until(next) => {
                if let Some((_, campaign_id)) = deadlines.pop_first() {
                    close_campaign(pool, campaign_id, close_max_attempts).await;
                }
            }
        }
    }
    tracing::info!("worker stopped");
    Ok(())
}

/// The active campaigns due within two refresh periods, overdue ones included. Two periods, so a
/// reload that runs late after a slow close still finds a deadline before it passes.
async fn load_deadlines(pool: &PgPool, refresh: Duration) -> sqlx::Result<Deadlines> {
    let due = closing::deadlines_until(pool, Utc::now() + 2 * refresh).await?;
    Ok(due.into_iter().collect())
}

/// Waits until `deadline`, at once if it has passed, or forever if there is none.
async fn sleep_until(deadline: Option<DateTime<Utc>>) {
    match deadline {
        Some(deadline) => {
            let wait = (deadline - Utc::now()).to_std().unwrap_or_default();
            time::sleep(wait).await;
        }
        None => std::future::pending().await,
    }
}

/// Closes the campaign. A failed close leaves it active, so the next reload picks it up again,
/// until it has failed `max_attempts` times.
async fn close_campaign(pool: &PgPool, campaign_id: i64, max_attempts: i16) {
    match closing::close(pool, campaign_id).await {
        Ok(Some(closed)) => tracing::info!(
            campaign_id,
            winners = closed.winners,
            losers = closed.losers,
            spent_cents = closed.spent_cents,
            returned_cents = closed.returned_cents,
            "campaign closed"
        ),
        // Closed early from the demo's Debug menu, or by another worker.
        Ok(None) => {}
        Err(error) => record_failure(pool, campaign_id, &error, max_attempts).await,
    }
}

async fn record_failure(pool: &PgPool, campaign_id: i64, error: &sqlx::Error, max_attempts: i16) {
    let recorded =
        closing::record_close_failure(pool, campaign_id, &error.to_string(), max_attempts).await;
    match recorded {
        Ok(Some(CampaignStatus::Failed)) => {
            tracing::error!(campaign_id, %error, "closing failed for the last time; marked failed");
        }
        Ok(Some(_)) => tracing::warn!(campaign_id, %error, "closing failed; retrying"),
        // Another worker closed it in the meantime.
        Ok(None) => {}
        // Not counted as an attempt, but the next reload still retries the close.
        Err(record_error) => {
            tracing::error!(campaign_id, %error, %record_error, "closing and recording it failed");
        }
    }
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
