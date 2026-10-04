//! Table rows as Rust types, serialized as the API's JSON, and a campaign's phase.

use chrono::{DateTime, Utc};
use marketplace::SizeGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "platform", rename_all = "lowercase")]
pub enum Platform {
    TikTok,
    Instagram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "campaign_status", rename_all = "snake_case")]
pub enum CampaignStatus {
    Draft,
    /// Waiting for approval. Never stored, as this demo approves a published campaign at once.
    InReview,
    Active,
    Closed,
    /// The worker gave up closing it after repeated errors.
    Failed,
}

/// A campaign's status with active split by the clock. Not stored, as it changes with time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Draft,
    InReview,
    Open,
    /// Past the deadline but not yet closed by the worker. Takes no more bids.
    Closing,
    Closed,
    Failed,
}

impl Phase {
    #[must_use]
    pub fn new(
        status: CampaignStatus,
        bidding_deadline: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Self {
        match status {
            CampaignStatus::Draft => Self::Draft,
            CampaignStatus::InReview => Self::InReview,
            CampaignStatus::Active if bidding_deadline.is_some_and(|d| now < d) => Self::Open,
            CampaignStatus::Active => Self::Closing,
            CampaignStatus::Closed => Self::Closed,
            CampaignStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "bid_status", rename_all = "snake_case")]
pub enum BidStatus {
    Pending,
    Won,
    Lost,
    Withdrawn,
}

pub use marketplace::LossReason;

/// What a creator's content is about.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub id: i16,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Country {
    /// ISO 3166-1 alpha-2, e.g. `DE`.
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    /// ISO 639-1, e.g. `de`.
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: i64,
    pub name: String,
    /// How dependably the creator delivers what they win, 0 to 100, over a rolling 30 days.
    pub reliability_score: i16,
    pub created_at: DateTime<Utc>,
}

/// A creator's social media account with its latest stats. Its size group is not stored but
/// derived from `view_score` by [`SizeGroup::of`], as the boundaries are our own assumption.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAccount {
    pub id: i64,
    pub creator_id: i64,
    pub platform: Platform,
    pub handle: String,
    /// Where the account is based (ISO 3166-1 alpha-2).
    pub country_code: String,
    pub followers: i32,
    /// Median views per post over posts 2 to 90 days old.
    pub view_score: i32,
    /// (likes + comments + shares + saves) / views, over the same posts.
    pub engagement_rate: f64,
    pub posts_per_week: f64,
    /// Content quality from 0 to 100.
    pub quality_score: i16,
    /// Accounts that are not brand safe are never matched to a campaign.
    pub brand_safe: bool,
    pub stats_updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Advertiser {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// A campaign on one platform, with money in euro cents. Bidding is open from `activated_at`
/// until `bidding_deadline`. A draft may leave the `Option` fields empty; publishing sets the
/// deadline and requires the rest. Of the frozen deal only the commission is here, to show the
/// advertiser; [`crate::selection::terms`] reads the whole deal.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Campaign {
    pub id: i64,
    pub advertiser_id: i64,
    pub status: CampaignStatus,
    pub title: Option<String>,
    /// What creators are asked to post.
    pub briefing: Option<String>,
    pub platform: Option<Platform>,

    pub budget_cents: Option<i64>,
    /// What 1,000 expected views are worth to the advertiser. Compared with what the picked
    /// sizes usually cost, it sets the offer to creators.
    pub target_cpm_cents: Option<i64>,
    /// The account sizes that see the campaign; they split the budget evenly. Never empty.
    pub size_groups: Vec<SizeGroup>,
    /// WePush's commission, in basis points (1500 is 15%). Frozen at publish; `None` on a draft.
    pub commission_bps: Option<i64>,

    /// The matching sliders, each 0, 25, 50, 75 or 100. They rank the campaign in creators'
    /// feeds and, through the match score, weigh against a bid's price when winners are picked.
    pub engagement_weight: i16,
    pub quality_weight: i16,
    pub reliability_weight: i16,

    /// When bidding closes and winners are picked.
    pub bidding_deadline: Option<DateTime<Utc>>,
    /// Winners must post within this many days of the bidding deadline.
    pub submission_window_days: i16,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// When the advertiser published it.
    pub submitted_at: Option<DateTime<Utc>>,
    /// When it was approved and bidding opened.
    pub activated_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    /// How many times the worker failed to close it.
    pub close_attempts: i16,
    pub last_close_error: Option<String>,
}

/// One platform account's offer to post for a campaign. `view_score`, `reliability_score` and
/// `match_score` are as of when the bid was placed or last edited.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    pub id: i64,
    pub campaign_id: i64,
    pub platform_account_id: i64,
    pub platform: Platform,
    /// What the advertiser pays, in euro cents; the creator gets this minus WePush's commission.
    pub amount_cents: i64,
    pub view_score: i32,
    pub reliability_score: i16,
    pub match_score: i16,
    pub status: BidStatus,
    /// Set only on lost bids.
    pub loss_reason: Option<LossReason>,
    /// Set when the bid is won or lost.
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
