//! Publishing a campaign, which freezes its deal. The API publishes drafts with it, and the
//! seeder its open campaigns.

use chrono::{DateTime, Utc};
use marketplace::Rules;
use sqlx::PgExecutor;

use crate::models::Campaign;

/// Publishes a draft: opens bidding until the deadline and freezes the deal rules and the offer
/// on the campaign. It goes straight to active, as this demo has no review step. `now` is the
/// clock the caller checked the deadline against, so the stored times agree with that check.
///
/// # Errors
///
/// Returns [`sqlx::Error::RowNotFound`] if the campaign is not a draft, or the database's error
/// if the draft is incomplete.
pub async fn publish(
    executor: impl PgExecutor<'_>,
    campaign_id: i64,
    bidding_deadline: DateTime<Utc>,
    now: DateTime<Utc>,
    rules: &Rules,
    offer_bps: i64,
) -> sqlx::Result<Campaign> {
    // Destructured so that a new rule fails to compile here until it is frozen too.
    let Rules {
        commission_bps,
        usual_rate_base_cents,
        usual_rate_per_1000_views_cents,
        fair_pay_floor_bps,
        max_bid_share_of_budget_bps,
        views_to_get_paid_bps,
        likely_views_limit_bps,
        discovery_bps,
    } = *rules;
    sqlx::query_as(
        "UPDATE campaigns SET
             status = 'active', bidding_deadline = $2,
             submitted_at = $3, activated_at = $3, updated_at = $3,
             offer_bps = $4, commission_bps = $5, usual_rate_base_cents = $6,
             usual_rate_per_1000_views_cents = $7, fair_pay_floor_bps = $8,
             max_bid_share_of_budget_bps = $9, views_to_get_paid_bps = $10,
             likely_views_limit_bps = $11, discovery_bps = $12
         WHERE id = $1 AND status = 'draft'
         RETURNING *",
    )
    .bind(campaign_id)
    .bind(bidding_deadline)
    .bind(now)
    .bind(offer_bps)
    .bind(commission_bps)
    .bind(usual_rate_base_cents)
    .bind(usual_rate_per_1000_views_cents)
    .bind(fair_pay_floor_bps)
    .bind(max_bid_share_of_budget_bps)
    .bind(views_to_get_paid_bps)
    .bind(likely_views_limit_bps)
    .bind(discovery_bps)
    .fetch_one(executor)
    .await
}
