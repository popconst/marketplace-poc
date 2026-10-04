-- Genres a campaign targets: an account must cover at least one, and covering more raises its
-- match score. No rows means any genre.
CREATE TABLE campaign_genres (
    campaign_id bigint NOT NULL REFERENCES campaigns (id) ON DELETE CASCADE,
    genre_id    smallint NOT NULL REFERENCES genres (id),
    PRIMARY KEY (campaign_id, genre_id)
);
