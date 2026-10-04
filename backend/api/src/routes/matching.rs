//! The creator's feed of matching campaigns, and the advertiser's estimate of what a draft's
//! budget buys. Both use the predicates in [`db::matching`], which documents the matching rule.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use axum::extract::State;
use chrono::{DateTime, Utc};
use db::config::Settings;
use db::matching::{
    ACCOUNT_COLUMNS, ACCOUNT_MEETS_TARGETING, AccountRow, CAMPAIGN_OPEN, CampaignTerms,
    push_matching_accounts, push_min_bid_fits, push_views_in,
};
use db::models::{Bid, Campaign, Platform};
use marketplace::{
    Candidate, Estimate, Match, PLANNED_GROUPS, Preferences, Rules, SizeEstimate, SizeGroup, Terms,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, QueryBuilder};

use super::campaigns::{CampaignRow, find_campaign};
use crate::error::ApiError;
use crate::extract::{Json, Path, Query};

/// The most accounts an estimate scores; larger pools are sampled down to about this many.
const SAMPLE_SIZE: i64 = 5_000;

/// The most matching accounts an estimate counts; past them it says only that there are more.
const MAX_SHOWN_ACCOUNTS: i64 = 100_000;

#[derive(Deserialize)]
pub struct FeedParams {
    /// Only campaigns that look for this genre, or for any genre.
    genre: Option<i16>,
    #[serde(default)]
    sort: FeedSort,
    /// Only campaigns whose suggested bid pays at least this after WePush's commission.
    min_payout_cents: Option<i64>,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FeedSort {
    #[default]
    Match,
    Payout,
    Deadline,
}

#[derive(Serialize)]
pub struct Feed {
    account: AccountRow,
    items: Vec<FeedItem>,
}

/// An open campaign, with the account's match and what it may bid under the frozen deal.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FeedItem {
    campaign: FeedCampaign,
    r#match: Match,
    min_bid_cents: i64,
    max_bid_cents: i64,
    suggested_bid_cents: i64,
    suggested_min_paid_views: i64,
    /// To preview the min paid views while a bid is typed. Bids and results carry the exact one.
    min_paid_views_per_euro: f64,
    /// The suggested bid's payout, used to sort and filter the feed. Not sent.
    #[serde(skip)]
    payout_cents: i64,
    /// The account's bid on the campaign, whatever its status.
    my_bid: Option<Bid>,
}

/// An open campaign as creators see it. Open campaigns are complete, so nothing is optional.
#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct FeedCampaign {
    id: i64,
    advertiser_id: i64,
    advertiser_name: String,
    title: String,
    briefing: String,
    platform: Platform,
    target_cpm_cents: i64,
    commission_bps: i64,
    bidding_deadline: DateTime<Utc>,
    submission_window_days: i16,
    country_codes: Vec<String>,
    language_codes: Vec<String>,
    genre_ids: Vec<i16>,
}

impl FeedCampaign {
    /// A campaign that lists no genre takes any.
    fn looks_for(&self, genre: i16) -> bool {
        self.genre_ids.is_empty() || self.genre_ids.contains(&genre)
    }
}

#[derive(sqlx::FromRow)]
struct FeedRow {
    #[sqlx(flatten)]
    campaign: FeedCampaign,
    #[sqlx(flatten)]
    campaign_terms: CampaignTerms,
}

impl FeedItem {
    fn new(row: FeedRow, account: &AccountRow) -> Self {
        let terms = &row.campaign_terms.terms;
        let view_score = account.view_score;
        let suggested_bid_cents = terms.suggested_bid_cents(view_score);
        Self {
            r#match: row.campaign_terms.score(account),
            min_bid_cents: terms.rules.min_bid_cents(view_score),
            max_bid_cents: terms.rules.max_bid_cents(view_score),
            suggested_bid_cents,
            suggested_min_paid_views: terms.rules.min_paid_views(view_score, suggested_bid_cents),
            min_paid_views_per_euro: terms.rules.min_paid_views_per_euro(view_score),
            payout_cents: terms.rules.payout_cents(suggested_bid_cents),
            campaign: row.campaign,
            my_bid: None,
        }
    }
}

