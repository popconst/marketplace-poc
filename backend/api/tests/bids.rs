mod common;

use axum::http::{Method, StatusCode};
use common::{
    AccountSpec, CampaignSpec, CampaignState, RELIABILITY, bid_uri, get, insert_account,
    insert_account_with, insert_advertiser, insert_campaign, insert_creator, new_account,
    place_bid, router_with, send, set_state, test_config, withdraw_bid,
};
use db::models::Platform;
use marketplace::SizeGroup;
use serde_json::{Value, json};
use sqlx::PgPool;

/// Views of the micro accounts most tests bid with.
const VIEWS: i32 = 12_500;

/// Asks whether the account's bid of this amount would win if bidding closed now.
async fn chance(
    pool: &PgPool,
    campaign: i64,
    account: i64,
    amount_cents: i64,
) -> (StatusCode, Value) {
    let uri = format!(
        "{}/chance?amount_cents={amount_cents}",
        bid_uri(campaign, account)
    );
    get(pool, &uri).await
}

async fn listed(pool: &PgPool, account: i64, query: &str) -> Vec<i64> {
    let uri = format!("/api/platform-accounts/{account}/bids{query}");
    let (status, body) = get(pool, &uri).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["campaignId"].as_i64().unwrap())
        .collect()
}

async fn feed_campaigns(pool: &PgPool, account: i64) -> Vec<i64> {
    let uri = format!("/api/platform-accounts/{account}/campaigns");
    let (status, feed) = get(pool, &uri).await;
    assert_eq!(status, StatusCode::OK, "{feed}");
    feed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["campaign"]["id"].as_i64().unwrap())
        .collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_bid_freezes_the_accounts_figures_and_an_edit_refreshes_them(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let account = insert_account(&pool, creator, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    let (status, bid) = place_bid(&pool, campaign, account, 15_000).await;

    assert_eq!(status, StatusCode::OK, "{bid}");
    assert_eq!(bid["campaignId"], campaign);
    assert_eq!(bid["platformAccountId"], account);
    assert_eq!(bid["platform"], "tiktok");
    assert_eq!(bid["amountCents"], 15_000);
    assert_eq!(bid["status"], "pending");
    assert_eq!(bid["viewScore"], VIEWS);
    assert_eq!(bid["reliabilityScore"], RELIABILITY);
    // Equal weights, and a 10% engagement rate scores 100: (64 + 72 + 88) / 3.
    assert_eq!(bid["matchScore"], 75);

    sqlx::query("UPDATE platform_accounts SET view_score = 20000 WHERE id = $1")
        .bind(account)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE creators SET reliability_score = 40 WHERE id = $1")
        .bind(creator)
        .execute(&pool)
        .await?;
    let (status, edited) = place_bid(&pool, campaign, account, 16_000).await;

    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["id"], bid["id"]);
    assert_eq!(edited["createdAt"], bid["createdAt"]);
    assert_eq!(edited["amountCents"], 16_000);
    assert_eq!(edited["viewScore"], 20_000);
    assert_eq!(edited["reliabilityScore"], 40);
    // (64 + 72 + 40) / 3.
    assert_eq!(edited["matchScore"], 59);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn amount_must_be_within_the_accounts_limits(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    // A winning bid may take 25% of €1,200, €300, less than the €368.57 max bid. Bidding takes
    // amounts up to the max bid anyway, as creators are not shown the budget.
    let campaign = CampaignSpec {
        budget_cents: 120_000,
        ..CampaignSpec::default()
    };
    let campaign = insert_campaign(&pool, advertiser, campaign).await?;
    let rules = test_config().rules;
    let min_bid_cents = rules.min_bid_cents(VIEWS);
    let max_bid_cents = rules.max_bid_cents(VIEWS);

    for (amount_cents, expected) in [
        (min_bid_cents - 1, StatusCode::UNPROCESSABLE_ENTITY),
        (min_bid_cents, StatusCode::OK),
        (max_bid_cents, StatusCode::OK),
        (max_bid_cents + 1, StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let (status, body) = place_bid(&pool, campaign, account, amount_cents).await;
        assert_eq!(status, expected, "{amount_cents}: {body}");
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn each_amount_error_names_its_limit(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    // A 10,000-view account usually charges €190, so its min bid is half that, €95. Above €325.71
    // its video would need over 60% of its views.
    let account = new_account(&pool, "lena.moves", 10_000).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    for (amount_cents, message) in [
        (9_499, "Must be at least €95."),
        (
            32_572,
            "At €325.72 your video would need 6,001 views within 5 days, more than it is likely \
             to reach. Bid at most €325.71.",
        ),
    ] {
        let (status, error) = place_bid(&pool, campaign, account, amount_cents).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
        assert_eq!(error["fields"], json!({ "amountCents": message }));
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_lone_bid_would_win(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    let (status, body) = chance(&pool, campaign, account, 15_000).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "wouldWin": true, "pendingBids": 1 }));
    // Once placed, the account's bid is replaced by the amount asked about, not counted twice.
    assert_eq!(
        place_bid(&pool, campaign, account, 15_000).await.0,
        StatusCode::OK
    );
    let (_, body) = chance(&pool, campaign, account, 16_000).await;
    assert_eq!(body, json!({ "wouldWin": true, "pendingBids": 1 }));
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_bid_outranked_by_cheaper_ones_would_not_win(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    // Only micro accounts, so micro has the whole €1,000, and a winning bid may take €250.
    let campaign = CampaignSpec {
        budget_cents: 100_000,
        size_groups: vec![SizeGroup::Micro],
        ..CampaignSpec::default()
    };
    let campaign = insert_campaign(&pool, advertiser, campaign).await?;
    // Five €200 bids from accounts like the one below take the €1,000.
    for i in 0..5 {
        let other = new_account(&pool, &format!("other.{i}"), VIEWS).await?;
        let (status, bid) = place_bid(&pool, campaign, other, 20_000).await;
        assert_eq!(status, StatusCode::OK, "{bid}");
    }
    let account = new_account(&pool, "lena.moves", VIEWS).await?;

    let (status, body) = chance(&pool, campaign, account, 21_000).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "wouldWin": false, "pendingBids": 6 }));
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_bid_above_the_budget_share_would_not_win(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    // A winning bid may take 25% of €1,200, €300.
    let campaign = CampaignSpec {
        budget_cents: 120_000,
        ..CampaignSpec::default()
    };
    let campaign = insert_campaign(&pool, advertiser, campaign).await?;

    for (amount_cents, would_win) in [(30_000, true), (30_001, false)] {
        let (status, body) = chance(&pool, campaign, account, amount_cents).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["wouldWin"], would_win, "{amount_cents}");
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_chance_refuses_amounts_outside_the_min_and_max_bid(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let rules = test_config().rules;

    for amount_cents in [
        rules.min_bid_cents(VIEWS) - 1,
        rules.max_bid_cents(VIEWS) + 1,
    ] {
        let (status, error) = chance(&pool, campaign, account, amount_cents).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
        // With the error that placing the bid gets.
        let (_, bid_error) = place_bid(&pool, campaign, account, amount_cents).await;
        assert_eq!(error, bid_error);
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn an_account_can_bid_only_on_campaigns_in_its_feed(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let open = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let in_austria = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    sqlx::query("INSERT INTO campaign_countries (campaign_id, country_code) VALUES ($1, 'AT')")
        .bind(in_austria)
        .execute(&pool)
        .await?;
    let in_french = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    sqlx::query("INSERT INTO campaign_languages (campaign_id, language_code) VALUES ($1, 'fr')")
        .bind(in_french)
        .execute(&pool)
        .await?;
    let nano_only = CampaignSpec {
        size_groups: vec![SizeGroup::Nano],
        ..CampaignSpec::default()
    };
    let nano_only = insert_campaign(&pool, advertiser, nano_only).await?;
    // A bid may take 25% of €20,000, €5,000, less than a million-view account's min bid.
    let small_mega = CampaignSpec {
        budget_cents: 2_000_000,
        size_groups: vec![SizeGroup::Mega],
        ..CampaignSpec::default()
    };
    let small_mega = insert_campaign(&pool, advertiser, small_mega).await?;

    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let fitting = insert_account(&pool, creator, "lena.moves", VIEWS).await?;
    let instagram = AccountSpec {
        platform: Platform::Instagram,
        ..AccountSpec::default()
    };
    let on_instagram = insert_account_with(&pool, creator, "lena.insta", instagram).await?;
    let unsafe_spec = AccountSpec {
        brand_safe: false,
        ..AccountSpec::default()
    };
    let not_brand_safe = insert_account_with(&pool, creator, "lena.edgy", unsafe_spec).await?;
    let mega_views = 1_000_000;
    let mega = insert_account(&pool, creator, "lena.mega", mega_views).await?;
    let rules = test_config().rules;

    for (why, campaign, account, views) in [
        ("another platform", open, on_instagram, VIEWS),
        ("not brand safe", open, not_brand_safe, VIEWS),
        ("outside the countries", in_austria, fitting, VIEWS),
        ("without the languages", in_french, fitting, VIEWS),
        ("a size not picked", nano_only, fitting, VIEWS),
        ("too big for the budget", small_mega, mega, mega_views),
    ] {
        assert!(
            !feed_campaigns(&pool, account).await.contains(&campaign),
            "{why}"
        );
        // Even at its min bid.
        let (status, error) = place_bid(&pool, campaign, account, rules.min_bid_cents(views)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{why}: {error}");
        assert_eq!(
            error["message"],
            format!("Platform account {account} cannot bid on campaign {campaign}."),
            "{why}"
        );
    }

    assert!(feed_campaigns(&pool, fitting).await.contains(&open));
    let (status, bid) = place_bid(&pool, open, fitting, rules.min_bid_cents(VIEWS)).await;
    assert_eq!(status, StatusCode::OK, "{bid}");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_creator_has_one_pending_bid_per_campaign(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let creator = insert_creator(&pool, "Lena Vogt").await?;
    let dancing = insert_account(&pool, creator, "lena.moves", VIEWS).await?;
    let cooking = insert_account(&pool, creator, "lena.cooks", 12_000).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    assert_eq!(
        place_bid(&pool, campaign, dancing, 15_000).await.0,
        StatusCode::OK
    );

    let (status, error) = place_bid(&pool, campaign, cooking, 15_000).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
    assert_eq!(error.get("fields"), None);
    // A withdrawn bid does not count.
    assert_eq!(
        withdraw_bid(&pool, campaign, dancing).await.0,
        StatusCode::OK
    );
    assert_eq!(
        place_bid(&pool, campaign, cooking, 15_000).await.0,
        StatusCode::OK
    );
    assert_eq!(
        place_bid(&pool, campaign, dancing, 15_000).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let other_campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    assert_eq!(
        place_bid(&pool, other_campaign, dancing, 15_000).await.0,
        StatusCode::OK
    );
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bids_change_only_while_the_campaign_is_open(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let past_deadline = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    assert_eq!(
        place_bid(&pool, past_deadline, account, 15_000).await.0,
        StatusCode::OK
    );
    set_state(&pool, past_deadline, CampaignState::PastDeadline).await?;
    let closed = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    set_state(&pool, closed, CampaignState::Closed).await?;

    for (status, error) in [
        place_bid(&pool, past_deadline, account, 16_000).await,
        withdraw_bid(&pool, past_deadline, account).await,
        place_bid(&pool, closed, account, 15_000).await,
    ] {
        assert_eq!(status, StatusCode::CONFLICT, "{error}");
        assert_eq!(error["error"], "conflict");
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn a_withdrawn_bid_can_be_placed_again(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let (_, placed) = place_bid(&pool, campaign, account, 15_000).await;

    let (status, withdrawn) = withdraw_bid(&pool, campaign, account).await;

    assert_eq!(status, StatusCode::OK, "{withdrawn}");
    assert_eq!(withdrawn["id"], placed["id"]);
    assert_eq!(withdrawn["status"], "withdrawn");
    assert_eq!(
        withdraw_bid(&pool, campaign, account).await,
        (StatusCode::OK, withdrawn)
    );

    let (status, reinstated) = place_bid(&pool, campaign, account, 14_000).await;
    assert_eq!(status, StatusCode::OK, "{reinstated}");
    assert_eq!(reinstated["id"], placed["id"]);
    assert_eq!(reinstated["status"], "pending");
    assert_eq!(reinstated["amountCents"], 14_000);
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_campaigns_accounts_and_bids_are_not_found(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;

    for (status, error) in [
        place_bid(&pool, 999, account, 15_000).await,
        place_bid(&pool, campaign, 999, 15_000).await,
        withdraw_bid(&pool, 999, account).await,
        withdraw_bid(&pool, campaign, 999).await,
        // The account has no bid on it.
        withdraw_bid(&pool, campaign, account).await,
        get(&pool, "/api/platform-accounts/999/bids").await,
    ] {
        assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
        assert_eq!(error["error"], "not_found");
    }
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lists_bids_newest_first_and_filters_by_status(pool: PgPool) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let first = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let second = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    let third = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    for campaign in [first, second, third] {
        assert_eq!(
            place_bid(&pool, campaign, account, 15_000).await.0,
            StatusCode::OK
        );
    }
    assert_eq!(withdraw_bid(&pool, second, account).await.0, StatusCode::OK);
    // The order is by when bids were placed, not by id.
    sqlx::query(
        "UPDATE bids SET created_at = created_at - interval '1 day' WHERE campaign_id = $1",
    )
    .bind(third)
    .execute(&pool)
    .await?;

    assert_eq!(listed(&pool, account, "").await, [second, first, third]);
    assert_eq!(
        listed(&pool, account, "?status=pending").await,
        [first, third]
    );
    assert_eq!(listed(&pool, account, "?status=withdrawn").await, [second]);
    assert!(listed(&pool, account, "?status=won").await.is_empty());

    let uri = format!("/api/platform-accounts/{account}/bids?status=open");
    let (status, error) = get(&pool, &uri).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "bad_request");
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bid_list_items_show_the_payout_after_the_campaigns_commission(
    pool: PgPool,
) -> sqlx::Result<()> {
    let advertiser = insert_advertiser(&pool, "Glow Labs").await?;
    let account = new_account(&pool, "lena.moves", VIEWS).await?;
    let campaign = insert_campaign(&pool, advertiser, CampaignSpec::default()).await?;
    assert_eq!(
        place_bid(&pool, campaign, account, 15_000).await.0,
        StatusCode::OK
    );
    // The commission changes after publishing; the campaign's bids keep the frozen one.
    let mut changed = test_config();
    changed.rules.commission_bps = 4_000;

    let uri = format!("/api/platform-accounts/{account}/bids");
    let (status, body) = send(router_with(&pool, changed, false), Method::GET, &uri, None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let frozen = test_config().rules;
    let item = &body["items"][0];
    assert_eq!(item["amountCents"], 15_000);
    assert_eq!(item["payoutCents"], frozen.payout_cents(15_000));
    assert_eq!(item["minPaidViews"], frozen.min_paid_views(VIEWS, 15_000));
    let shown = &item["campaign"];
    assert_eq!(shown["id"], campaign);
    assert_eq!(shown["title"], "Summer serum launch");
    assert_eq!(shown["advertiserName"], "Glow Labs");
    assert_eq!(shown["platform"], "tiktok");
    assert_eq!(
        shown["briefing"],
        "Show your morning routine with the serum."
    );
    assert!(shown["biddingDeadline"].is_string());
    assert_eq!(
        shown["submissionWindowDays"],
        test_config().limits.posting_window_default_days
    );
    assert_eq!(shown["phase"], "open");
    Ok(())
}
