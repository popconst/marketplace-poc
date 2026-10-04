//! Which accounts may see and bid on which campaign, defined once so that the creator feed,
//! bidding, the advertiser's estimate and the demo bots agree.
//!
//! An account matches a campaign when it meets the targeting ([`ACCOUNT_MEETS_TARGETING`], in
//! SQL) and can bid ([`Terms::can_bid`], in Rust): its size is picked and its min bid fits the
//! share of the budget one bid may take. The feed and bidding run `can_bid` on the rows SQL
//! returns. The estimate and the bots count in SQL, so [`push_matching_accounts`] also writes
//! `can_bid` in SQL via [`Rules::max_view_score_for_budget`]; the marketplace test
//! `max_view_score_for_budget_agrees_with_can_bid` checks that the two agree.
//!
//! The SQL here uses fixed table aliases, which the surrounding query must define: `c` for
//! `campaigns`, `pa` for `platform_accounts` and `cr` for `creators`. Each predicate is in
//! parentheses, so it can be joined to others with `AND`.

use marketplace::{Account, Match, Preferences, Rules, SizeGroup, Terms};
use serde::Serialize;
use sqlx::{Postgres, QueryBuilder};

/// Campaign `c` is active and before its deadline. `now()` is the transaction's start time.
pub const CAMPAIGN_OPEN: &str = "(c.status = 'active' AND c.bidding_deadline > now())";

/// Account `pa` meets campaign `c`'s targeting: platform, brand safety, countries, languages and
/// genres, regardless of the account's size and the campaign's status. An empty targeting list
/// means any.
pub const ACCOUNT_MEETS_TARGETING: &str = "(
    pa.platform = c.platform
    AND pa.brand_safe
    -- The campaign targets no country, or the account's.
    AND (NOT EXISTS (SELECT 1 FROM campaign_countries WHERE campaign_id = c.id)
         OR EXISTS (SELECT 1 FROM campaign_countries
                    WHERE campaign_id = c.id AND country_code = pa.country_code))
    -- The campaign targets no language, or one the account posts in.
    AND (NOT EXISTS (SELECT 1 FROM campaign_languages WHERE campaign_id = c.id)
         OR EXISTS (SELECT 1 FROM campaign_languages cl
                    JOIN platform_account_languages pal ON pal.language_code = cl.language_code
                    WHERE cl.campaign_id = c.id AND pal.platform_account_id = pa.id))
    -- The campaign targets no genre, or one the account covers.
    AND (NOT EXISTS (SELECT 1 FROM campaign_genres WHERE campaign_id = c.id)
         OR EXISTS (SELECT 1 FROM campaign_genres cg
                    JOIN platform_account_genres pag ON pag.genre_id = cg.genre_id
                    WHERE cg.campaign_id = c.id AND pag.platform_account_id = pa.id))
)";

/// The columns of campaign `c` that [`Terms`] decodes from. Most are NULL on drafts, so select
/// them only from published campaigns.
pub const CAMPAIGN_TERMS_COLUMNS: &str = "c.budget_cents, c.size_groups, c.offer_bps,
    c.commission_bps, c.usual_rate_base_cents, c.usual_rate_per_1000_views_cents,
    c.fair_pay_floor_bps, c.max_bid_share_of_budget_bps,
    c.views_to_get_paid_bps, c.likely_views_limit_bps, c.discovery_bps";

/// The columns of account `pa` and its creator `cr` that [`AccountRow`] decodes from.
pub const ACCOUNT_COLUMNS: &str =
    "pa.view_score, pa.followers, pa.engagement_rate, pa.quality_score, cr.reliability_score,
     ARRAY(SELECT genre_id FROM platform_account_genres WHERE platform_account_id = pa.id)
         AS genre_ids";

/// An account's figures for matching, selected by [`ACCOUNT_COLUMNS`] and shown in the feed.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    pub view_score: i32,
    pub followers: i32,
    pub engagement_rate: f64,
    #[serde(skip)]
    pub quality_score: i16,
    /// From the account's creator.
    #[serde(skip)]
    pub reliability_score: i16,
    #[serde(skip)]
    pub genre_ids: Vec<i16>,
}

