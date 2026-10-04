mod common;

use common::{insert_account, insert_creator};
use db::models::Platform;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_creator_can_hold_several_accounts_on_one_platform(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool).await?;

    let first = insert_account(&pool, creator.id, Platform::TikTok, "lena.moves").await?;
    let second = insert_account(&pool, creator.id, Platform::TikTok, "lena.cooks").await?;

    assert_eq!(
        (first.creator_id, second.creator_id),
        (creator.id, creator.id)
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn handles_are_unique_per_platform_regardless_of_case(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool).await?;
    insert_account(&pool, creator.id, Platform::TikTok, "lena.moves").await?;
    insert_account(&pool, creator.id, Platform::Instagram, "lena.moves").await?;

    let duplicate = insert_account(&pool, creator.id, Platform::TikTok, "Lena.Moves").await;

    assert_eq!(
        common::violated_constraint(&duplicate.unwrap_err()),
        Some("platform_accounts_platform_handle_key")
    );
    Ok(())
}