/// The open campaigns the account matches. Not paged, as few campaigns are open at a time.
pub async fn feed(
    State(pool): State<PgPool>,
    Path(account_id): Path<i64>,
    Query(params): Query<FeedParams>,
) -> Result<Json<Feed>, ApiError> {
    let account = find_account(&pool, account_id).await?;
    let group = SizeGroup::of(account.view_score);
    if !PLANNED_GROUPS.contains(&group) {
        // A starter is too small for any campaign.
        let items = Vec::new();
        return Ok(Json(Feed { account, items }));
    }

    let rows = open_campaigns_in_reach(&pool, account_id, group).await?;
    let mut items: Vec<FeedItem> = rows
        .into_iter()
        // SQL checked the targeting and the size; the rest of the matching rule is checked here.
        .filter(|row| row.campaign_terms.terms.can_bid(account.view_score))
        .filter(|row| {
            params
                .genre
                .is_none_or(|genre| row.campaign.looks_for(genre))
        })
        .map(|row| FeedItem::new(row, &account))
        .filter(|item| {
            params
                .min_payout_cents
                .is_none_or(|min| item.payout_cents >= min)
        })
        .collect();
    attach_bids(&pool, account_id, &mut items).await?;
    sort_feed(&mut items, params.sort);
    Ok(Json(Feed { account, items }))
}

async fn find_account(pool: &PgPool, account_id: i64) -> Result<AccountRow, ApiError> {
    // A QueryBuilder only to splice in the shared columns: sqlx takes no other non-literal SQL.
    let mut query = QueryBuilder::new("SELECT ");
    query
        .push(ACCOUNT_COLUMNS)
        .push(" FROM platform_accounts pa JOIN creators cr ON cr.id = pa.creator_id WHERE pa.id = ")
        .push_bind(account_id);
    query
        .build_query_as()
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Platform account", account_id))
}

/// The open campaigns whose targeting the account meets and whose sizes include its own:
///
/// ```sql
/// SELECT c.*, a.name AS advertiser_name
/// FROM campaign_details c
/// JOIN advertisers a ON a.id = c.advertiser_id
/// JOIN platform_accounts pa ON pa.id = $account_id  -- one row, for the targeting predicate
/// WHERE <CAMPAIGN_OPEN> AND <ACCOUNT_MEETS_TARGETING> AND $group = ANY (c.size_groups)
/// ```
///
/// Whether the account can bid, the filters and the order are left to Rust: they need its prices.
async fn open_campaigns_in_reach(
    pool: &PgPool,
    account_id: i64,
    group: SizeGroup,
) -> sqlx::Result<Vec<FeedRow>> {
    let mut query = QueryBuilder::new(
        "SELECT c.*, a.name AS advertiser_name
         FROM campaign_details c
         JOIN advertisers a ON a.id = c.advertiser_id
         JOIN platform_accounts pa ON pa.id = ",
    );
    query
        .push_bind(account_id)
        .push(" WHERE ")
        .push(CAMPAIGN_OPEN)
        .push(" AND ")
        .push(ACCOUNT_MEETS_TARGETING)
        .push(" AND ")
        .push_bind(group)
        .push(" = ANY (c.size_groups)");
    query.build_query_as().fetch_all(pool).await
}

async fn attach_bids(pool: &PgPool, account_id: i64, items: &mut [FeedItem]) -> sqlx::Result<()> {
    let campaign_ids: Vec<i64> = items.iter().map(|item| item.campaign.id).collect();
    let bids: Vec<Bid> = sqlx::query_as(
        "SELECT * FROM bids WHERE platform_account_id = $1 AND campaign_id = ANY ($2)",
    )
    .bind(account_id)
    .bind(&campaign_ids)
    .fetch_all(pool)
    .await?;
    let mut bids: HashMap<i64, Bid> = bids.into_iter().map(|bid| (bid.campaign_id, bid)).collect();
    for item in items {
        item.my_bid = bids.remove(&item.campaign.id);
    }
    Ok(())
}