impl AccountRow {
    #[must_use]
    pub fn as_account(&self) -> Account<'_> {
        Account {
            engagement_rate: self.engagement_rate,
            quality_score: self.quality_score,
            reliability_score: self.reliability_score,
            genre_ids: &self.genre_ids,
        }
    }
}

/// The columns of campaign `c` that the match score reads. See [`CampaignTerms`].
pub const CAMPAIGN_PREFERENCES_COLUMNS: &str =
    "c.engagement_weight, c.quality_weight, c.reliability_weight,
     ARRAY(SELECT genre_id FROM campaign_genres WHERE campaign_id = c.id) AS genre_ids";

/// A published campaign's frozen terms and match preferences. The feed, bidding and the demo bots
/// all read campaigns as this, so they judge bids alike. Decodes from [`CAMPAIGN_TERMS_COLUMNS`]
/// and [`CAMPAIGN_PREFERENCES_COLUMNS`], on `campaigns` or the `campaign_details` view.
#[derive(Debug, sqlx::FromRow)]
pub struct CampaignTerms {
    #[sqlx(flatten)]
    pub terms: Terms,
    pub engagement_weight: i16,
    pub quality_weight: i16,
    pub reliability_weight: i16,
    pub genre_ids: Vec<i16>,
}

impl CampaignTerms {
    #[must_use]
    pub fn score(&self, account: &AccountRow) -> Match {
        let preferences = Preferences {
            engagement_weight: self.engagement_weight,
            quality_weight: self.quality_weight,
            reliability_weight: self.reliability_weight,
            genre_ids: &self.genre_ids,
        };
        marketplace::score(&account.as_account(), &preferences)
    }
}

/// Pushes the `FROM` and `WHERE` clauses that select the accounts matching the campaign, joined
/// to their creators. The terms are passed in because a draft has none of its own; for a draft,
/// pass the terms it would be published with now. The pushed SQL:
///
/// ```sql
/// FROM platform_accounts pa
/// JOIN creators cr ON cr.id = pa.creator_id
/// JOIN campaigns c ON c.id = $campaign_id
/// WHERE <ACCOUNT_MEETS_TARGETING>
///   AND (<views in the first picked size> OR <views in the next> ...)
///   AND pa.view_score <= $max_view_score_for_budget
/// ```
pub fn push_matching_accounts(query: &mut QueryBuilder<Postgres>, campaign_id: i64, terms: &Terms) {
    query
        .push(
            " FROM platform_accounts pa
              JOIN creators cr ON cr.id = pa.creator_id
              JOIN campaigns c ON c.id = ",
        )
        .push_bind(campaign_id)
        .push(" WHERE ")
        .push(ACCOUNT_MEETS_TARGETING)
        .push(" AND (");
    for (i, &group) in terms.size_groups.iter().enumerate() {
        if i > 0 {
            query.push(" OR ");
        }
        push_views_in(query, group);
    }
    query.push(") AND ");
    push_min_bid_fits(query, &terms.rules, terms.budget_cents);
}

/// Pushes the condition that account `pa`'s min bid is within the share of the budget one bid may
/// take, as a cap on its views from [`Rules::max_view_score_for_budget`].
pub fn push_min_bid_fits(query: &mut QueryBuilder<Postgres>, rules: &Rules, budget_cents: i64) {
    match rules.max_view_score_for_budget(budget_cents) {
        Some(max_view_score) => query.push("pa.view_score <= ").push_bind(max_view_score),
        // Below the minimum budget no account can bid.
        None => query.push("FALSE"),
    };
}

/// Pushes the condition that account `pa`'s views are in the size group, bounds inclusive.
pub fn push_views_in(query: &mut QueryBuilder<Postgres>, group: SizeGroup) {
    query
        .push("(pa.view_score >= ")
        .push_bind(group.min_views());
    if let Some(max_views) = group.max_views() {
        query.push(" AND pa.view_score <= ").push_bind(max_views);
    }
    query.push(")");
}
