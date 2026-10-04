-- Countries a campaign targets. No rows means any country.
CREATE TABLE campaign_countries (
    campaign_id  bigint NOT NULL REFERENCES campaigns (id) ON DELETE CASCADE,
    country_code text NOT NULL REFERENCES countries (code),
    PRIMARY KEY (campaign_id, country_code)
);
