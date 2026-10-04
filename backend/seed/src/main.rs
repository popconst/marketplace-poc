//! Seeds a migrated database with deterministic demo data. Reading order: `data` and `genre`
//! (plain data), the generators `advertisers`, `campaigns` and `creators` (no I/O), `copy`, then
//! this file, which inserts everything.

mod advertisers;
mod campaigns;
mod copy;
mod creators;
mod data;
mod genre;

use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow, bail, ensure};
use chrono::{DateTime, SubsecRound as _, Utc};
use clap::Parser;
use db::config::{Settings, SettingsArgs};
use sqlx::{PgConnection, PgPool};

use crate::advertisers::{MAX_ADVERTISERS, SeedAdvertiser};
use crate::campaigns::SeedCampaign;
use crate::copy::Chunk;
use crate::creators::Generator;
use crate::data::CAMPAIGN_TERMS;

/// Creators sent per round of COPY statements; keeps memory to a few tens of MB.
const CHUNK_CREATORS: usize = 100_000;
const PROGRESS_EVERY: Duration = Duration::from_secs(5);

/// Fills a migrated database with deterministic fake advertisers, campaigns, creators and
/// platform accounts. For development and the demo only.
#[derive(Parser)]
struct Args {
    /// Postgres connection URL.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,

    /// Creators to generate, each with one to four platform accounts (about 1.5 on average).
    #[arg(long, default_value_t = 50_000)]
    creators: usize,

    /// Advertisers to generate, each with one open campaign and one or two drafts. The default,
    /// one per row of campaign terms, gives every open campaign different terms.
    #[arg(long, default_value_t = CAMPAIGN_TERMS.len())]
    advertisers: usize,

    /// The same seed and counts always produce the same rows.
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// First empty creators, advertisers and everything that references them, campaigns and
    /// bids included.
    #[arg(long, conflicts_with = "if_empty")]
    reset: bool,

    /// Seed only an empty database, and exit successfully if it has creators or advertisers.
    /// Without this or --reset, a database with data is an error.
    #[arg(long)]
    if_empty: bool,

    /// The settings the open campaigns are published with, read as the API reads them.
    #[command(flatten)]
    settings: SettingsArgs,
}

struct Seeded {
    campaigns: usize,
    accounts: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        args.advertisers <= MAX_ADVERTISERS,
        "--advertisers can be at most {MAX_ADVERTISERS}"
    );
    let settings = args
        .settings
        .into_settings()
        .map_err(|problem| anyhow!("invalid settings: {problem}"))?;
    let started = Instant::now();
    // Timestamps count back from now, so the data always looks current. Whole seconds, because
    // Postgres stores microseconds and would round finer times.
    let now = Utc::now().trunc_subsecs(0);

    let pool = db::connect(&args.database_url, 1).await?;
    // One transaction, so a failed or interrupted run leaves the database as it was.
    let mut tx = pool.begin().await?;
    let has_data = has_marketplace_data(&mut tx).await?;
    if args.reset {
        empty_marketplace(&mut tx).await?;
    } else if has_data && args.if_empty {
        println!("the database already has creators or advertisers; nothing seeded");
        return Ok(());
    } else if has_data {
        bail!(
            "the database already has creators or advertisers; pass --reset to replace them \
             (this also deletes all campaigns and bids)"
        );
    }
    let seeded = insert_everything(
        &mut tx,
        args.seed,
        args.advertisers,
        args.creators,
        &settings,
        now,
    )
    .await?;
    tx.commit().await?;

    analyze(&pool).await?;
    let size: String =
        sqlx::query_scalar("SELECT pg_size_pretty(pg_database_size(current_database()))")
            .fetch_one(&pool)
            .await?;
    println!(
        "seeded {} advertisers with {} campaigns, {} creators and {} platform accounts in {}s; \
         database size {size}",
        args.advertisers,
        seeded.campaigns,
        args.creators,
        seeded.accounts,
        started.elapsed().as_secs(),
    );
    Ok(())
}

