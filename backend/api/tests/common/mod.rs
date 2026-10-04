// Each test binary uses a different subset of this module.
#![allow(dead_code)]

use api::AppState;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use chrono::{TimeDelta, Utc};
use db::config::{CampaignLimits, Settings};
use db::models::Platform;
use marketplace::{Rules, SizeGroup, Terms};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

/// Stats of every inserted account and its creator. Views vary per test.
pub const FOLLOWERS: i32 = 48_200;
pub const ENGAGEMENT_RATE: f64 = 0.064;
pub const QUALITY: i16 = 72;
pub const RELIABILITY: i16 = 88;

/// Settings for the test routers and campaigns, spelled out so that changing a default does not
/// change what the tests check.
pub fn test_config() -> Settings {
    Settings {
        rules: Rules {
            commission_bps: 2_500,
            usual_rate_base_cents: 9_000,
            usual_rate_per_1000_views_cents: 1_000,
            fair_pay_floor_bps: 5_000,
            max_bid_share_of_budget_bps: 2_500,
            views_to_get_paid_bps: 3_500,
            likely_views_limit_bps: 6_000,
            discovery_bps: 1_500,
        },
        min_offer_bps: 5_000,
        limits: CampaignLimits {
            min_bidding_hours: 24,
            max_bidding_hours: 720,
            posting_window_default_days: 3,
            posting_window_max_days: 7,
        },
    }
}

pub fn router(pool: &PgPool) -> Router {
    router_with(pool, test_config(), false)
}

pub fn dev_router(pool: &PgPool) -> Router {
    router_with(pool, test_config(), true)
}

pub fn router_with(pool: &PgPool, settings: Settings, dev_tools: bool) -> Router {
    let state = AppState {
        pool: pool.clone(),
        settings,
        dev_tools,
    };
    api::router(state)
}

/// Sends a request with an optional JSON body and returns the status and the JSON response.
pub async fn send(
    router: Router,
    method: Method,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    };
    let response = router.oneshot(request.unwrap()).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

pub async fn get(pool: &PgPool, uri: &str) -> (StatusCode, Value) {
    send(router(pool), Method::GET, uri, None).await
}

pub async fn post(pool: &PgPool, uri: &str, body: Option<&Value>) -> (StatusCode, Value) {
    send(router(pool), Method::POST, uri, body).await
}

pub async fn patch(pool: &PgPool, uri: &str, body: &Value) -> (StatusCode, Value) {
    send(router(pool), Method::PATCH, uri, Some(body)).await
}

pub async fn put(pool: &PgPool, uri: &str, body: &Value) -> (StatusCode, Value) {
    send(router(pool), Method::PUT, uri, Some(body)).await
}

pub async fn delete(pool: &PgPool, uri: &str) -> (StatusCode, Value) {
    send(router(pool), Method::DELETE, uri, None).await
}

pub fn bid_uri(campaign: i64, account: i64) -> String {
    format!("/api/campaigns/{campaign}/bids/{account}")
}

/// Places or changes the account's bid on the campaign.
pub async fn place_bid(
    pool: &PgPool,
    campaign: i64,
    account: i64,
    amount_cents: i64,
) -> (StatusCode, Value) {
    let body = json!({ "amountCents": amount_cents });
    put(pool, &bid_uri(campaign, account), &body).await
}

pub async fn withdraw_bid(pool: &PgPool, campaign: i64, account: i64) -> (StatusCode, Value) {
    delete(pool, &bid_uri(campaign, account)).await
}

pub async fn insert_advertiser(pool: &PgPool, name: &str) -> sqlx::Result<i64> {
    sqlx::query_scalar("INSERT INTO advertisers (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
}

pub async fn insert_creator(pool: &PgPool, name: &str) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO creators (name, reliability_score) VALUES ($1, $2)
         RETURNING id",
    )
    .bind(name)
    .bind(RELIABILITY)
    .fetch_one(pool)
    .await
}

#[derive(Clone, Copy)]
pub struct AccountSpec {
    pub platform: Platform,
    pub view_score: i32,
    pub country_code: &'static str,
    pub brand_safe: bool,
}

impl Default for AccountSpec {
    fn default() -> Self {
        Self {
            platform: Platform::TikTok,
            view_score: 12_500,
            country_code: "DE",
            brand_safe: true,
        }
    }
}

pub async fn insert_account_with(
    pool: &PgPool,
    creator: i64,
    handle: &str,
    account: AccountSpec,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO platform_accounts
             (creator_id, platform, handle, country_code, followers, view_score, engagement_rate,
              posts_per_week, quality_score, brand_safe, stats_updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 4.5, $8, $9, '2026-09-28T10:00:00Z')
         RETURNING id",
    )
    .bind(creator)
    .bind(account.platform)
    .bind(handle)
    .bind(account.country_code)
    .bind(FOLLOWERS)
    .bind(account.view_score)
    .bind(ENGAGEMENT_RATE)
    .bind(QUALITY)
    .bind(account.brand_safe)
    .fetch_one(pool)
    .await
}

