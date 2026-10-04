-- An account can post in several languages.
CREATE TABLE platform_account_languages (
    platform_account_id bigint NOT NULL REFERENCES platform_accounts (id) ON DELETE CASCADE,
    language_code       text NOT NULL REFERENCES languages (code),
    PRIMARY KEY (platform_account_id, language_code)
);

CREATE INDEX platform_account_languages_language_code_idx
    ON platform_account_languages (language_code);
