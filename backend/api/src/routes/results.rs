//! What a closed campaign bought: its bids as [`db::closing`] decided them, the real version of
//! what [`super::progress`] projects while bidding is open.

use std::cmp::Reverse;

use axum::extract::State;
use chrono::{DateTime, Utc};
use db::matching::{
    ACCOUNT_COLUMNS, AccountRow, CAMPAIGN_PREFERENCES_COLUMNS, CAMPAIGN_TERMS_COLUMNS,
    CampaignTerms,
};
use db::models::LossReason;
use marketplace::{Match, SizeGroup, cpm_cents};
use serde::Serialize;
use sqlx::{PgPool, QueryBuilder};

use super::campaigns::missing_or_conflict;
use crate::error::ApiError;
use crate::extract::{Json, Path};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignResults {
    closed_at: DateTime<Utc>,
    budget_cents: i64,
    spent_cents: i64,
    /// The budget no winning bid took.
    returned_cents: i64,
    /// The sum of the winners' min paid views.
    min_paid_views: i64,
    expected_views: i64,
    /// `None` without winners.
    effective_cpm_cents: Option<i64>,
    target_cpm_cents: i64,
    /// Smallest size group first, then the highest amount.
    winners: Vec<Winner>,
    /// By loss reason, in the order [`LossReason`] declares them, then the highest amount.
    /// Withdrawn bids are neither winners nor losers.
    losers: Vec<Loser>,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct BidAccount {
    account_id: i64,
    handle: String,
    creator_name: String,
    country_code: String,
}

#[derive(sqlx::FromRow)]
struct DecidedBid {
    #[sqlx(flatten)]
    account: BidAccount,
    /// The account's stats as they are now.
    #[sqlx(flatten)]
    stats: AccountRow,
    /// The bid's, as it was placed or last edited.
    #[sqlx(rename = "bid_view_score")]
    view_score: i32,
    match_score: i16,
    amount_cents: i64,
    /// Set on lost bids only, by the bids table's check.
    loss_reason: Option<LossReason>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Winner {
    #[serde(flatten)]
    account: BidAccount,
    followers: i32,
    engagement_rate: f64,
    /// The views the bid was priced on: the account's views per post when it bid.
    view_score: i32,
    r#match: Match,
    /// What the advertiser pays.
    amount_cents: i64,
    /// By the bid's view score, as winner selection grouped it.
    size_group: SizeGroup,
    /// The views the video must reach for the creator to be paid.
    min_paid_views: i64,
    /// What the amount costs per 1,000 of the bid's views.
    expected_cpm_cents: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Loser {
    #[serde(flatten)]
    account: BidAccount,
    view_score: i32,
    r#match: Match,
    amount_cents: i64,
    loss_reason: LossReason,
    size_group: SizeGroup,
    expected_cpm_cents: i64,
}

impl DecidedBid {
    /// The stored score winner selection used, with factors the feed computes from today's stats.
    fn match_for(&self, campaign: &CampaignTerms) -> Match {
        Match {
            score: self.match_score,
            ..campaign.score(&self.stats)
        }
    }
}

impl Winner {
    fn new(bid: DecidedBid, campaign: &CampaignTerms) -> Self {
        let rules = &campaign.terms.rules;
        Self {
            r#match: bid.match_for(campaign),
            size_group: SizeGroup::of(bid.view_score),
            min_paid_views: rules.min_paid_views(bid.view_score, bid.amount_cents),
            expected_cpm_cents: bid_cpm_cents(bid.amount_cents, bid.view_score),
            account: bid.account,
            followers: bid.stats.followers,
            engagement_rate: bid.stats.engagement_rate,
            view_score: bid.view_score,
            amount_cents: bid.amount_cents,
        }
    }
}

impl Loser {
    fn new(bid: DecidedBid, loss_reason: LossReason, campaign: &CampaignTerms) -> Self {
        Self {
            r#match: bid.match_for(campaign),
            size_group: SizeGroup::of(bid.view_score),
            expected_cpm_cents: bid_cpm_cents(bid.amount_cents, bid.view_score),
            account: bid.account,
            view_score: bid.view_score,
            amount_cents: bid.amount_cents,
            loss_reason,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ClosedCampaign {
    closed_at: DateTime<Utc>,
    target_cpm_cents: i64,
    #[sqlx(flatten)]
    campaign_terms: CampaignTerms,
}

pub async fn show(
    State(pool): State<PgPool>,
    Path(campaign_id): Path<i64>,
) -> Result<Json<CampaignResults>, ApiError> {
    let mut query = QueryBuilder::new("SELECT c.closed_at, c.target_cpm_cents, ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(", ")
        .push(CAMPAIGN_PREFERENCES_COLUMNS)
        .push(" FROM campaigns c WHERE c.status = 'closed' AND c.id = ")
        .push_bind(campaign_id);
    let closed: Option<ClosedCampaign> = query.build_query_as().fetch_optional(&pool).await?;
    let Some(campaign) = closed else {
        let conflict = format!("Campaign {campaign_id} is not closed.");
        return Err(missing_or_conflict(&pool, campaign_id, conflict).await);
    };
    // A closed campaign and its decided bids never change, so the two reads need no shared
    // snapshot. Postgres sorts an enum in declaration order, which orders the losers by reason;
    // winners are sorted below, by size, which is not a column.
    let mut query = QueryBuilder::new(
        "SELECT pa.id AS account_id, pa.handle, cr.name AS creator_name, pa.country_code,
             b.view_score AS bid_view_score, b.match_score, b.amount_cents, b.loss_reason, ",
    );
    query
        .push(ACCOUNT_COLUMNS)
        .push(
            " FROM bids b
              JOIN platform_accounts pa ON pa.id = b.platform_account_id
              JOIN creators cr ON cr.id = pa.creator_id
              WHERE b.status IN ('won', 'lost') AND b.campaign_id = ",
        )
        .push_bind(campaign_id)
        .push(" ORDER BY b.loss_reason, b.amount_cents DESC, pa.id");
    let bids: Vec<DecidedBid> = query.build_query_as().fetch_all(&pool).await?;

    let campaign_terms = &campaign.campaign_terms;
    let terms = &campaign_terms.terms;
    let mut winners = Vec::new();
    let mut losers = Vec::new();
    for bid in bids {
        match bid.loss_reason {
            None => winners.push(Winner::new(bid, campaign_terms)),
            Some(reason) => losers.push(Loser::new(bid, reason, campaign_terms)),
        }
    }
    winners.sort_by_key(|w| (w.size_group, Reverse(w.amount_cents), w.account.account_id));
    let spent_cents: i64 = winners.iter().map(|w| w.amount_cents).sum();
    let min_paid_views: i64 = winners.iter().map(|w| w.min_paid_views).sum();
    let expected_views: i64 = winners.iter().map(|w| i64::from(w.view_score)).sum();

    Ok(Json(CampaignResults {
        closed_at: campaign.closed_at,
        budget_cents: terms.budget_cents,
        spent_cents,
        returned_cents: terms.budget_cents - spent_cents,
        min_paid_views,
        expected_views,
        effective_cpm_cents: cpm_cents(spent_cents, expected_views),
        target_cpm_cents: campaign.target_cpm_cents,
        winners,
        losers,
    }))
}

/// A bid's view score is positive, by the bids table's check, so its CPM always exists.
fn bid_cpm_cents(amount_cents: i64, view_score: i32) -> i64 {
    cpm_cents(amount_cents, i64::from(view_score)).unwrap_or_default()
}
