//! The data layer shared by the API, the worker and the seeder: the connection pool, migrations,
//! settings, row types, and the SQL that more than one binary or route module runs. SQL that only
//! one route needs sits next to its handler in the API.
//!
//! Reading order: `config`, `models`, `matching`, `selection`, `campaigns`, then `closing`.

pub mod campaigns;
pub mod closing;
pub mod config;
pub mod matching;
pub mod models;
pub mod options;
pub mod selection;

use std::time::Duration;

use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

/// Migrations from `backend/migrations`, embedded at compile time.
pub static MIGRATOR: Migrator = sqlx::migrate!("../migrations");

/// Opens a connection pool and verifies the database is reachable.
///
/// # Errors
///
/// Returns an error if the URL is invalid or no connection is made within the acquire timeout.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        // sqlx waits 30s by default; failing fast lets callers report the database as down.
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await
}
