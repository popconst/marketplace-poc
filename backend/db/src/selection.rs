//! Loads what [`marketplace::select_winners`] needs: a campaign's terms and pending bids. Closing
//! decides the bids with them; the progress view previews what closing would decide now.

use marketplace::{PendingBid, Terms};
use sqlx::{PgExecutor, QueryBuilder};

use crate::matching::CAMPAIGN_TERMS_COLUMNS;

/// The terms frozen on the campaign at publish, or `None` if it is missing or a draft.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn terms(executor: impl PgExecutor<'_>, campaign_id: i64) -> sqlx::Result<Option<Terms>> {
    let mut query = QueryBuilder::new("SELECT ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(" FROM campaigns c WHERE c.status <> 'draft' AND c.id = ")
        .push_bind(campaign_id);
    query.build_query_as().fetch_optional(executor).await
}

/// The campaign's pending bids, with the figures frozen when each was placed or last edited.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn pending_bids(
    executor: impl PgExecutor<'_>,
    campaign_id: i64,
) -> sqlx::Result<Vec<PendingBid>> {
    sqlx::query_as(
        "SELECT id, amount_cents, view_score, match_score, reliability_score
         FROM bids WHERE campaign_id = $1 AND status = 'pending'",
    )
    .bind(campaign_id)
    .fetch_all(executor)
    .await
}
