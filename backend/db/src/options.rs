//! What a campaign can target: every genre, and the countries and languages of existing accounts.

use sqlx::PgPool;

use crate::models::{Country, Genre, Language};

/// Every genre, by id.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn genres(pool: &PgPool) -> sqlx::Result<Vec<Genre>> {
    sqlx::query_as("SELECT id, name FROM genres ORDER BY id")
        .fetch_all(pool)
        .await
}

/// Countries with at least one brand-safe account, by name. Only brand-safe accounts are
/// matched, so any other country would match nobody.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn countries_in_use(pool: &PgPool) -> sqlx::Result<Vec<Country>> {
    sqlx::query_as(
        "SELECT c.code, c.name FROM countries c
         WHERE EXISTS (
             SELECT 1 FROM platform_accounts pa
             WHERE pa.country_code = c.code AND pa.brand_safe
         )
         ORDER BY c.name",
    )
    .fetch_all(pool)
    .await
}

/// Languages with at least one account, brand-safe or not, by name. Requiring a brand-safe
/// account, as for countries, would join every account to its languages: too slow at millions.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn languages_in_use(pool: &PgPool) -> sqlx::Result<Vec<Language>> {
    sqlx::query_as(
        "SELECT l.code, l.name FROM languages l
         WHERE EXISTS (
             SELECT 1 FROM platform_account_languages pal WHERE pal.language_code = l.code
         )
         ORDER BY l.name",
    )
    .fetch_all(pool)
    .await
}
