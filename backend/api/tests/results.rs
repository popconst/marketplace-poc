mod common;

use axum::http::StatusCode;
use common::{
    CampaignSpec, CampaignState, ENGAGEMENT_RATE, FOLLOWERS, QUALITY, RELIABILITY, get,
    insert_advertiser, insert_campaign, new_account, place_bid, set_state, test_config,
};
use db::models::BidStatus;
use marketplace::{Account, LossReason, Match, Outcome, Preferences, cpm_cents};
use serde_json::{Value, json};
use sqlx::PgPool;

const NANO_VIEWS: i32 = 1_500;
const MICRO_VIEWS: i32 = 12_500;

fn results_uri(campaign: i64) -> String {
    format!("/api/campaigns/{campaign}/results")
}

/// Places each `(handle, views, amount, outcome)` bid from a new account, records the outcome and
/// closes the campaign. Results only report what closing decided, so the tests set the outcomes
/// directly instead of running winner selection. Returns the accounts, in order.
async fn close_with(
    pool: &PgPool,
    campaign: i64,
    bids: &[(&str, i32, i64, Outcome)],
) -> sqlx::Result<Vec<i64>> {
    let mut accounts = Vec::new();
    for &(handle, views, amount_cents, outcome) in bids {
        let account = new_account(pool, handle, views).await?;
        let (status, bid) = place_bid(pool, campaign, account, amount_cents).await;
        assert_eq!(status, StatusCode::OK, "{bid}");
        let (decided, loss_reason) = match outcome {
            Outcome::Won => (BidStatus::Won, None),
            Outcome::Lost(reason) => (BidStatus::Lost, Some(reason)),
        };
        sqlx::query(
            "UPDATE bids SET status = $2, loss_reason = $3, decided_at = now() WHERE id = $1",
        )
        .bind(bid["id"].as_i64())
        .bind(decided)
        .bind(loss_reason)
        .execute(pool)
        .await?;
        accounts.push(account);
    }
    set_state(pool, campaign, CampaignState::Closed).await?;
    Ok(accounts)
}

/// The match of an account with the test stats and this content quality on a default campaign.
fn match_with_quality(quality_score: i16) -> Match {
    let account = Account {
        engagement_rate: ENGAGEMENT_RATE,
        quality_score,
        reliability_score: RELIABILITY,
        genre_ids: &[],
    };
    let spec = CampaignSpec::default();
    let preferences = Preferences {
        engagement_weight: spec.engagement_weight,
        quality_weight: spec.quality_weight,
        reliability_weight: spec.reliability_weight,
        genre_ids: &[],
    };
    marketplace::score(&account, &preferences)
}

