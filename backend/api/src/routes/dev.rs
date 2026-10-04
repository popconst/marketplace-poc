//! Demo-only routes, served with `DEV_TOOLS`: bots that bid on open campaigns, so that a demo has
//! bids to show, and closing a campaign now or in a few seconds instead of at its deadline.

use axum::Router;
use axum::extract::State;
use axum::routing::post;
use chrono::{DateTime, Utc};
use db::closing::Closing;
use db::matching::{
    ACCOUNT_COLUMNS, AccountRow, CAMPAIGN_OPEN, CAMPAIGN_PREFERENCES_COLUMNS,
    CAMPAIGN_TERMS_COLUMNS, CampaignTerms, push_matching_accounts,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, QueryBuilder};

use super::bids::place_bid;
use super::campaigns::missing_or_conflict;
use crate::error::ApiError;
use crate::extract::{Json, Path};
use crate::state::AppState;

const DEFAULT_BIDS_PER_CAMPAIGN: u16 = 20;
const MAX_BIDS_PER_CAMPAIGN: u16 = 200;
/// Accounts sampled per bid wanted, as some decline to bid and some are refused.
const CANDIDATES_PER_BID: i64 = 3;
/// Bots bid 85% to 130% of their suggested bid, in basis points.
const MIN_BID_FACTOR_BPS: i64 = 8_500;
const MAX_BID_FACTOR_BPS: i64 = 13_000;
/// How far "close soon" moves a campaign's deadline: long enough to watch, short enough to wait.
const CLOSE_SOON_SECS: i32 = 10;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/simulate-bids", post(simulate_bids))
        .route("/campaigns/{campaign_id}/close", post(close_campaign))
        .route("/campaigns/{campaign_id}/close-soon", post(close_soon))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimulationParams {
    /// The most bids to place on each campaign, capped at [`MAX_BIDS_PER_CAMPAIGN`].
    bids_per_campaign: Option<u16>,
}