/// Whether any creator or advertiser exists; every other seeded table references one of them.
/// As the run's first query, it is also where an unmigrated database fails.
async fn has_marketplace_data(conn: &mut PgConnection) -> anyhow::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM creators) OR EXISTS (SELECT 1 FROM advertisers)",
    )
    .fetch_one(conn)
    .await
    .context("is the database migrated? (cargo run -p db --bin migrate)")
}

async fn empty_marketplace(conn: &mut PgConnection) -> sqlx::Result<()> {
    // Listed instead of CASCADE, so a new table that references these makes the reset fail
    // rather than being emptied silently.
    sqlx::query(
        "TRUNCATE creators, platform_accounts, platform_account_genres,
             platform_account_languages, advertisers, campaigns, campaign_countries,
             campaign_languages, campaign_genres, bids
         RESTART IDENTITY",
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn insert_everything(
    conn: &mut PgConnection,
    seed: u64,
    advertiser_count: usize,
    creator_count: usize,
    settings: &Settings,
    now: DateTime<Utc>,
) -> anyhow::Result<Seeded> {
    let advertisers = advertisers::advertisers(seed, advertiser_count, now);
    insert_advertisers(conn, &advertisers).await?;
    let campaigns = campaigns::campaigns(seed, &advertisers, now);
    for campaign in &campaigns {
        insert_campaign(conn, campaign, settings).await?;
    }
    let accounts = copy_creators(conn, Generator::new(seed, now), creator_count).await?;
    move_id_sequences_past_seeded_rows(conn).await?;
    Ok(Seeded {
        campaigns: campaigns.len(),
        accounts,
    })
}

async fn insert_advertisers(
    conn: &mut PgConnection,
    advertisers: &[SeedAdvertiser],
) -> sqlx::Result<()> {
    for SeedAdvertiser { advertiser, .. } in advertisers {
        // OVERRIDING SYSTEM VALUE lets us set the identity column, which the campaigns reference.
        sqlx::query(
            "INSERT INTO advertisers (id, name, created_at) OVERRIDING SYSTEM VALUE
             VALUES ($1, $2, $3)",
        )
        .bind(advertiser.id)
        .bind(&advertiser.name)
        .bind(advertiser.created_at)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Inserts a campaign as a draft, then publishes an open one as the API does, freezing the deal
/// rules of `settings` on it. Every campaign starts with the default posting window.
async fn insert_campaign(
    conn: &mut PgConnection,
    campaign: &SeedCampaign,
    settings: &Settings,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO campaigns (id, advertiser_id, title, briefing, platform, budget_cents,
             target_cpm_cents, size_groups, engagement_weight, quality_weight,
             reliability_weight, submission_window_days, created_at, updated_at)
         OVERRIDING SYSTEM VALUE
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(campaign.id)
    .bind(campaign.advertiser_id)
    .bind(campaign.title)
    .bind(campaign.briefing)
    .bind(campaign.platform)
    .bind(campaign.budget_cents)
    .bind(campaign.target_cpm_cents)
    .bind(campaign.size_groups)
    .bind(campaign.sliders.engagement)
    .bind(campaign.sliders.quality)
    .bind(campaign.sliders.reliability)
    .bind(settings.limits.posting_window_default_days)
    .bind(campaign.created_at)
    .bind(campaign.updated_at)
    .execute(&mut *conn)
    .await?;

    // One row per targeted country, language and genre; an empty list inserts none.
    sqlx::query(
        "INSERT INTO campaign_countries (campaign_id, country_code)
         SELECT $1, unnest($2::text[])",
    )
    .bind(campaign.id)
    .bind(campaign.country_codes)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO campaign_languages (campaign_id, language_code)
         SELECT $1, unnest($2::text[])",
    )
    .bind(campaign.id)
    .bind(campaign.language_codes)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO campaign_genres (campaign_id, genre_id) SELECT $1, unnest($2::smallint[])",
    )
    .bind(campaign.id)
    .bind(campaign.genre_ids)
    .execute(&mut *conn)
    .await?;

    if let Some(published) = campaign.published {
        let id = campaign.id;
        let target_cpm_cents = campaign
            .target_cpm_cents
            .with_context(|| format!("open campaign {id} has no target CPM"))?;
        let offer_bps = settings
            .offer_bps(target_cpm_cents, campaign.size_groups)
            .with_context(|| format!("open campaign {id} picks no size"))?;
        db::campaigns::publish(
            &mut *conn,
            id,
            published.bidding_deadline,
            published.at,
            &settings.rules,
            offer_bps,
        )
        .await?;
    }
    Ok(())
}

/// Streams `count` creators with COPY, printing progress on long runs. Returns the account count.
async fn copy_creators(
    conn: &mut PgConnection,
    mut creators: Generator,
    count: usize,
) -> sqlx::Result<usize> {
    let started = Instant::now();
    let mut last_progress = started;
    let mut chunk = Chunk::default();
    let mut copied = 0;
    let mut accounts = 0;
    while copied < count {
        let size = CHUNK_CREATORS.min(count - copied);
        for creator in creators.by_ref().take(size) {
            accounts += creator.accounts.len();
            chunk.push(&creator);
        }
        chunk.copy(conn).await?;
        copied += size;
        if last_progress.elapsed() >= PROGRESS_EVERY {
            print_progress(copied, count, started.elapsed());
            last_progress = Instant::now();
        }
    }
    Ok(accounts)
}

fn print_progress(copied: usize, total: usize, elapsed: Duration) {
    let left = elapsed.as_secs() * (total - copied) as u64 / copied as u64;
    println!(
        "{copied}/{total} creators ({}%), {}s elapsed, about {left}s left",
        copied * 100 / total,
        elapsed.as_secs(),
    );
}

/// Lets the next insert get a new id. On an empty table `max(id)` is NULL, which `setval` ignores.
async fn move_id_sequences_past_seeded_rows(conn: &mut PgConnection) -> sqlx::Result<()> {
    sqlx::query(
        "SELECT
             setval(pg_get_serial_sequence('advertisers', 'id'), (SELECT max(id) FROM advertisers)),
             setval(pg_get_serial_sequence('campaigns', 'id'), (SELECT max(id) FROM campaigns)),
             setval(pg_get_serial_sequence('creators', 'id'), (SELECT max(id) FROM creators)),
             setval(pg_get_serial_sequence('platform_accounts', 'id'),
                    (SELECT max(id) FROM platform_accounts))",
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Refreshes the planner's statistics, as Postgres recommends after a bulk load.
async fn analyze(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "ANALYZE advertisers, campaigns, campaign_countries, campaign_languages, campaign_genres,
             creators, platform_accounts, platform_account_genres, platform_account_languages",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn now() -> DateTime<Utc> {
        "2026-10-01T12:00:00Z".parse().unwrap()
    }

    /// Every generator's output as `Debug` text, which shows every field and floats exactly.
    fn generated(seed: u64) -> [String; 3] {
        let advertisers = advertisers::advertisers(seed, MAX_ADVERTISERS, now());
        let campaigns = campaigns::campaigns(seed, &advertisers, now());
        let creators: Vec<_> = Generator::new(seed, now()).take(1_000).collect();
        [
            format!("{advertisers:?}"),
            format!("{campaigns:?}"),
            format!("{creators:?}"),
        ]
    }

    #[test]
    fn same_seed_gives_same_rows() {
        assert_eq!(generated(7), generated(7));
    }

    #[test]
    fn different_seeds_give_different_rows() {
        for (rows, other_rows) in generated(7).iter().zip(&generated(8)) {
            assert_ne!(rows, other_rows);
        }
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn seeded_rows_fit_the_schema(pool: PgPool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;
        insert_everything(&mut conn, 1, 3, 200, &Settings::default(), now()).await?;
        for table in ["advertisers", "campaigns", "creators", "platform_accounts"] {
            let (next_id, max_id): (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT nextval(pg_get_serial_sequence('{table}', 'id')), max(id) FROM {table}"
            )))
            .fetch_one(&mut *conn)
            .await?;
            assert_eq!(next_id, max_id + 1, "{table}");
        }
        Ok(())
    }
}