pub async fn insert_account(
    pool: &PgPool,
    creator: i64,
    handle: &str,
    view_score: i32,
) -> sqlx::Result<i64> {
    let account = AccountSpec {
        view_score,
        ..AccountSpec::default()
    };
    insert_account_with(pool, creator, handle, account).await
}

/// Inserts a creator named after the handle with one default account, and returns the account.
pub async fn new_account(pool: &PgPool, handle: &str, view_score: i32) -> sqlx::Result<i64> {
    let creator = insert_creator(pool, handle).await?;
    insert_account(pool, creator, handle, view_score).await
}

pub async fn add_genres(pool: &PgPool, account: i64, genre_ids: &[i16]) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO platform_account_genres (platform_account_id, genre_id)
         SELECT $1, unnest($2::smallint[])",
    )
    .bind(account)
    .bind(genre_ids)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_languages(pool: &PgPool, account: i64, codes: &[&str]) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO platform_account_languages (platform_account_id, language_code)
         SELECT $1, unnest($2::text[])",
    )
    .bind(account)
    .bind(codes)
    .execute(pool)
    .await?;
    Ok(())
}

/// A campaign for [`insert_campaign`]. It targets no country, language or genre.
#[derive(Clone)]
pub struct CampaignSpec {
    pub platform: Platform,
    pub budget_cents: i64,
    pub target_cpm_cents: i64,
    pub size_groups: Vec<SizeGroup>,
    pub engagement_weight: i16,
    pub quality_weight: i16,
    pub reliability_weight: i16,
    pub closing_in: TimeDelta,
}

impl Default for CampaignSpec {
    fn default() -> Self {
        Self {
            platform: Platform::TikTok,
            budget_cents: 1_000_000,
            target_cpm_cents: 1_200,
            size_groups: vec![SizeGroup::Nano, SizeGroup::Micro, SizeGroup::Macro],
            engagement_weight: 50,
            quality_weight: 50,
            reliability_weight: 50,
            closing_in: TimeDelta::days(3),
        }
    }
}

impl CampaignSpec {
    /// The terms [`insert_campaign`] publishes the campaign with.
    pub fn terms(&self) -> Terms {
        test_config()
            .terms(self.budget_cents, self.target_cpm_cents, &self.size_groups)
            .expect("the campaign picks a size")
    }
}

/// Inserts the campaign as a draft, then publishes it with [`test_config`] as of a day ago,
/// freezing its deal the way the publish endpoint does.
pub async fn insert_campaign(
    pool: &PgPool,
    advertiser: i64,
    campaign: CampaignSpec,
) -> sqlx::Result<i64> {
    let settings = test_config();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO campaigns
             (advertiser_id, title, briefing, platform, budget_cents, target_cpm_cents,
              size_groups, engagement_weight, quality_weight, reliability_weight,
              submission_window_days)
         VALUES ($1, 'Summer serum launch', 'Show your morning routine with the serum.', $2, $3,
                 $4, $5, $6, $7, $8, $9)
         RETURNING id",
    )
    .bind(advertiser)
    .bind(campaign.platform)
    .bind(campaign.budget_cents)
    .bind(campaign.target_cpm_cents)
    .bind(&campaign.size_groups)
    .bind(campaign.engagement_weight)
    .bind(campaign.quality_weight)
    .bind(campaign.reliability_weight)
    .bind(settings.limits.posting_window_default_days)
    .fetch_one(pool)
    .await?;
    let terms = campaign.terms();
    let now = Utc::now();
    db::campaigns::publish(
        pool,
        id,
        now + campaign.closing_in,
        now - TimeDelta::days(1),
        &terms.rules,
        terms.offer_bps,
    )
    .await?;
    Ok(id)
}

/// A state other than open that [`set_state`] moves a campaign to.
#[derive(Clone, Copy, Debug)]
pub enum CampaignState {
    Draft,
    InReview,
    /// Still active, but past its bidding deadline.
    PastDeadline,
    Closed,
    Failed,
}

pub async fn set_state(pool: &PgPool, campaign: i64, state: CampaignState) -> sqlx::Result<()> {
    let update = match state {
        CampaignState::Draft => {
            "UPDATE campaigns SET status = 'draft', submitted_at = NULL, activated_at = NULL
             WHERE id = $1"
        }
        CampaignState::InReview => {
            "UPDATE campaigns SET status = 'in_review', activated_at = NULL WHERE id = $1"
        }
        CampaignState::PastDeadline => {
            "UPDATE campaigns SET bidding_deadline = now() - interval '1 hour' WHERE id = $1"
        }
        CampaignState::Closed => {
            "UPDATE campaigns SET status = 'closed', closed_at = now() WHERE id = $1"
        }
        CampaignState::Failed => "UPDATE campaigns SET status = 'failed' WHERE id = $1",
    };
    sqlx::query(update).bind(campaign).execute(pool).await?;
    Ok(())
}
