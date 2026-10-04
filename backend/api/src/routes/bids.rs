//! Bids: an account's offer to post for an open campaign.

use axum::extract::State;
use chrono::{DateTime, Utc};
use db::matching::{
    ACCOUNT_COLUMNS, ACCOUNT_MEETS_TARGETING, AccountRow, CAMPAIGN_OPEN,
    CAMPAIGN_PREFERENCES_COLUMNS, CAMPAIGN_TERMS_COLUMNS, CampaignTerms,
};
use db::models::{Bid, BidStatus, CampaignStatus, Phase, Platform};
use marketplace::{Outcome, PendingBid, Rules, VIEWS_COUNTING_DAYS};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, QueryBuilder};

use super::campaigns::missing_or_conflict;
use crate::error::{ApiError, FieldErrors};
use crate::extract::{Json, Path, Query};
use crate::format::{euros, grouped};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidAmount {
    amount_cents: i64,
}

#[derive(Deserialize)]
pub struct ChanceParams {
    amount_cents: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chance {
    would_win: bool,
    /// The campaign's pending bids, this one included.
    pending_bids: usize,
}

/// Whether reading the campaign or the bidder locks it for the rest of the transaction, as
/// writing a bid must. A chance only reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lock {
    Yes,
    No,
}

#[derive(sqlx::FromRow)]
struct Bidder {
    id: i64,
    creator_id: i64,
    /// Copied onto the bid. Meeting the targeting makes it the campaign's platform too.
    platform: Platform,
    #[sqlx(flatten)]
    account: AccountRow,
    meets_targeting: bool,
}

#[derive(Deserialize)]
pub struct ListParams {
    status: Option<BidStatus>,
}

#[derive(Serialize)]
pub struct BidList {
    items: Vec<BidListItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BidListItem {
    #[serde(flatten)]
    bid: Bid,
    campaign: BidListCampaign,
    /// What the account receives if the bid wins: the amount less WePush's commission.
    payout_cents: i64,
    /// The views the video must reach for the account to be paid.
    min_paid_views: i64,
}

/// The bid's campaign, complete because it was open when the bid was placed.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BidListCampaign {
    id: i64,
    title: String,
    advertiser_name: String,
    platform: Platform,
    briefing: String,
    bidding_deadline: DateTime<Utc>,
    /// Winners must post within this many days of the bidding deadline.
    submission_window_days: i16,
    phase: Phase,
}

#[derive(sqlx::FromRow)]
struct BidListRow {
    #[sqlx(flatten)]
    bid: Bid,
    title: String,
    advertiser_name: String,
    briefing: String,
    campaign_status: CampaignStatus,
    bidding_deadline: DateTime<Utc>,
    submission_window_days: i16,
    /// Frozen when the campaign was published.
    #[sqlx(flatten)]
    rules: Rules,
}

impl BidListItem {
    fn new(row: BidListRow, now: DateTime<Utc>) -> Self {
        let bid = row.bid;
        Self {
            campaign: BidListCampaign {
                id: bid.campaign_id,
                title: row.title,
                advertiser_name: row.advertiser_name,
                platform: bid.platform,
                briefing: row.briefing,
                bidding_deadline: row.bidding_deadline,
                submission_window_days: row.submission_window_days,
                phase: Phase::new(row.campaign_status, Some(row.bidding_deadline), now),
            },
            payout_cents: row.rules.payout_cents(bid.amount_cents),
            min_paid_views: row.rules.min_paid_views(bid.view_score, bid.amount_cents),
            bid,
        }
    }
}

/// Places a bid, changes a pending one or reinstates a withdrawn one. Every rule is checked each
/// time, and the bid takes a fresh copy of the account's figures.
pub async fn place(
    State(pool): State<PgPool>,
    Path((campaign_id, account_id)): Path<(i64, i64)>,
    Json(BidAmount { amount_cents }): Json<BidAmount>,
) -> Result<Json<Bid>, ApiError> {
    place_bid(&pool, campaign_id, account_id, amount_cents)
        .await
        .map(Json)
}

