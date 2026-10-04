mod common;

use std::collections::HashMap;
use std::time::Duration;

use chrono::{TimeDelta, Utc};
use common::{
    BidSpec, insert_account, insert_bid, insert_campaign, insert_campaign_closing_in,
    insert_creator, insert_draft,
};
use db::closing::{Closing, close, deadlines_until, record_close_failure};
use db::models::{Bid, BidStatus, Campaign, CampaignStatus, LossReason, Platform};
use marketplace::Outcome;
use serde_json::Value;
use sqlx::PgPool;

fn bid(amount_cents: i64, view_score: i32, match_score: i16) -> BidSpec {
    BidSpec {
        amount_cents,
        view_score,
        match_score,
        ..BidSpec::default()
    }
}

async fn insert_bid_of_new_account(
    pool: &PgPool,
    campaign_id: i64,
    bid: BidSpec,
) -> sqlx::Result<Bid> {
    let creator = insert_creator(pool).await?;
    let handle = format!("account.{}", creator.id);
    let account = insert_account(pool, creator.id, Platform::TikTok, &handle).await?;
    insert_bid(pool, campaign_id, account.id, bid).await
}

async fn campaign(pool: &PgPool, id: i64) -> sqlx::Result<Campaign> {
    sqlx::query_as("SELECT * FROM campaigns WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

async fn bids(pool: &PgPool, campaign_id: i64) -> sqlx::Result<Vec<Bid>> {
    sqlx::query_as("SELECT * FROM bids WHERE campaign_id = $1 ORDER BY id")
        .bind(campaign_id)
        .fetch_all(pool)
        .await
}

async fn snapshot(pool: &PgPool, campaign_id: i64) -> sqlx::Result<Value> {
    Ok(serde_json::json!({
        "campaign": campaign(pool, campaign_id).await?,
        "bids": bids(pool, campaign_id).await?,
    }))
}

async fn sessions_waiting_for_a_lock(pool: &PgPool, count: i64) -> sqlx::Result<()> {
    loop {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(pool)
        .await?;
        if waiting >= count {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn stored(outcome: Outcome) -> (BidStatus, Option<LossReason>) {
    match outcome {
        Outcome::Won => (BidStatus::Won, None),
        Outcome::Lost(reason) => (BidStatus::Lost, Some(reason)),
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn closing_records_what_select_winners_decides(pool: PgPool) -> sqlx::Result<()> {
    let id = insert_campaign(&pool).await?.id;
    // €600 across three sizes has room for only some of these bids, so every outcome occurs. A
    // winning bid may take 25% of it, €150, so the €160 bid is over the limit.
    sqlx::query("UPDATE campaigns SET budget_cents = 60000 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;
    for figures in [
        bid(10_000, 1_500, 80),
        bid(10_000, 1_500, 80),
        bid(10_000, 1_500, 80),
        bid(10_000, 1_500, 80),
        bid(16_000, 1_500, 80),
        bid(15_000, 12_500, 75),
        bid(15_000, 12_500, 75),
        bid(15_000, 12_500, 70),
        bid(9_500, 3_500, 60),
    ] {
        insert_bid_of_new_account(&pool, id, figures).await?;
    }
    let terms = db::selection::terms(&pool, id).await?.unwrap();
    let pending = db::selection::pending_bids(&pool, id).await?;
    let expected = marketplace::select_winners(&terms, &pending);
    let expected_by_id: HashMap<i64, _> = pending
        .iter()
        .zip(&expected.outcomes)
        .map(|(bid, &outcome)| (bid.id, stored(outcome)))
        .collect();
    for outcome in [
        (BidStatus::Won, None),
        (BidStatus::Lost, Some(LossReason::Outranked)),
        (BidStatus::Lost, Some(LossReason::DidNotFit)),
        (BidStatus::Lost, Some(LossReason::OverLimit)),
    ] {
        assert!(
            expected_by_id.values().any(|&o| o == outcome),
            "{outcome:?}"
        );
    }

    let closing = close(&pool, id).await?;

    assert_eq!(
        closing,
        Some(Closing {
            winners: expected.winner_count(),
            losers: pending.len() - expected.winner_count(),
            spent_cents: expected.spent_cents(),
            returned_cents: expected.returned_cents,
        })
    );
    for bid in bids(&pool, id).await? {
        assert_eq!(
            (bid.status, bid.loss_reason),
            expected_by_id[&bid.id],
            "bid {}",
            bid.id
        );
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bids_are_decided_at_the_moment_the_campaign_closes(pool: PgPool) -> sqlx::Result<()> {
    let id = insert_campaign(&pool).await?.id;
    insert_bid_of_new_account(&pool, id, BidSpec::default()).await?;
    insert_bid_of_new_account(&pool, id, BidSpec::default()).await?;

    close(&pool, id).await?;

    let campaign = campaign(&pool, id).await?;
    assert_eq!(campaign.status, CampaignStatus::Closed);
    let closed_at = campaign.closed_at.unwrap();
    assert_eq!(campaign.updated_at, closed_at);
    for bid in bids(&pool, id).await? {
        assert_eq!(
            (bid.decided_at, bid.updated_at),
            (Some(closed_at), closed_at)
        );
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn withdrawn_bids_stay_as_they_are(pool: PgPool) -> sqlx::Result<()> {
    let id = insert_campaign(&pool).await?.id;
    let pending = insert_bid_of_new_account(&pool, id, BidSpec::default()).await?;
    let withdrawn = insert_bid_of_new_account(&pool, id, BidSpec::default()).await?;
    let withdrawn: Bid =
        sqlx::query_as("UPDATE bids SET status = 'withdrawn' WHERE id = $1 RETURNING *")
            .bind(withdrawn.id)
            .fetch_one(&pool)
            .await?;

    let closing = close(&pool, id).await?;

    assert_eq!(closing.map(|c| (c.winners, c.losers)), Some((1, 0)));
    let after = bids(&pool, id).await?;
    assert_eq!((after[0].id, after[0].status), (pending.id, BidStatus::Won));
    assert_eq!(
        serde_json::to_value(&after[1]).unwrap(),
        serde_json::to_value(&withdrawn).unwrap()
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn closing_a_campaign_that_is_not_active_changes_nothing(pool: PgPool) -> sqlx::Result<()> {
    let closed = insert_campaign(&pool).await?.id;
    insert_bid_of_new_account(&pool, closed, BidSpec::default()).await?;
    close(&pool, closed).await?;
    let draft = insert_draft(&pool).await?.id;
    let before = [
        snapshot(&pool, closed).await?,
        snapshot(&pool, draft).await?,
    ];

    // The last one does not exist.
    for id in [closed, draft, draft + 1] {
        assert_eq!(close(&pool, id).await?, None, "campaign {id}");
    }

    let after = [
        snapshot(&pool, closed).await?,
        snapshot(&pool, draft).await?,
    ];
    assert_eq!(after, before);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn closing_waits_for_a_bid_in_flight_and_happens_once(pool: PgPool) -> sqlx::Result<()> {
    let id = insert_campaign(&pool).await?.id;
    let creator = insert_creator(&pool).await?;
    let account = insert_account(&pool, creator.id, Platform::TikTok, "lena.moves").await?;
    // Hold the campaign `FOR SHARE` while placing the bid, as bidding does.
    let mut placing = pool.begin().await?;
    sqlx::query("SELECT 1 FROM campaigns WHERE id = $1 FOR SHARE")
        .bind(id)
        .execute(&mut *placing)
        .await?;
    sqlx::query(
        "INSERT INTO bids
             (campaign_id, platform_account_id, platform, amount_cents, view_score,
              reliability_score, match_score)
         VALUES ($1, $2, 'tiktok', 15000, 12500, 88, 75)",
    )
    .bind(id)
    .bind(account.id)
    .execute(&mut *placing)
    .await?;
    // Commit the bid once both closers wait for the lock. If closing stopped waiting for bids,
    // the timeout fails the test instead of letting it hang.
    let place = async {
        tokio::time::timeout(
            Duration::from_secs(10),
            sessions_waiting_for_a_lock(&pool, 2),
        )
        .await
        .expect("both closers wait for the bid in flight")?;
        placing.commit().await
    };

    let (first, second, placed) = tokio::join!(close(&pool, id), close(&pool, id), place);

    placed?;
    // One closer decided the bid; the other then found the campaign closed.
    let decided: Vec<usize> = [first?, second?]
        .iter()
        .flatten()
        .map(|closing| closing.winners + closing.losers)
        .collect();
    assert_eq!(decided, [1]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn deadlines_until_lists_active_campaigns_due_by_then(pool: PgPool) -> sqlx::Result<()> {
    let soon = insert_campaign_closing_in(&pool, TimeDelta::seconds(10))
        .await?
        .id;
    let overdue = insert_campaign_closing_in(&pool, TimeDelta::hours(-1))
        .await?
        .id;
    insert_campaign_closing_in(&pool, TimeDelta::hours(1)).await?;
    let closed = insert_campaign_closing_in(&pool, TimeDelta::hours(-3))
        .await?
        .id;
    close(&pool, closed).await?;
    insert_draft(&pool).await?;

    let due = deadlines_until(&pool, Utc::now() + TimeDelta::seconds(30)).await?;

    let ids: Vec<i64> = due.iter().map(|&(_, id)| id).collect();
    assert_eq!(ids, [overdue, soon]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_campaign_that_keeps_failing_is_marked_failed(pool: PgPool) -> sqlx::Result<()> {
    let id = insert_campaign_closing_in(&pool, TimeDelta::hours(-1))
        .await?
        .id;

    for (error, status) in [
        ("error 1", CampaignStatus::Active),
        ("error 2", CampaignStatus::Active),
        ("error 3", CampaignStatus::Failed),
    ] {
        assert_eq!(
            record_close_failure(&pool, id, error, 3).await?,
            Some(status)
        );
    }

    let campaign = campaign(&pool, id).await?;
    assert_eq!(campaign.close_attempts, 3);
    assert_eq!(campaign.last_close_error.as_deref(), Some("error 3"));
    assert!(deadlines_until(&pool, Utc::now()).await?.is_empty());
    assert_eq!(record_close_failure(&pool, id, "error 4", 3).await?, None);
    Ok(())
}