fn sort_feed(items: &mut [FeedItem], sort: FeedSort) {
    match sort {
        FeedSort::Match => items.sort_by_key(|item| {
            (
                Reverse(item.r#match.score),
                item.campaign.bidding_deadline,
                item.campaign.id,
            )
        }),
        FeedSort::Payout => {
            items.sort_by_key(|item| (Reverse(item.payout_cents), item.campaign.id));
        }
        FeedSort::Deadline => {
            items.sort_by_key(|item| (item.campaign.bidding_deadline, item.campaign.id));
        }
    }
}

/// What a draft can expect, as far as its fields allow. Counting accounts needs the platform;
/// what the budget buys also needs the budget and the target CPM.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignEstimate {
    /// The fields the estimate needs but the campaign lacks, by their names in requests.
    missing: Vec<&'static str>,
    /// Matching accounts of the picked sizes, rounded, and at most [`MAX_SHOWN_ACCOUNTS`].
    matching_accounts: Option<i64>,
    matching_accounts_capped: bool,
    sizes: Vec<SizeEstimate>,
    // What the budget buys, from `marketplace::estimate`. The five fields are null together:
    // until the campaign has a platform, a budget and a target CPM, while no account matches,
    // or if no likely bidder wins. They are kept flat, as the editor shows them side by side.
    videos: Option<i64>,
    views: Option<i64>,
    average_cpm_cents: Option<i64>,
    spent_cents: Option<i64>,
    fills_budget: Option<bool>,
}

/// What the draft can expect from the accounts that match it now, priced with the terms
/// publishing would freeze now, as a draft has none of its own.
pub async fn estimate(
    State(pool): State<PgPool>,
    State(settings): State<Settings>,
    Path(campaign_id): Path<i64>,
) -> Result<Json<CampaignEstimate>, ApiError> {
    let row = find_campaign(&pool, campaign_id).await?;
    let campaign = &row.campaign;
    let terms = current_terms(campaign, &settings);
    let counts = match campaign.platform {
        Some(_) => Some(
            count_matching_by_size(&pool, campaign_id, &settings.rules, campaign.budget_cents)
                .await?,
        ),
        None => None,
    };
    // The sizes do not overlap, so the matching accounts are those of the picked sizes. Each size
    // was counted to one past the cap, so a total past the cap means more accounts match.
    let counted = counts.as_ref().map(|counts| {
        counts
            .iter()
            .filter(|(group, _)| campaign.size_groups.contains(group))
            .map(|(_, count)| count)
            .sum::<i64>()
    });
    let matching = counted.map(|count| count.min(MAX_SHOWN_ACCOUNTS));
    let bought = match (&terms, matching) {
        (Some(terms), Some(matching @ 1..)) => {
            what_the_budget_buys(&pool, &row, terms, matching).await?
        }
        _ => None,
    };

    Ok(Json(CampaignEstimate {
        missing: [
            ("platform", campaign.platform.is_none()),
            ("budgetCents", campaign.budget_cents.is_none()),
            ("targetCpmCents", campaign.target_cpm_cents.is_none()),
        ]
        .into_iter()
        .filter_map(|(field, missing)| missing.then_some(field))
        .collect(),
        matching_accounts: matching.map(marketplace::round_to_two_figures),
        matching_accounts_capped: counted.is_some_and(|count| count > MAX_SHOWN_ACCOUNTS),
        sizes: marketplace::sizes(&campaign.size_groups, terms.as_ref(), counts.as_ref()),
        videos: bought.map(|bought| bought.videos),
        views: bought.map(|bought| bought.views),
        average_cpm_cents: bought.map(|bought| bought.average_cpm_cents),
        spent_cents: bought.map(|bought| bought.spent_cents),
        fills_budget: bought.map(|bought| bought.fills_budget),
    }))
}

/// The terms publishing would freeze now; `None` without a budget, a target CPM or a planned size.
fn current_terms(campaign: &Campaign, settings: &Settings) -> Option<Terms> {
    settings.terms(
        campaign.budget_cents?,
        campaign.target_cpm_cents?,
        &campaign.size_groups,
    )
}

