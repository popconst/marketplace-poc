mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use chrono::{DateTime, Utc};
use common::{
    AccountSpec, CampaignSpec, CampaignState, insert_account, insert_account_with,
    insert_advertiser, insert_campaign, insert_creator, new_account, place_bid, set_state,
};
use db::models::Platform;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const SIMULATE_URI: &str = "/api/dev/simulate-bids";

fn close_uri(campaign: i64) -> String {
    format!("/api/dev/campaigns/{campaign}/close")
}

fn close_soon_uri(campaign: i64) -> String {
    format!("/api/dev/campaigns/{campaign}/close-soon")
}

async fn post_dev(pool: &PgPool, uri: &str, body: Option<&Value>) -> (StatusCode, Value) {
    common::send(common::dev_router(pool), Method::POST, uri, body).await
}

/// Runs one round of bot bids and returns its counts.
async fn simulate(pool: &PgPool, body: Option<&Value>) -> Value {
    let (status, body) = post_dev(pool, SIMULATE_URI, body).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// Inserts creators with a micro and a macro account each; both can bid on a default campaign.
async fn insert_creators(pool: &PgPool, count: i32) -> sqlx::Result<()> {
    for i in 0..count {
        let creator = insert_creator(pool, &format!("Creator {i}")).await?;
        insert_account(pool, creator, &format!("micro.{i}"), 4_000 + 500 * i).await?;
        insert_account(pool, creator, &format!("macro.{i}"), 40_000 + 1_000 * i).await?;
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bots_bid_only_with_matching_accounts_of_open_campaigns(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let open = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let draft = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, draft, CampaignState::Draft).await?;
    insert_creators(&pool, 20).await?;
    // Accounts that cannot bid: too small for any size, a size not picked, another platform.
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let starter = insert_account(&pool, creator, "lena.starts", 499).await?;
    let mega = insert_account(&pool, creator, "lena.mega", 500_000).await?;
    let instagram = AccountSpec {
        platform: Platform::Instagram,
        ..AccountSpec::default()
    };
    let instagram = insert_account_with(&pool, creator, "lena.insta", instagram).await?;

    let result = simulate(&pool, None).await;

    assert_eq!(result["campaigns"], 1);
    // 20 creators can bid, one account each, and the default is 20 bids per campaign.
    let placed = result["placed"].as_i64().unwrap();
    assert!((1..=20).contains(&placed), "{result}");
    let bids: Vec<(i64, i64)> = sqlx::query_as("SELECT campaign_id, platform_account_id FROM bids")
        .fetch_all(&pool)
        .await?;
    assert_eq!(result["placed"], bids.len());
    for (campaign, account) in bids {
        assert_eq!(campaign, open);
        assert!(![starter, mega, instagram].contains(&account), "{account}");
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_second_round_bids_only_with_new_accounts(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    insert_creators(&pool, 20).await?;
    let body = json!({ "bidsPerCampaign": 5 });

    let first = simulate(&pool, Some(&body)).await;
    let second = simulate(&pool, Some(&body)).await;

    let placed: Vec<i64> = [&first, &second]
        .into_iter()
        .map(|round| round["placed"].as_i64().unwrap())
        .collect();
    assert!(placed.iter().all(|n| (1..=5).contains(n)), "{placed:?}");
    // One row per bid: a second bid from the same account would have updated its row.
    let bids: i64 = sqlx::query_scalar("SELECT count(*) FROM bids")
        .fetch_one(&pool)
        .await?;
    assert_eq!(bids, placed.iter().sum::<i64>());
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn closing_now_closes_an_active_campaign_once(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let account = new_account(&pool, "lena.moves", 12_500).await?;
    assert_eq!(
        place_bid(&pool, campaign, account, 15_000).await.0,
        StatusCode::OK
    );

    let (status, closing) = post_dev(&pool, &close_uri(campaign), None).await;

    assert_eq!(status, StatusCode::OK, "{closing}");
    assert_eq!(
        closing,
        json!({ "winners": 1, "losers": 0, "spentCents": 15_000, "returnedCents": 985_000 })
    );
    // Neither a closed campaign nor a draft is active.
    let draft = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, draft, CampaignState::Draft).await?;
    for campaign in [campaign, draft] {
        let (status, error) = post_dev(&pool, &close_uri(campaign), None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{error}");
        assert_eq!(error["error"], "conflict");
    }
    let (status, error) = post_dev(&pool, &close_uri(999), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
    assert_eq!(error["error"], "not_found");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn closing_soon_moves_an_active_campaigns_deadline_to_ten_seconds_from_now(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    let (status, body) = post_dev(&pool, &close_soon_uri(campaign), None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let deadline: DateTime<Utc> =
        sqlx::query_scalar("SELECT bidding_deadline FROM campaigns WHERE id = $1")
            .bind(campaign)
            .fetch_one(&pool)
            .await?;
    // Ten seconds from the update, less the moment the test took since.
    let seconds = (deadline - Utc::now()).num_seconds();
    assert!((8..=10).contains(&seconds), "{seconds}");
    // A draft is not active.
    let draft = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, draft, CampaignState::Draft).await?;
    let (status, error) = post_dev(&pool, &close_soon_uri(draft), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    Ok(())
}

#[tokio::test]
async fn the_api_router_leaves_the_dev_routes_out() {
    let pool = PgPool::connect_lazy("postgres://localhost/unused").unwrap();

    for uri in [SIMULATE_URI, &close_uri(1), &close_soon_uri(1)] {
        let response = common::router(&pool)
            .oneshot(Request::post(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}
