//! How an active campaign is doing: what its pending bids would buy if bidding closed now, by
//! running [`marketplace::select_winners`] on them as closing will.

use axum::extract::State;
use db::matching::CAMPAIGN_TERMS_COLUMNS;
use marketplace::{Outcome, SizeGroup, Terms, cpm_cents};
use serde::Serialize;
use sqlx::{PgPool, QueryBuilder};

use super::campaigns::missing_or_conflict;
use crate::error::ApiError;
use crate::extract::{Json, Path};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignProgress {
    pending_bids: usize,
    budget_cents: i64,
    /// What the winning bids add up to.
    spent_cents: i64,
    /// The budget no size group would spend.
    returned_cents: i64,
    winners: usize,
    /// The sum of the winners' min paid views: what their videos must reach for all of them to
    /// be paid.
    min_paid_views: i64,
    /// The sum of the winners' view scores, as frozen on their bids.
    expected_views: i64,
    /// What the winners cost per 1,000 of their views; `None` without winners.
    effective_cpm_cents: Option<i64>,
    target_cpm_cents: i64,
    /// One for each picked size group, smallest first.
    groups: Vec<GroupProgress>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GroupProgress {
    group: SizeGroup,
    budget_cents: i64,
    /// May exceed the group's budget, as a group also spends what the bigger one before it left.
    spent_cents: i64,
    winners: usize,
    /// The pending bids from accounts in the group.
    bids: usize,
}

#[derive(sqlx::FromRow)]
struct ActiveCampaign {
    target_cpm_cents: i64,
    #[sqlx(flatten)]
    terms: Terms,
}

pub async fn show(
    State(pool): State<PgPool>,
    Path(campaign_id): Path<i64>,
) -> Result<Json<CampaignProgress>, ApiError> {
    // One snapshot for both reads: otherwise a closing that commits between them would show an
    // active campaign with no pending bids.
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .await?;
    let mut query = QueryBuilder::new("SELECT c.target_cpm_cents, ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(" FROM campaigns c WHERE c.status = 'active' AND c.id = ")
        .push_bind(campaign_id);
    let active: Option<ActiveCampaign> = query.build_query_as().fetch_optional(&mut *tx).await?;
    let Some(campaign) = active else {
        let conflict = format!("Campaign {campaign_id} is not active.");
        return Err(missing_or_conflict(&mut *tx, campaign_id, conflict).await);
    };
    let bids = db::selection::pending_bids(&mut *tx, campaign_id).await?;
    tx.commit().await?;

    let terms = &campaign.terms;
    let selection = marketplace::select_winners(terms, &bids);
    let spent_cents = selection.spent_cents();
    let min_paid_views: i64 = bids
        .iter()
        .zip(&selection.outcomes)
        .filter(|&(_, &outcome)| outcome == Outcome::Won)
        .map(|(bid, _)| terms.rules.min_paid_views(bid.view_score, bid.amount_cents))
        .sum();
    let expected_views = selection.expected_views();
    let groups = selection
        .groups
        .iter()
        .map(|result| GroupProgress {
            group: result.group,
            budget_cents: result.budget_cents,
            spent_cents: result.spent_cents,
            winners: result.winners,
            bids: bids
                .iter()
                .filter(|bid| SizeGroup::of(bid.view_score) == result.group)
                .count(),
        })
        .collect();

    Ok(Json(CampaignProgress {
        pending_bids: bids.len(),
        budget_cents: terms.budget_cents,
        spent_cents,
        returned_cents: selection.returned_cents,
        winners: selection.winner_count(),
        min_paid_views,
        expected_views,
        effective_cpm_cents: cpm_cents(spent_cents, expected_views),
        target_cpm_cents: campaign.target_cpm_cents,
        groups,
    }))
}
