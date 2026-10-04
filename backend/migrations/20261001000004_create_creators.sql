CREATE TABLE creators (
    id                bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name              text NOT NULL CHECK (name <> ''),
    -- How dependably the creator delivers what they win, 0 to 100, over a rolling 30 days.
    -- Used when choosing winning bids.
    reliability_score smallint NOT NULL CHECK (reliability_score BETWEEN 0 AND 100),
    created_at        timestamptz NOT NULL DEFAULT now()
);

-- Substring search on the name in the account list.
CREATE INDEX creators_name_trgm_idx ON creators USING gin (name gin_trgm_ops);
