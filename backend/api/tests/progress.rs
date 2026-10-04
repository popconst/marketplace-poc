mod common;

use axum::http::StatusCode;
use common::{
    CampaignSpec, CampaignState, get, insert_advertiser, insert_campaign, new_account, place_bid,
    set_state, withdraw_bid,
};
use serde_json::{Value, json};
use sqlx::PgPool;

fn progress_uri(campaign: i64) -> String {
    format!("/api/campaigns/{campaign}/progress")
}

fn results_uri(campaign: i64) -> String {
    format!("/api/campaigns/{campaign}/results")
}

async fn bid_from_new_account(
    pool: &PgPool,
    campaign: i64,
    handle: &str,
    views: i32,
    amount_cents: i64,
) -> i64 {
    let account = new_account(pool, handle, views).await.unwrap();
    let (status, bid) = place_bid(pool, campaign, account, amount_cents).await;
    assert_eq!(status, StatusCode::OK, "{bid}");
    account
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn progress_of_a_campaign_without_bids_is_zero(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let spec = CampaignSpec::default();
    let campaign = insert_campaign(&pool, advertiser, spec.clone()).await?;

    let (status, progress) = get(&pool, &progress_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{progress}");
    let groups: Vec<Value> = spec
        .terms()
        .split_budget()
        .iter()
        .map(|budget| {
            json!({
                "group": budget.group,
                "budgetCents": budget.budget_cents,
                "spentCents": 0,
                "winners": 0,
                "bids": 0,
            })
        })
        .collect();
    assert_eq!(
        progress,
        json!({
            "pendingBids": 0,
            "budgetCents": 1_000_000,
            "spentCents": 0,
            "returnedCents": 1_000_000,
            "winners": 0,
            "minPaidViews": 0,
            "expectedViews": 0,
            "effectiveCpmCents": null,
            "targetCpmCents": 1_200,
            "groups": groups,
        })
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn progress_projects_what_closing_decides(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    // A bid may take 25% of €400, €100, so €400 cannot buy all five €100 bids below.
    let campaign = CampaignSpec {
        budget_cents: 40_000,
        ..CampaignSpec::default()
    };
    let campaign = insert_campaign(&pool, advertiser, campaign).await?;
    for (handle, views) in [
        ("lena.moves", 1_500),
        ("jonas.cooks", 1_500),
        ("mia.travels", 1_500),
        ("ana.paints", 1_500),
        ("tom.lifts", 10_000),
    ] {
        bid_from_new_account(&pool, campaign, handle, views, 10_000).await;
    }
    // A withdrawn bid is neither projected nor decided.
    let withdrawn = bid_from_new_account(&pool, campaign, "eva.sings", 1_500, 9_000).await;
    assert_eq!(
        withdraw_bid(&pool, campaign, withdrawn).await.0,
        StatusCode::OK
    );

    let (status, progress) = get(&pool, &progress_uri(campaign)).await;
    db::closing::close(&pool, campaign).await?;
    let (_, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{progress}");
    assert_eq!(progress["pendingBids"], 5);
    let winners = results["winners"].as_array().unwrap();
    assert!((1..5).contains(&winners.len()), "{results}");
    assert_eq!(progress["winners"], winners.len());
    for total in [
        "spentCents",
        "returnedCents",
        "expectedViews",
        "effectiveCpmCents",
    ] {
        assert_eq!(progress[total], results[total], "{total}");
    }
    // What each winner's video must reach to be paid, as the results list it, added up.
    let min_paid_views: i64 = winners
        .iter()
        .map(|w| w["minPaidViews"].as_i64().unwrap())
        .sum();
    assert_eq!(progress["minPaidViews"], min_paid_views);
    let groups = progress["groups"].as_array().unwrap();
    for group in groups {
        let size = &group["group"];
        let won = winners.iter().filter(|w| &w["sizeGroup"] == size).count();
        assert_eq!(group["winners"], won, "{size}");
    }
    let bids_per_group: Vec<(&str, u64)> = groups
        .iter()
        .map(|group| {
            let name = group["group"].as_str().unwrap();
            (name, group["bids"].as_u64().unwrap())
        })
        .collect();
    assert_eq!(bids_per_group, [("nano", 4), ("micro", 1), ("macro", 0)]);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn only_active_campaigns_have_progress(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let draft = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, draft, CampaignState::Draft).await?;
    let (status, error) = get(&pool, &progress_uri(draft)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["error"], "conflict");

    // Past the deadline the bids still wait for closing.
    let closing = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, closing, CampaignState::PastDeadline).await?;
    let (status, progress) = get(&pool, &progress_uri(closing)).await;
    assert_eq!(status, StatusCode::OK, "{progress}");

    let (status, error) = get(&pool, &progress_uri(999)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
    assert_eq!(error["error"], "not_found");
    Ok(())
}