/// [`place`] outside a request, so that the demo's bots obey the same rules.
pub(super) async fn place_bid(
    pool: &PgPool,
    campaign_id: i64,
    account_id: i64,
    amount_cents: i64,
) -> Result<Bid, ApiError> {
    let mut tx = pool.begin().await?;
    let campaign = find_open_campaign(&mut tx, campaign_id, Lock::Yes).await?;
    let bidder = find_bidder(&mut tx, campaign_id, account_id, Lock::Yes).await?;
    ensure_can_bid(&campaign, &bidder, campaign_id)?;
    check_amount(
        amount_cents,
        &campaign.terms.rules,
        bidder.account.view_score,
    )?;
    ensure_no_other_pending_bid(&mut tx, campaign_id, &bidder).await?;
    let match_score = campaign.score(&bidder.account).score;
    let bid = upsert_bid(&mut tx, campaign_id, &bidder, amount_cents, match_score).await?;
    tx.commit().await?;
    Ok(bid)
}

/// Whether a bid of this amount would win if bidding closed now: runs
/// [`marketplace::select_winners`] on the campaign's pending bids, with the account's bid set to
/// the amount as placing it would record it. Checks what placing checks, but writes nothing.
pub async fn chance(
    State(pool): State<PgPool>,
    Path((campaign_id, account_id)): Path<(i64, i64)>,
    Query(ChanceParams { amount_cents }): Query<ChanceParams>,
) -> Result<Json<Chance>, ApiError> {
    // One snapshot for every read, as in the progress view.
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .await?;
    let campaign = find_open_campaign(&mut tx, campaign_id, Lock::No).await?;
    let bidder = find_bidder(&mut tx, campaign_id, account_id, Lock::No).await?;
    ensure_can_bid(&campaign, &bidder, campaign_id)?;
    check_amount(
        amount_cents,
        &campaign.terms.rules,
        bidder.account.view_score,
    )?;
    let existing_id = existing_bid_id(&mut tx, campaign_id, account_id).await?;
    let mut bids = db::selection::pending_bids(&mut *tx, campaign_id).await?;
    tx.commit().await?;

    // As placing it would record it: the account's figures now, and the id of its existing bid.
    // A new bid would get an id above every other, so it loses a tie to them.
    let bid = PendingBid {
        id: existing_id.unwrap_or(i64::MAX),
        amount_cents,
        view_score: bidder.account.view_score,
        match_score: campaign.score(&bidder.account).score,
        reliability_score: bidder.account.reliability_score,
    };
    // Replaces the account's pending bid, if it has one.
    bids.retain(|other| other.id != bid.id);
    bids.push(bid);
    let selection = marketplace::select_winners(&campaign.terms, &bids);
    Ok(Json(Chance {
        would_win: selection.outcomes.last() == Some(&Outcome::Won),
        pending_bids: bids.len(),
    }))
}

/// Withdraws a pending bid, which can be placed again until the deadline. Idempotent.
pub async fn withdraw(
    State(pool): State<PgPool>,
    Path((campaign_id, account_id)): Path<(i64, i64)>,
) -> Result<Json<Bid>, ApiError> {
    let mut tx = pool.begin().await?;
    find_open_campaign(&mut tx, campaign_id, Lock::Yes).await?;
    let bid: Bid = sqlx::query_as(
        "SELECT * FROM bids WHERE campaign_id = $1 AND platform_account_id = $2 FOR UPDATE",
    )
    .bind(campaign_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError::NotFound(format!(
            "Platform account {account_id} has no bid on campaign {campaign_id}."
        ))
    })?;
    // Closing decides the bids in the transaction that closes the campaign, so an open campaign's
    // bids are only ever pending or withdrawn.
    let bid = if bid.status == BidStatus::Pending {
        sqlx::query_as(
            "UPDATE bids SET status = 'withdrawn', updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(bid.id)
        .fetch_one(&mut *tx)
        .await?
    } else {
        bid
    };
    tx.commit().await?;
    Ok(Json(bid))
}