#[derive(Serialize)]
struct Simulation {
    /// How many open campaigns the bots bid on.
    campaigns: usize,
    placed: usize,
    /// Bids that a rule refused.
    skipped: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NewDeadline {
    bidding_deadline: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct OpenCampaign {
    id: i64,
    #[sqlx(flatten)]
    campaign_terms: CampaignTerms,
}

#[derive(sqlx::FromRow)]
struct Candidate {
    id: i64,
    #[sqlx(flatten)]
    account: AccountRow,
}

/// One round of bot bidding. On each open campaign, a random sample of matching accounts that
/// have not bid yet decide by [`choose_bid`], and each bid goes through [`place_bid`] like a real
/// one. Refused bids, mostly from a creator who already bid with another account, are skipped.
async fn simulate_bids(
    State(pool): State<PgPool>,
    params: Option<Json<SimulationParams>>,
) -> Result<Json<Simulation>, ApiError> {
    let params = params.map(|Json(params)| params).unwrap_or_default();
    let bids_per_campaign = params
        .bids_per_campaign
        .unwrap_or(DEFAULT_BIDS_PER_CAMPAIGN)
        .min(MAX_BIDS_PER_CAMPAIGN);
    let candidates_per_campaign = i64::from(bids_per_campaign) * CANDIDATES_PER_BID;

    let campaigns = open_campaigns(&pool).await?;
    let (mut placed, mut skipped) = (0, 0);
    for campaign in &campaigns {
        let mut placed_here = 0;
        for candidate in candidates(&pool, campaign, candidates_per_campaign).await? {
            if placed_here == bids_per_campaign {
                break;
            }
            let Some(amount_cents) = choose_bid(&campaign.campaign_terms, &candidate.account)
            else {
                continue;
            };
            match place_bid(&pool, campaign.id, candidate.id, amount_cents).await {
                Ok(_) => {
                    placed += 1;
                    placed_here += 1;
                }
                Err(error @ ApiError::Internal(_)) => return Err(error),
                Err(_) => skipped += 1,
            }
        }
    }
    Ok(Json(Simulation {
        campaigns: campaigns.len(),
        placed,
        skipped,
    }))
}

/// Every open campaign, with its terms and preferences.
async fn open_campaigns(pool: &PgPool) -> sqlx::Result<Vec<OpenCampaign>> {
    let mut query = QueryBuilder::new("SELECT c.id, ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(", ")
        .push(CAMPAIGN_PREFERENCES_COLUMNS)
        .push(" FROM campaigns c WHERE ")
        .push(CAMPAIGN_OPEN)
        .push(" ORDER BY c.id");
    query.build_query_as().fetch_all(pool).await
}

/// Up to `limit` random accounts that match the campaign and have not bid on it yet:
///
/// ```sql
/// SELECT pa.id, <ACCOUNT_COLUMNS>
/// <FROM, JOIN and WHERE of push_matching_accounts>
///   AND NOT EXISTS (SELECT 1 FROM bids b
///                   WHERE b.campaign_id = c.id AND b.platform_account_id = pa.id)
/// ORDER BY random()
/// LIMIT $limit
/// ```
///
/// `ORDER BY random()` sorts every matching account, which is fine for a demo.
async fn candidates(
    pool: &PgPool,
    campaign: &OpenCampaign,
    limit: i64,
) -> sqlx::Result<Vec<Candidate>> {
    let mut query = QueryBuilder::new("SELECT pa.id, ");
    query.push(ACCOUNT_COLUMNS);
    push_matching_accounts(&mut query, campaign.id, &campaign.campaign_terms.terms);
    query
        .push(
            " AND NOT EXISTS (SELECT 1 FROM bids b
                              WHERE b.campaign_id = c.id AND b.platform_account_id = pa.id)
              ORDER BY random() LIMIT ",
        )
        .push_bind(limit);
    query.build_query_as().fetch_all(pool).await
}

/// What the account bids, if anything. It bids with a chance of its match score in percent, a
/// random share of its suggested bid in whole euros, kept between its min and max bid.
fn choose_bid(campaign: &CampaignTerms, account: &AccountRow) -> Option<i64> {
    let terms = &campaign.terms;
    let view_score = account.view_score;
    let chance = f64::from(campaign.score(account).score) / 100.0;
    if !rand::random_bool(chance) {
        return None;
    }
    let factor_bps = rand::random_range(MIN_BID_FACTOR_BPS..=MAX_BID_FACTOR_BPS);
    let amount_cents = terms.suggested_bid_cents(view_score) * factor_bps / 10_000;
    // Rounded half up to whole euros, as people bid.
    let amount_cents = (amount_cents + 50) / 100 * 100;
    Some(amount_cents.clamp(
        terms.rules.min_bid_cents(view_score),
        terms.rules.max_bid_cents(view_score),
    ))
}

/// Closes an active campaign now, as the worker does once its deadline passes.
async fn close_campaign(
    State(pool): State<PgPool>,
    Path(campaign_id): Path<i64>,
) -> Result<Json<Closing>, ApiError> {
    // `close` changes nothing and returns `None` when the campaign is missing or not active.
    let Some(closing) = db::closing::close(&pool, campaign_id).await? else {
        let conflict = format!("Campaign {campaign_id} is not active.");
        return Err(missing_or_conflict(&pool, campaign_id, conflict).await);
    };
    Ok(Json(closing))
}

/// Moves an active campaign's bidding deadline to [`CLOSE_SOON_SECS`] from now, so that the worker
/// closes it as it would at a real deadline.
async fn close_soon(
    State(pool): State<PgPool>,
    Path(campaign_id): Path<i64>,
) -> Result<Json<NewDeadline>, ApiError> {
    // Waits for any bid in flight, which holds the campaign while it is written.
    let deadline: Option<DateTime<Utc>> = sqlx::query_scalar(
        "UPDATE campaigns
         SET bidding_deadline = now() + $2 * interval '1 second', updated_at = now()
         WHERE id = $1 AND status = 'active'
         RETURNING bidding_deadline",
    )
    .bind(campaign_id)
    .bind(CLOSE_SOON_SECS)
    .fetch_optional(&pool)
    .await?;
    let Some(bidding_deadline) = deadline else {
        let conflict = format!("Campaign {campaign_id} is not active.");
        return Err(missing_or_conflict(&pool, campaign_id, conflict).await);
    };
    Ok(Json(NewDeadline { bidding_deadline }))
}
