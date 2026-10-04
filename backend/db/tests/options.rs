mod common;

use common::{insert_account, insert_creator};
use db::models::Platform;
use db::options::{countries_in_use, languages_in_use};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn countries_need_a_brand_safe_account_and_languages_any_account(
    pool: PgPool,
) -> sqlx::Result<()> {
    assert!(countries_in_use(&pool).await?.is_empty());
    assert!(languages_in_use(&pool).await?.is_empty());

    let creator = insert_creator(&pool).await?;
    let german = insert_account(&pool, creator.id, Platform::TikTok, "lena.moves").await?;
    let korean = insert_account(&pool, creator.id, Platform::TikTok, "lena.seoul").await?;
    sqlx::query(
        "UPDATE platform_accounts SET country_code = 'KR', brand_safe = false WHERE id = $1",
    )
    .bind(korean.id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO platform_account_languages (platform_account_id, language_code)
         VALUES ($1, 'de'), ($1, 'en'), ($2, 'ko')",
    )
    .bind(german.id)
    .bind(korean.id)
    .execute(&pool)
    .await?;

    assert_eq!(
        serde_json::to_value(countries_in_use(&pool).await?).unwrap(),
        json!([{ "code": "DE", "name": "Germany" }])
    );
    assert_eq!(
        serde_json::to_value(languages_in_use(&pool).await?).unwrap(),
        json!([
            { "code": "en", "name": "English" },
            { "code": "de", "name": "German" },
            { "code": "ko", "name": "Korean" },
        ])
    );
    Ok(())
}