/// How many accounts of each planned size meet the campaign's targeting and, given a budget, have
/// a min bid that fits it. Each count stops one past [`MAX_SHOWN_ACCOUNTS`], enough to say there
/// are more, so that a big pool costs no more than the cap. Per size:
///
/// ```sql
/// SELECT count(*) FROM (
///     SELECT 1 FROM platform_accounts pa
///     JOIN campaigns c ON c.id = $campaign_id  -- one row, for the targeting predicate
///     WHERE <ACCOUNT_MEETS_TARGETING> AND <views in the size> [AND <min bid fits the budget>]
///     LIMIT $max_shown_accounts + 1
/// ) matching
/// ```
async fn count_matching_by_size(
    pool: &PgPool,
    campaign_id: i64,
    rules: &Rules,
    budget_cents: Option<i64>,
) -> sqlx::Result<BTreeMap<SizeGroup, i64>> {
    let mut counts = BTreeMap::new();
    for group in PLANNED_GROUPS {
        let mut query = QueryBuilder::new(
            "SELECT count(*) FROM (
                 SELECT 1 FROM platform_accounts pa JOIN campaigns c ON c.id = ",
        );
        query
            .push_bind(campaign_id)
            .push(" WHERE ")
            .push(ACCOUNT_MEETS_TARGETING)
            .push(" AND ");
        push_views_in(&mut query, group);
        if let Some(budget_cents) = budget_cents {
            query.push(" AND ");
            push_min_bid_fits(&mut query, rules, budget_cents);
        }
        query
            .push(" LIMIT ")
            .push_bind(MAX_SHOWN_ACCOUNTS + 1)
            .push(") matching");
        counts.insert(group, query.build_query_scalar().fetch_one(pool).await?);
    }
    Ok(counts)
}

/// Runs [`marketplace::estimate`] on a sample of the `matching` accounts, scored for the campaign.
async fn what_the_budget_buys(
    pool: &PgPool,
    row: &CampaignRow,
    terms: &Terms,
    matching: i64,
) -> sqlx::Result<Option<Estimate>> {
    let campaign = &row.campaign;
    let preferences = Preferences {
        engagement_weight: campaign.engagement_weight,
        quality_weight: campaign.quality_weight,
        reliability_weight: campaign.reliability_weight,
        genre_ids: &row.genre_ids,
    };
    let sample = sample_matching(pool, campaign.id, terms, matching).await?;
    let candidates: Vec<Candidate> = sample
        .iter()
        .map(|account| Candidate {
            view_score: account.view_score,
            match_score: marketplace::score(&account.as_account(), &preferences).score,
            reliability_score: account.reliability_score,
        })
        .collect();
    Ok(marketplace::estimate(&candidates, terms, matching))
}

/// About [`SAMPLE_SIZE`] of the `matching` accounts: those whose id is a multiple of a stride.
/// A fixed sample keeps the estimate from jumping on each autosave, and the stride spreads it
/// over all ids instead of taking the oldest accounts. Past the count cap the stride stops
/// growing, and the sample is the lowest such ids. The query still visits every matching
/// account; sampling bounds only the scoring work.
async fn sample_matching(
    pool: &PgPool,
    campaign_id: i64,
    terms: &Terms,
    matching: i64,
) -> sqlx::Result<Vec<AccountRow>> {
    // Rounded up, so that there are at most about SAMPLE_SIZE multiples of it.
    let stride = (matching + SAMPLE_SIZE - 1) / SAMPLE_SIZE;
    let mut query = QueryBuilder::new("SELECT ");
    query.push(ACCOUNT_COLUMNS);
    push_matching_accounts(&mut query, campaign_id, terms);
    query
        .push(" AND pa.id % ")
        .push_bind(stride)
        .push(" = 0 ORDER BY pa.id LIMIT ")
        .push_bind(SAMPLE_SIZE);
    query.build_query_as().fetch_all(pool).await
}
