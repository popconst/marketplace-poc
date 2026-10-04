// Each test file compiles this module separately and uses a different subset of it.
#![allow(dead_code)]

use chrono::{TimeDelta, Utc};
use db::config::Settings;
use db::models::{Bid, Campaign, Creator, Platform, PlatformAccount};
use sqlx::PgPool;

/// The view score of accounts from [`insert_account`]: a micro account.
pub const VIEWS: i32 = 12_500;
/// The budget of campaigns from [`insert_campaign`]: €10,000.
pub const BUDGET_CENTS: i64 = 1_000_000;

pub fn violated_constraint(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}

/// Runs an `UPDATE` on row `id` that must fail, and returns the violated constraint.
pub async fn update_fails_with(
    pool: &PgPool,
    table: &str,
    id: i64,
    assignments: &str,
) -> Option<String> {
    let error = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE {table} SET {assignments} WHERE id = $1"
    )))
    .bind(id)
    .execute(pool)
    .await
    .expect_err(assignments);
    violated_constraint(&error).map(str::to_owned)
}

pub async fn insert_creator(pool: &PgPool) -> sqlx::Result<Creator> {
    sqlx::query_as(
        "INSERT INTO creators (name, reliability_score)
         VALUES ('Lena Vogt', 88)
         RETURNING *",
    )
    .fetch_one(pool)
    .await
}

/// Inserts a brand-safe account based in Germany, with [`VIEWS`].
pub async fn insert_account(
    pool: &PgPool,
    creator_id: i64,
    platform: Platform,
    handle: &str,
) -> sqlx::Result<PlatformAccount> {
    sqlx::query_as(
        "INSERT INTO platform_accounts
             (creator_id, platform, handle, country_code, followers, view_score, engagement_rate,
              posts_per_week, quality_score, brand_safe, stats_updated_at)
         VALUES ($1, $2, $3, 'DE', 48200, $4, 0.064, 4.5, 72, true, '2026-09-28T10:00:00Z')
         RETURNING *",
    )
    .bind(creator_id)
    .bind(platform)
    .bind(handle)
    .bind(VIEWS)
    .fetch_one(pool)
    .await
}

/// Inserts an empty draft for a new advertiser, with only the default posting window.
pub async fn insert_draft(pool: &PgPool) -> sqlx::Result<Campaign> {
    let advertiser_id: i64 =
        sqlx::query_scalar("INSERT INTO advertisers (name) VALUES ('Glow Labs') RETURNING id")
            .fetch_one(pool)
            .await?;
    sqlx::query_as(
        "INSERT INTO campaigns (advertiser_id, submission_window_days)
         VALUES ($1, 3)
         RETURNING *",
    )
    .bind(advertiser_id)
    .fetch_one(pool)
    .await
}

/// Inserts a complete draft on `tiktok` with a [`BUDGET_CENTS`] budget, a €12 target CPM and the
/// default sizes.
pub async fn insert_complete_draft(pool: &PgPool) -> sqlx::Result<Campaign> {
    let draft = insert_draft(pool).await?;
    sqlx::query_as(
        "UPDATE campaigns SET
             title = 'Summer serum launch', briefing = 'Show your morning routine with the serum.',
             platform = 'tiktok', budget_cents = $2, target_cpm_cents = 1200
         WHERE id = $1
         RETURNING *",
    )
    .bind(draft.id)
    .bind(BUDGET_CENTS)
    .fetch_one(pool)
    .await
}

/// Inserts a campaign published a day ago with today's settings, bidding closing in three days.
pub async fn insert_campaign(pool: &PgPool) -> sqlx::Result<Campaign> {
    insert_campaign_closing_in(pool, TimeDelta::days(3)).await
}

/// Like [`insert_campaign`], with bidding closing `closing_in` from now. It may be negative, but
/// not a day or more, as the campaign was published a day ago.
pub async fn insert_campaign_closing_in(
    pool: &PgPool,
    closing_in: TimeDelta,
) -> sqlx::Result<Campaign> {
    let draft = insert_complete_draft(pool).await?;
    let settings = Settings::default();
    let offer_bps = settings
        .offer_bps(draft.target_cpm_cents.unwrap(), &draft.size_groups)
        .unwrap();
    let now = Utc::now();
    let published_at = now - TimeDelta::days(1);
    db::campaigns::publish(
        pool,
        draft.id,
        now + closing_in,
        published_at,
        &settings.rules,
        offer_bps,
    )
    .await
}

/// [`Default`] is a €150 `tiktok` bid from a micro account matching at 75.
#[derive(Clone, Copy)]
pub struct BidSpec {
    pub platform: Platform,
    pub amount_cents: i64,
    pub view_score: i32,
    pub match_score: i16,
}

impl Default for BidSpec {
    fn default() -> Self {
        Self {
            platform: Platform::TikTok,
            amount_cents: 15_000,
            view_score: VIEWS,
            match_score: 75,
        }
    }
}

/// Inserts the account's pending bid on the campaign, with its creator's reliability of 88.
pub async fn insert_bid(
    pool: &PgPool,
    campaign_id: i64,
    account_id: i64,
    bid: BidSpec,
) -> sqlx::Result<Bid> {
    sqlx::query_as(
        "INSERT INTO bids
             (campaign_id, platform_account_id, platform, amount_cents, view_score,
              reliability_score, match_score)
         VALUES ($1, $2, $3, $4, $5, 88, $6)
         RETURNING *",
    )
    .bind(campaign_id)
    .bind(account_id)
    .bind(bid.platform)
    .bind(bid.amount_cents)
    .bind(bid.view_score)
    .bind(bid.match_score)
    .fetch_one(pool)
    .await
}
