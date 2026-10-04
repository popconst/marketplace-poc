-- Languages a campaign targets. No rows means any language.
CREATE TABLE campaign_languages (
    campaign_id   bigint NOT NULL REFERENCES campaigns (id) ON DELETE CASCADE,
    language_code text NOT NULL REFERENCES languages (code),
    PRIMARY KEY (campaign_id, language_code)
);
