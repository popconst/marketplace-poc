CREATE TYPE bid_status AS ENUM ('pending', 'won', 'lost', 'withdrawn');

-- Why a bid lost when its campaign closed.
CREATE TYPE loss_reason AS ENUM (
    -- Its size group's budget went to bids of better value.
    'outranked',
    -- A worse-value bid in its size group won after it, being cheap enough for what was left.
    'did_not_fit',
    -- Above the account's max bid or the share of the budget one winning bid may take
    -- (marketplace::select_winners).
    'over_limit'
);

-- One platform account's offer to post for a campaign, editable until the bidding deadline.
CREATE TABLE bids (
    id                  bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    campaign_id         bigint NOT NULL,
    platform_account_id bigint NOT NULL,
    platform            platform NOT NULL,
    -- What the advertiser pays, in euro cents. The creator receives this minus WePush's
    -- commission.
    amount_cents        bigint NOT NULL CHECK (amount_cents > 0),
    -- Copied from the account and its creator when the bid is placed or edited, so the outcome
    -- depends only on this row.
    view_score          integer NOT NULL CHECK (view_score > 0),
    reliability_score   smallint NOT NULL CHECK (reliability_score BETWEEN 0 AND 100),
    -- The account's match score for the campaign at the same moment.
    match_score         smallint NOT NULL CHECK (match_score BETWEEN 0 AND 100),
    status              bid_status NOT NULL DEFAULT 'pending',
    loss_reason         loss_reason,
    decided_at          timestamptz,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT bids_one_per_account_per_campaign UNIQUE (campaign_id, platform_account_id),
    CONSTRAINT bids_campaign_platform_fkey
        FOREIGN KEY (campaign_id, platform) REFERENCES campaigns (id, platform),
    CONSTRAINT bids_account_platform_fkey
        FOREIGN KEY (platform_account_id, platform) REFERENCES platform_accounts (id, platform),
    -- Won and lost bids carry the time they were decided; only lost ones carry a reason.
    CONSTRAINT bids_decision CHECK (
        (status IN ('won', 'lost')) = (decided_at IS NOT NULL)
        AND (status = 'lost') = (loss_reason IS NOT NULL)
    )
);

-- An account's bids, newest first.
CREATE INDEX bids_platform_account_id_created_at_idx
    ON bids (platform_account_id, created_at DESC, id DESC);