/// The account's bids, newest first. Not paged, as an account bids on few campaigns.
pub async fn list(
    State(pool): State<PgPool>,
    Path(account_id): Path<i64>,
    Query(params): Query<ListParams>,
) -> Result<Json<BidList>, ApiError> {
    ensure_account_exists(&pool, account_id).await?;
    // Each bid with its campaign and the deal the campaign froze. The index on
    // (platform_account_id, created_at DESC, id DESC) serves the filter and the order.
    let mut query = QueryBuilder::new(
        "SELECT b.*, c.title, a.name AS advertiser_name, c.briefing, c.status AS campaign_status,
             c.bidding_deadline, c.submission_window_days, ",
    );
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(
            " FROM bids b
              JOIN campaigns c ON c.id = b.campaign_id
              JOIN advertisers a ON a.id = c.advertiser_id
              WHERE b.platform_account_id = ",
        )
        .push_bind(account_id);
    if let Some(status) = params.status {
        query.push(" AND b.status = ").push_bind(status);
    }
    query.push(" ORDER BY b.created_at DESC, b.id DESC");
    let rows: Vec<BidListRow> = query.build_query_as().fetch_all(&pool).await?;
    let now = Utc::now();
    Ok(Json(BidList {
        items: rows
            .into_iter()
            .map(|row| BidListItem::new(row, now))
            .collect(),
    }))
}

/// The open campaign's terms. With [`Lock::Yes`], locks it for the rest of the transaction.
///
/// Bids lock the campaign `FOR SHARE`, so they do not wait for each other. Closing locks it
/// `FOR UPDATE`, so it waits for the bids in flight and later bids wait for it. A bid therefore
/// commits before closing starts, and closing decides it, or it finds the campaign closed, as
/// Postgres re-checks the `WHERE` on the row closing committed.
async fn find_open_campaign(
    conn: &mut PgConnection,
    id: i64,
    lock: Lock,
) -> Result<CampaignTerms, ApiError> {
    let mut query = QueryBuilder::new("SELECT ");
    query
        .push(CAMPAIGN_TERMS_COLUMNS)
        .push(", ")
        .push(CAMPAIGN_PREFERENCES_COLUMNS)
        .push(" FROM campaigns c WHERE c.id = ")
        .push_bind(id)
        .push(" AND ")
        .push(CAMPAIGN_OPEN);
    if lock == Lock::Yes {
        query.push(" FOR SHARE");
    }
    let open: Option<CampaignTerms> = query.build_query_as().fetch_optional(&mut *conn).await?;
    let Some(campaign) = open else {
        let conflict = format!("Campaign {id} is not open for bids.");
        return Err(missing_or_conflict(conn, id, conflict).await);
    };
    Ok(campaign)
}

/// The account, as a bidder on the campaign. With [`Lock::Yes`], locks the account's creator for
/// the rest of the transaction.
///
/// The lock makes one creator's bids take turns, which keeps "one pending bid per creator and
/// campaign" true; no unique index can, as the creator is on the account, not the bid. A second
/// bid waits here for the first to commit. Its pending-bid check is a later statement, so under
/// READ COMMITTED it sees the first bid.
async fn find_bidder(
    conn: &mut PgConnection,
    campaign_id: i64,
    account_id: i64,
    lock: Lock,
) -> Result<Bidder, ApiError> {
    let mut query = QueryBuilder::new("SELECT pa.id, pa.creator_id, pa.platform, ");
    query
        .push(ACCOUNT_COLUMNS)
        .push(", ")
        .push(ACCOUNT_MEETS_TARGETING)
        .push(
            " AS meets_targeting
             FROM platform_accounts pa
             JOIN creators cr ON cr.id = pa.creator_id
             -- The one campaign, so that the targeting predicate can read it as `c`.
             JOIN campaigns c ON c.id = ",
        )
        .push_bind(campaign_id)
        .push(" WHERE pa.id = ")
        .push_bind(account_id);
    if lock == Lock::Yes {
        query.push(" FOR UPDATE OF cr");
    }
    query
        .build_query_as()
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Platform account", account_id))
}

