//! Rows in COPY's text format, sent a chunk at a time: COPY is far faster than INSERT at millions
//! of accounts. A row is an array as long as its table's column list, so the compiler checks
//! that every column gets a field.

use std::fmt::{self, Display, Write as _};

use db::models::{Creator, Platform, PlatformAccount};
use sqlx::PgConnection;

use crate::creators::{SeedAccount, SeedCreator};

/// The rows of a batch of creators, buffered per table.
#[derive(Default)]
pub struct Chunk {
    creators: String,
    accounts: String,
    account_genres: String,
    account_languages: String,
}

impl Chunk {
    pub fn push(&mut self, seed: &SeedCreator) {
        push_creator(&mut self.creators, &seed.creator);
        for SeedAccount {
            account,
            genre_ids,
            language_codes,
        } in &seed.accounts
        {
            push_account(&mut self.accounts, account);
            for genre_id in genre_ids {
                let row: [&dyn Display; ACCOUNT_GENRE_COLUMNS.len()] = [&account.id, genre_id];
                push_row(&mut self.account_genres, &row);
            }
            for language_code in language_codes {
                let row: [&dyn Display; ACCOUNT_LANGUAGE_COLUMNS.len()] =
                    [&account.id, language_code];
                push_row(&mut self.account_languages, &row);
            }
        }
    }

    /// Sends and empties the buffers, parents before children so foreign keys resolve.
    pub async fn copy(&mut self, conn: &mut PgConnection) -> sqlx::Result<()> {
        let tables = [
            ("creators", &CREATOR_COLUMNS[..], &mut self.creators),
            (
                "platform_accounts",
                &ACCOUNT_COLUMNS[..],
                &mut self.accounts,
            ),
            (
                "platform_account_genres",
                &ACCOUNT_GENRE_COLUMNS[..],
                &mut self.account_genres,
            ),
            (
                "platform_account_languages",
                &ACCOUNT_LANGUAGE_COLUMNS[..],
                &mut self.account_languages,
            ),
        ];
        for (table, columns, rows) in tables {
            let statement = format!("COPY {table} ({}) FROM STDIN", columns.join(", "));
            let mut copy = conn.copy_in_raw(&statement).await?;
            copy.send(rows.as_bytes()).await?;
            copy.finish().await?;
            rows.clear();
        }
        Ok(())
    }
}

const CREATOR_COLUMNS: [&str; 4] = ["id", "name", "reliability_score", "created_at"];

fn push_creator(rows: &mut String, creator: &Creator) {
    let row: [&dyn Display; CREATOR_COLUMNS.len()] = [
        &creator.id,
        &Text(&creator.name),
        &creator.reliability_score,
        &creator.created_at,
    ];
    push_row(rows, &row);
}

const ACCOUNT_COLUMNS: [&str; 13] = [
    "id",
    "creator_id",
    "platform",
    "handle",
    "country_code",
    "followers",
    "view_score",
    "engagement_rate",
    "posts_per_week",
    "quality_score",
    "brand_safe",
    "stats_updated_at",
    "created_at",
];

fn push_account(rows: &mut String, account: &PlatformAccount) {
    let row: [&dyn Display; ACCOUNT_COLUMNS.len()] = [
        &account.id,
        &account.creator_id,
        &platform_label(account.platform),
        &Text(&account.handle),
        &account.country_code,
        &account.followers,
        &account.view_score,
        &account.engagement_rate,
        &account.posts_per_week,
        &account.quality_score,
        &account.brand_safe,
        &account.stats_updated_at,
        &account.created_at,
    ];
    push_row(rows, &row);
}

const ACCOUNT_GENRE_COLUMNS: [&str; 2] = ["platform_account_id", "genre_id"];
const ACCOUNT_LANGUAGE_COLUMNS: [&str; 2] = ["platform_account_id", "language_code"];

/// Appends one tab-separated row. Postgres parses each field's `Display` text as its column's
/// type, such as `true` or a `DateTime<Utc>`'s `2026-10-01 12:00:00 UTC`.
fn push_row(rows: &mut String, fields: &[&dyn Display]) {
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            rows.push('\t');
        }
        write!(rows, "{field}").expect("writing to a String cannot fail");
    }
    rows.push('\n');
}

/// The label of the `platform` enum in the database.
fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::TikTok => "tiktok",
        Platform::Instagram => "instagram",
    }
}

/// A text field, with the characters COPY's text format reserves escaped.
struct Text<'a>(&'a str);

impl Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '\\' => f.write_str("\\\\")?,
                '\t' => f.write_str("\\t")?,
                '\n' => f.write_str("\\n")?,
                '\r' => f.write_str("\\r")?,
                c => f.write_char(c)?,
            }
        }
        Ok(())
    }
}
