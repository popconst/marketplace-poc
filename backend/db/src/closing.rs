//! Closing a campaign: deciding its pending bids with [`marketplace::select_winners`], exactly
//! once. The worker closes due campaigns and records failed attempts.

use chrono::{DateTime, Utc};
use marketplace::{Outcome, PendingBid, Selection, Terms};
use serde::Serialize;
use sqlx::{PgConnection, PgPool, QueryBuilder};

use crate::matching::CAMPAIGN_TERMS_COLUMNS;
use crate::models::{BidStatus, CampaignStatus};
use crate::selection;

/// What closing a campaign decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Closing {
    pub winners: usize,
    pub losers: usize,
    pub spent_cents: i64,
    pub returned_cents: i64,
}

/// Closes an active campaign in one transaction: decides its pending bids with
/// [`marketplace::select_winners`] and marks it closed. Returns `None` and changes nothing if the
/// campaign is missing or not active, so a repeated or concurrent close does nothing. It does not
/// check the deadline, as the demo's debug button closes campaigns early on purpose.
///
/// # Errors
///
/// Returns an error if a query fails, in which case nothing has changed.
pub async fn close(pool: &PgPool, campaign_id: i64) -> sqlx::Result<Option<Closing>> {
    let mut tx = pool.begin().await?;
    let Some(terms) = lock_active_campaign(&mut tx, campaign_id).await? else {
        return Ok(None);
    };
    let bids = selection::pending_bids(&mut *tx, campaign_id).await?;
    let decision = marketplace::select_winners(&terms, &bids);
    record_decision(&mut tx, &bids, &decision).await?;
    // `now()` is the transaction's start time, so the bids and the campaign get the same time.
    sqlx::query(
        "UPDATE campaigns SET status = 'closed', closed_at = now(), updated_at = now()
         WHERE id = $1",
    )
    .bind(campaign_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let winners = decision.winner_count();
    Ok(Some(Closing {
        winners,
        losers: bids.len() - winners,
        spent_cents: decision.spent_cents(),
        returned_cents: decision.returned_cents,
    }))
}

/// Locks the campaign for the rest of the transaction and returns its terms, or `None` if it is
/// missing or not active. Placing or withdrawing a bid holds the campaign `FOR SHARE`, so this
/// waits for bids in flight, and later bids find it closed. A second closer also waits, then finds
/// the campaign no longer active, as Postgres re-checks the `WHERE` on the committed row.
async fn lock_active_campaign(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Terms>> {
    let mut query = QueryBuilder::new("SELECT ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(" FROM campaigns c WHERE c.status = 'active' AND c.id = ")
        .push_bind(id)
        .push(" FOR UPDATE");
    query.build_query_as().fetch_optional(conn).await
}

/// Writes every bid's outcome in one statement: `UNNEST` zips the three per-bid arrays into rows
/// of (id, status, loss reason).
async fn record_decision(
    conn: &mut PgConnection,
    bids: &[PendingBid],
    decision: &Selection,
) -> sqlx::Result<()> {
    let mut ids = Vec::with_capacity(bids.len());
    let mut statuses = Vec::with_capacity(bids.len());
    let mut loss_reasons = Vec::with_capacity(bids.len());
    for (bid, &outcome) in bids.iter().zip(&decision.outcomes) {
        let (status, loss_reason) = match outcome {
            Outcome::Won => (BidStatus::Won, None),
            Outcome::Lost(reason) => (BidStatus::Lost, Some(reason)),
        };
        ids.push(bid.id);
        statuses.push(status);
        loss_reasons.push(loss_reason);
    }
    sqlx::query(
        "UPDATE bids b SET status = d.status, loss_reason = d.loss_reason, decided_at = now(),
             updated_at = now()
         FROM UNNEST($1::bigint[], $2::bid_status[], $3::loss_reason[])
             AS d (id, status, loss_reason)
         WHERE b.id = d.id",
    )
    .bind(&ids)
    .bind(&statuses)
    .bind(&loss_reasons)
    .execute(conn)
    .await?;
    Ok(())
}

/// The (deadline, id) of every active campaign whose bidding deadline is at or before `until`,
/// overdue ones included, soonest first. Served by the partial index `campaigns_closing_idx`.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn deadlines_until(
    pool: &PgPool,
    until: DateTime<Utc>,
) -> sqlx::Result<Vec<(DateTime<Utc>, i64)>> {
    sqlx::query_as(
        "SELECT bidding_deadline, id FROM campaigns
         WHERE status = 'active' AND bidding_deadline <= $1
         ORDER BY bidding_deadline, id",
    )
    .bind(until)
    .fetch_all(pool)
    .await
}

/// Records a failed close attempt; the one that reaches `max_attempts` marks the campaign failed.
/// Returns the new status, or `None` if the campaign is no longer active.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn record_close_failure(
    pool: &PgPool,
    campaign_id: i64,
    error: &str,
    max_attempts: i16,
) -> sqlx::Result<Option<CampaignStatus>> {
    // `SET` expressions read the old row, so `close_attempts + 1` includes this attempt.
    sqlx::query_scalar(
        "UPDATE campaigns SET
             close_attempts = close_attempts + 1,
             last_close_error = $2,
             status = CASE WHEN close_attempts + 1 >= $3 THEN 'failed' ELSE status END,
             updated_at = now()
         WHERE id = $1 AND status = 'active'
         RETURNING status",
    )
    .bind(campaign_id)
    .bind(error)
    .bind(max_attempts)
    .fetch_optional(pool)
    .await
}
