-- One social media account. A creator can have several, including several on the same platform.
CREATE TABLE platform_accounts (
    id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    creator_id       bigint NOT NULL REFERENCES creators (id) ON DELETE CASCADE,
    platform         platform NOT NULL,
    handle           text NOT NULL CHECK (handle <> ''),
    -- Where the account is based, which campaigns target.
    country_code     text NOT NULL REFERENCES countries (code),
    followers        integer NOT NULL CHECK (followers >= 0),
    -- Median views per post over posts 2 to 90 days old.
    view_score       integer NOT NULL CHECK (view_score >= 0),
    -- (likes + comments + shares + saves) / views, over the same posts.
    engagement_rate  double precision NOT NULL CHECK (engagement_rate BETWEEN 0 AND 1),
    posts_per_week   double precision NOT NULL CHECK (posts_per_week >= 0),
    -- Content quality from 0 to 100: video quality, look, editing, past brand work.
    quality_score    smallint NOT NULL CHECK (quality_score BETWEEN 0 AND 100),
    -- Accounts that are not brand safe are never matched to a campaign.
    brand_safe       boolean NOT NULL,
    stats_updated_at timestamptz NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),

    -- Lets bids reference (account, platform), so a bid's platform must be its account's.
    CONSTRAINT platform_accounts_id_platform_key UNIQUE (id, platform)
);

-- Handles are case-insensitive on both platforms.
CREATE UNIQUE INDEX platform_accounts_platform_handle_key
    ON platform_accounts (platform, lower(handle));
CREATE INDEX platform_accounts_creator_id_idx ON platform_accounts (creator_id);

-- The account list sorts by one of these, highest first, and pages by (sort value, id).
CREATE INDEX platform_accounts_view_score_idx ON platform_accounts (view_score DESC, id DESC);
CREATE INDEX platform_accounts_followers_idx ON platform_accounts (followers DESC, id DESC);
CREATE INDEX platform_accounts_engagement_rate_idx
    ON platform_accounts (engagement_rate DESC, id DESC);
CREATE INDEX platform_accounts_quality_score_idx ON platform_accounts (quality_score DESC, id DESC);
-- Filtering by country is the common case, so it also gets the default sort order.
CREATE INDEX platform_accounts_country_code_view_score_idx
    ON platform_accounts (country_code, view_score DESC, id DESC);
-- Substring search on the handle in the account list.
CREATE INDEX platform_accounts_handle_trgm_idx ON platform_accounts USING gin (handle gin_trgm_ops);