/// Fails unless the account meets the targeting and [`marketplace::Terms::can_bid`] allows its
/// views: the same test that puts the campaign in the account's feed.
fn ensure_can_bid(
    campaign: &CampaignTerms,
    bidder: &Bidder,
    campaign_id: i64,
) -> Result<(), ApiError> {
    if bidder.meets_targeting && campaign.terms.can_bid(bidder.account.view_score) {
        Ok(())
    } else {
        Err(ApiError::Unprocessable(format!(
            "Platform account {} cannot bid on campaign {campaign_id}.",
            bidder.id
        )))
    }
}

/// Fails unless the amount is between the account's min and max bid. Neither depends on the
/// budget, which creators are not shown.
fn check_amount(amount_cents: i64, rules: &Rules, view_score: i32) -> Result<(), ApiError> {
    let min_bid_cents = rules.min_bid_cents(view_score);
    let max_bid_cents = rules.max_bid_cents(view_score);
    let error = if amount_cents < min_bid_cents {
        format!("Must be at least {}.", euros(min_bid_cents))
    } else if amount_cents > max_bid_cents {
        format!(
            "At {} your video would need {} views within {VIEWS_COUNTING_DAYS} days, more than \
             it is likely to reach. Bid at most {}.",
            euros(amount_cents),
            grouped(rules.min_paid_views(view_score, amount_cents)),
            euros(max_bid_cents)
        )
    } else {
        return Ok(());
    };
    Err(ApiError::Validation(FieldErrors::from([(
        "amountCents",
        error,
    )])))
}

/// Fails if another of the creator's accounts has a pending bid on the campaign. [`find_bidder`]
/// keeps this true under concurrent bids.
async fn ensure_no_other_pending_bid(
    conn: &mut PgConnection,
    campaign_id: i64,
    bidder: &Bidder,
) -> Result<(), ApiError> {
    let other_pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM bids b JOIN platform_accounts pa ON pa.id = b.platform_account_id
             WHERE b.campaign_id = $1 AND b.status = 'pending'
                 AND pa.creator_id = $2 AND pa.id <> $3
         )",
    )
    .bind(campaign_id)
    .bind(bidder.creator_id)
    .bind(bidder.id)
    .fetch_one(conn)
    .await?;
    if other_pending {
        Err(ApiError::Unprocessable(
            "Another account of this creator has a pending bid on the campaign; withdraw it first."
                .to_owned(),
        ))
    } else {
        Ok(())
    }
}

/// Inserts the bid or overwrites the account's earlier one. Reusing the row keeps its id and
/// `created_at`, so the bid keeps its place in the account's list and its time in a tie at closing.
async fn upsert_bid(
    conn: &mut PgConnection,
    campaign_id: i64,
    bidder: &Bidder,
    amount_cents: i64,
    match_score: i16,
) -> sqlx::Result<Bid> {
    sqlx::query_as(
        "INSERT INTO bids
             (campaign_id, platform_account_id, platform, amount_cents, view_score,
              reliability_score, match_score)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (campaign_id, platform_account_id) DO UPDATE SET
             amount_cents = EXCLUDED.amount_cents, view_score = EXCLUDED.view_score,
             reliability_score = EXCLUDED.reliability_score, match_score = EXCLUDED.match_score,
             status = 'pending', updated_at = now()
         RETURNING *",
    )
    .bind(campaign_id)
    .bind(bidder.id)
    .bind(bidder.platform)
    .bind(amount_cents)
    .bind(bidder.account.view_score)
    .bind(bidder.account.reliability_score)
    .bind(match_score)
    .fetch_one(conn)
    .await
}

/// The id of the account's bid on the campaign, whatever its status.
async fn existing_bid_id(
    conn: &mut PgConnection,
    campaign_id: i64,
    account_id: i64,
) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar("SELECT id FROM bids WHERE campaign_id = $1 AND platform_account_id = $2")
        .bind(campaign_id)
        .bind(account_id)
        .fetch_optional(conn)
        .await
}

async fn ensure_account_exists(pool: &PgPool, id: i64) -> Result<(), ApiError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM platform_accounts WHERE id = $1)")
            .bind(id)
            .fetch_one(pool)
            .await?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::not_found("Platform account", id))
    }
}
