-- An account can cover several genres.
CREATE TABLE platform_account_genres (
    platform_account_id bigint NOT NULL REFERENCES platform_accounts (id) ON DELETE CASCADE,
    genre_id            smallint NOT NULL REFERENCES genres (id),
    PRIMARY KEY (platform_account_id, genre_id)
);

CREATE INDEX platform_account_genres_genre_id_idx ON platform_account_genres (genre_id);
