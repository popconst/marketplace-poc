-- in_review is never stored, as this demo approves a published campaign at once. failed means
-- the worker gave up closing it after repeated errors.
CREATE TYPE campaign_status AS ENUM ('draft', 'in_review', 'active', 'closed', 'failed');

-- Account sizes by views, with boundaries set by the marketplace crate. Accounts below nano
-- (starters) cannot be picked.
CREATE TYPE size_group AS ENUM ('nano', 'micro', 'macro', 'mega');

-- Money is in euro cents and shares in basis points (2500 is 25%). The API enforces business
-- limits such as the minimum budget; the checks here only keep each row well formed.
CREATE TABLE campaigns (
    id                     bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    advertiser_id          bigint NOT NULL REFERENCES advertisers (id),
    status                 campaign_status NOT NULL DEFAULT 'draft',

    -- Columns without a default may be NULL on a draft; publishing requires them.
    title                  text CHECK (title <> ''),
    -- What creators are asked to post.
    briefing               text CHECK (briefing <> ''),
    platform               platform,

    budget_cents           bigint CHECK (budget_cents > 0),
    -- What 1,000 expected views are worth to the advertiser. Compared with what the picked
    -- account sizes usually cost per 1,000 views, it sets the offer below.
    target_cpm_cents       bigint CHECK (target_cpm_cents > 0),
    -- The account sizes that see the campaign. The budget is split evenly across them.
    size_groups            size_group[] NOT NULL DEFAULT '{nano,micro,macro}',

    -- The advertiser's matching sliders, each 0, 25, 50, 75 or 100.
    engagement_weight      smallint NOT NULL DEFAULT 50,
    quality_weight         smallint NOT NULL DEFAULT 50,
    reliability_weight     smallint NOT NULL DEFAULT 50,

    -- When bidding closes and winners are picked. Set at publish.
    bidding_deadline       timestamptz,
    -- Winners must post within this many days of the bidding deadline. A new draft gets the
    -- default from the settings.
    submission_window_days smallint NOT NULL CHECK (submission_window_days > 0),

    -- The deal, copied from the settings at publish so that a later change never alters a
    -- published campaign; NULL on a draft. Each column is named after the field of
    -- marketplace::Terms or marketplace::Rules that explains it.
    offer_bps bigint CHECK (offer_bps > 0),
    commission_bps bigint CHECK (commission_bps BETWEEN 0 AND 10000),
    usual_rate_base_cents bigint CHECK (usual_rate_base_cents > 0),
    usual_rate_per_1000_views_cents bigint CHECK (usual_rate_per_1000_views_cents > 0),
    fair_pay_floor_bps bigint CHECK (fair_pay_floor_bps BETWEEN 0 AND 10000),
    max_bid_share_of_budget_bps bigint CHECK (max_bid_share_of_budget_bps BETWEEN 1 AND 10000),
    views_to_get_paid_bps bigint CHECK (views_to_get_paid_bps BETWEEN 1 AND 10000),
    likely_views_limit_bps bigint CHECK (likely_views_limit_bps BETWEEN 1 AND 10000),
    discovery_bps bigint CHECK (discovery_bps BETWEEN 0 AND 10000),

    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    -- When the advertiser published it.
    submitted_at           timestamptz,
    -- When it was approved and bidding opened.
    activated_at           timestamptz,
    closed_at              timestamptz,
    -- The worker's failed attempts to close it, and the last error. After too many, it is
    -- marked failed.
    close_attempts         smallint NOT NULL DEFAULT 0 CHECK (close_attempts >= 0),
    last_close_error       text,

    -- Lets bids reference (campaign, platform), so a bid's platform must be its campaign's.
    CONSTRAINT campaigns_id_platform_key UNIQUE (id, platform),
    CONSTRAINT campaigns_size_groups_not_empty CHECK (cardinality(size_groups) > 0),
    CONSTRAINT campaigns_engagement_weight_step CHECK (engagement_weight IN (0, 25, 50, 75, 100)),
    CONSTRAINT campaigns_quality_weight_step CHECK (quality_weight IN (0, 25, 50, 75, 100)),
    CONSTRAINT campaigns_reliability_weight_step CHECK (reliability_weight IN (0, 25, 50, 75, 100)),
    CONSTRAINT campaigns_complete_unless_draft CHECK (
        status = 'draft' OR (
            title IS NOT NULL AND briefing IS NOT NULL AND platform IS NOT NULL
            AND budget_cents IS NOT NULL AND target_cpm_cents IS NOT NULL
            AND bidding_deadline IS NOT NULL
        )
    ),
    CONSTRAINT campaigns_deal_frozen_unless_draft CHECK (
        status = 'draft' OR (
            offer_bps IS NOT NULL AND commission_bps IS NOT NULL
            AND usual_rate_base_cents IS NOT NULL AND usual_rate_per_1000_views_cents IS NOT NULL
            AND fair_pay_floor_bps IS NOT NULL AND max_bid_share_of_budget_bps IS NOT NULL
            AND views_to_get_paid_bps IS NOT NULL AND likely_views_limit_bps IS NOT NULL
            AND discovery_bps IS NOT NULL
        )
    ),
    -- "(A) = (B)" means both or neither: a timestamp is set exactly when the status has got
    -- that far.
    CONSTRAINT campaigns_submitted_at_status CHECK ((status = 'draft') = (submitted_at IS NULL)),
    CONSTRAINT campaigns_activated_at_status CHECK (
        (status IN ('active', 'closed', 'failed')) = (activated_at IS NOT NULL)
    ),
    CONSTRAINT campaigns_closed_at_status CHECK ((status = 'closed') = (closed_at IS NOT NULL)),
    -- Bidding closes after it opens. How long it stays open is up to the API's settings.
    CONSTRAINT campaigns_bidding_period CHECK (
        activated_at IS NULL OR bidding_deadline > activated_at
    )
);

CREATE INDEX campaigns_advertiser_id_idx ON campaigns (advertiser_id);
-- The worker's queue: active campaigns by deadline.
CREATE INDEX campaigns_closing_idx ON campaigns (bidding_deadline) WHERE status = 'active';