fn handles<'a>(results: &'a Value, list: &str) -> Vec<&'a str> {
    results[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["handle"].as_str().unwrap())
        .collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn results_order_winners_by_size_then_amount_and_losers_by_reason(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let (outranked, did_not_fit, over_limit) = (
        Outcome::Lost(LossReason::Outranked),
        Outcome::Lost(LossReason::DidNotFit),
        Outcome::Lost(LossReason::OverLimit),
    );
    close_with(
        &pool,
        campaign,
        &[
            ("micro.cheap", MICRO_VIEWS, 11_000, Outcome::Won),
            ("nano.cheap", NANO_VIEWS, 9_000, Outcome::Won),
            ("micro.dear", MICRO_VIEWS, 15_000, Outcome::Won),
            ("nano.dear", NANO_VIEWS, 10_000, Outcome::Won),
            ("over.limit", NANO_VIEWS, 9_900, over_limit),
            ("did.not.fit", NANO_VIEWS, 9_500, did_not_fit),
            ("outranked.cheap", MICRO_VIEWS, 12_000, outranked),
            ("outranked.dear", MICRO_VIEWS, 14_000, outranked),
        ],
    )
    .await?;

    let (status, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{results}");
    assert_eq!(
        handles(&results, "winners"),
        ["nano.dear", "nano.cheap", "micro.dear", "micro.cheap"]
    );
    // By reason in declaration order, whatever the amount.
    assert_eq!(
        handles(&results, "losers"),
        [
            "outranked.dear",
            "outranked.cheap",
            "did.not.fit",
            "over.limit"
        ]
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn results_add_up_what_the_winners_cost(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let spec = CampaignSpec::default();
    let campaign = insert_campaign(&pool, advertiser, spec.clone()).await?;
    let outranked = Outcome::Lost(LossReason::Outranked);
    close_with(
        &pool,
        campaign,
        &[
            ("nano.wins", NANO_VIEWS, 10_000, Outcome::Won),
            ("micro.wins", MICRO_VIEWS, 15_000, Outcome::Won),
            ("micro.loses", MICRO_VIEWS, 14_000, outranked),
        ],
    )
    .await?;

    let (status, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{results}");
    // Only the winners count.
    let spent_cents = 10_000 + 15_000;
    let expected_views = i64::from(NANO_VIEWS + MICRO_VIEWS);
    assert!(results["closedAt"].is_string());
    assert_eq!(results["budgetCents"], spec.budget_cents);
    assert_eq!(results["spentCents"], spent_cents);
    assert_eq!(results["returnedCents"], spec.budget_cents - spent_cents);
    assert_eq!(results["expectedViews"], expected_views);
    // Each winner's min paid views by the campaign's rules.
    let rules = test_config().rules;
    assert_eq!(
        results["minPaidViews"],
        rules.min_paid_views(NANO_VIEWS, 10_000) + rules.min_paid_views(MICRO_VIEWS, 15_000)
    );
    assert_eq!(
        results["effectiveCpmCents"],
        json!(cpm_cents(spent_cents, expected_views))
    );
    assert_eq!(results["targetCpmCents"], spec.target_cpm_cents);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn result_rows_show_the_account_and_what_its_bid_cost(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let outranked = Outcome::Lost(LossReason::Outranked);
    let accounts = close_with(
        &pool,
        campaign,
        &[
            ("lena.moves", MICRO_VIEWS, 15_000, Outcome::Won),
            ("jonas.cooks", NANO_VIEWS, 9_000, outranked),
        ],
    )
    .await?;

    let (status, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{results}");
    let rules = test_config().rules;
    assert_eq!(
        results["winners"],
        json!([{
            "accountId": accounts[0],
            "handle": "lena.moves",
            "creatorName": "lena.moves",
            "countryCode": "DE",
            "followers": FOLLOWERS,
            "engagementRate": ENGAGEMENT_RATE,
            "viewScore": MICRO_VIEWS,
            "match": match_with_quality(QUALITY),
            "amountCents": 15_000,
            "sizeGroup": "micro",
            "minPaidViews": rules.min_paid_views(MICRO_VIEWS, 15_000),
            "expectedCpmCents": cpm_cents(15_000, MICRO_VIEWS.into()),
        }])
    );
    assert_eq!(
        results["losers"],
        json!([{
            "accountId": accounts[1],
            "handle": "jonas.cooks",
            "creatorName": "jonas.cooks",
            "countryCode": "DE",
            "viewScore": NANO_VIEWS,
            "match": match_with_quality(QUALITY),
            "amountCents": 9_000,
            "lossReason": "outranked",
            "sizeGroup": "nano",
            "expectedCpmCents": cpm_cents(9_000, NANO_VIEWS.into()),
        }])
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn result_rows_show_the_bid_as_decided_and_its_match_factors_as_of_now(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let outranked = Outcome::Lost(LossReason::Outranked);
    let accounts = close_with(
        &pool,
        campaign,
        &[
            ("lena.moves", MICRO_VIEWS, 15_000, Outcome::Won),
            ("jonas.cooks", MICRO_VIEWS, 14_000, outranked),
        ],
    )
    .await?;
    // The accounts' stats change after closing.
    sqlx::query(
        "UPDATE platform_accounts SET view_score = 40000, quality_score = 20 WHERE id = ANY ($1)",
    )
    .bind(&accounts)
    .execute(&pool)
    .await?;

    let (status, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{results}");
    let as_decided = match_with_quality(QUALITY);
    let as_of_now = match_with_quality(20);
    assert_ne!(as_of_now.score, as_decided.score);
    for row in [&results["winners"][0], &results["losers"][0]] {
        assert_eq!(row["viewScore"], MICRO_VIEWS);
        assert_eq!(row["match"]["score"], as_decided.score);
        assert_eq!(row["match"]["factors"], json!(as_of_now.factors));
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_campaign_closed_without_winners_returns_its_budget(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    db::closing::close(&pool, campaign).await?;

    let (status, results) = get(&pool, &results_uri(campaign)).await;

    assert_eq!(status, StatusCode::OK, "{results}");
    assert_eq!(results["spentCents"], 0);
    assert_eq!(results["returnedCents"], 1_000_000);
    assert_eq!(results["expectedViews"], 0);
    assert_eq!(results["effectiveCpmCents"], json!(null));
    assert_eq!(results["winners"], json!([]));
    assert_eq!(results["losers"], json!([]));
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn only_closed_campaigns_have_results(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let active = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    let (status, error) = get(&pool, &results_uri(active)).await;

    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["error"], "conflict");
    let (status, error) = get(&pool, &results_uri(999)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
    assert_eq!(error["error"], "not_found");
    Ok(())
}
