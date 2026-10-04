mod common;

use common::{BidSpec, insert_account, insert_bid, insert_campaign, insert_creator};
use db::models::Platform;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bid_platform_must_match_both_account_and_campaign(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool).await?;
    let instagram = insert_account(&pool, creator.id, Platform::Instagram, "lena.moves").await?;
    let tiktok_campaign = insert_campaign(&pool).await?;

    for (platform, constraint) in [
        (Platform::TikTok, "bids_account_platform_fkey"),
        (Platform::Instagram, "bids_campaign_platform_fkey"),
    ] {
        let bid = BidSpec {
            platform,
            ..BidSpec::default()
        };
        let error = insert_bid(&pool, tiktok_campaign.id, instagram.id, bid)
            .await
            .unwrap_err();
        assert_eq!(
            common::violated_constraint(&error),
            Some(constraint),
            "{platform:?}"
        );
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decided_bids_carry_a_time_and_lost_bids_a_reason(pool: PgPool) -> sqlx::Result<()> {
    let creator = insert_creator(&pool).await?;
    let account = insert_account(&pool, creator.id, Platform::TikTok, "lena.moves").await?;
    let campaign = insert_campaign(&pool).await?;
    let bid = insert_bid(&pool, campaign.id, account.id, BidSpec::default()).await?;

    for assignments in [
        "status = 'won'",
        "status = 'lost', decided_at = now()",
        "status = 'won', decided_at = now(), loss_reason = 'outranked'",
    ] {
        let violated = common::update_fails_with(&pool, "bids", bid.id, assignments).await;
        assert_eq!(violated.as_deref(), Some("bids_decision"), "{assignments}");
    }

    sqlx::query(
        "UPDATE bids SET status = 'lost', decided_at = now(), loss_reason = 'over_limit'
         WHERE id = $1",
    )
    .bind(bid.id)
    .execute(&pool)
    .await?;
    Ok(())
}
